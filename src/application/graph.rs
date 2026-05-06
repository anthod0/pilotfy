use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphRuntimeConfig {
    pub enabled: bool,
    pub db_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProvenanceNode {
    pub id: String,
    pub kind: String,
    pub properties: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProvenanceEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub properties: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TaskProvenance {
    pub nodes: Vec<ProvenanceNode>,
    pub edges: Vec<ProvenanceEdge>,
}

#[derive(Clone)]
pub struct GraphProjectionService {
    pool: SqlitePool,
    config: GraphRuntimeConfig,
}

impl GraphProjectionService {
    pub fn new(pool: SqlitePool, config: GraphRuntimeConfig) -> Self {
        Self { pool, config }
    }

    pub async fn project_task(&self, task_id: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        let Some(db_dir) = self.config.db_dir.clone() else {
            return Err(Error::InvalidConfig {
                key: "LLMPARTY_GRAPH_DB_DIR",
                message: "graph projection is enabled but no database directory is configured"
                    .to_string(),
            });
        };

        let snapshot = self.load_task_snapshot(task_id).await?;
        project_snapshot_to_lbug(PathBuf::from(db_dir), snapshot)
    }

    pub async fn task_provenance(&self, task_id: &str) -> Result<TaskProvenance> {
        if !self.config.enabled {
            return Ok(TaskProvenance {
                nodes: vec![],
                edges: vec![],
            });
        }

        #[cfg(feature = "lbug")]
        if let Some(db_dir) = self.config.db_dir.clone() {
            return query_task_provenance(PathBuf::from(db_dir), task_id);
        }

        let snapshot = self.load_task_snapshot(task_id).await?;
        Ok(snapshot_to_provenance(snapshot))
    }

    async fn load_task_snapshot(&self, task_id: &str) -> Result<TaskGraphSnapshot> {
        let task_row = sqlx::query(
            r#"SELECT task_id, input, created_at, updated_at
               FROM tasks WHERE task_id = ?"#,
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| Error::NotFound(format!("task {task_id} not found")))?;

        let event_rows = sqlx::query(
            r#"SELECT event_type, payload, created_at
               FROM task_events WHERE task_id = ? ORDER BY created_at, event_id"#,
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        let mut decisions = Vec::new();
        for row in event_rows {
            let event_type: String = row.try_get("event_type")?;
            if !matches!(
                event_type.as_str(),
                "task.planning_completed"
                    | "task.planning_resolved"
                    | "task.planning_needs_input"
                    | "task.planning_failed"
            ) {
                continue;
            }
            let payload: String = row.try_get("payload")?;
            let payload: Value = serde_json::from_str(&payload)?;
            let Some(decision_value) = payload.get("decision").filter(|value| value.is_object())
            else {
                continue;
            };
            let decision_id = decision_value
                .get("decision_id")
                .and_then(Value::as_str)
                .unwrap_or("dec_unknown")
                .to_string();
            if decisions
                .iter()
                .any(|decision: &GraphDecision| decision.decision_id == decision_id)
            {
                continue;
            }
            let workspace_confidence = decision_value
                .get("workspace")
                .and_then(|workspace| workspace.get("confidence"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            let mut evidence = Vec::new();
            if let Some(items) = decision_value.get("evidence").and_then(Value::as_array) {
                for (index, item) in items.iter().enumerate() {
                    evidence.push(GraphEvidence {
                        evidence_id: item
                            .get("evidence_id")
                            .and_then(Value::as_str)
                            .map(ToString::to_string)
                            .unwrap_or_else(|| format!("{decision_id}_ev_{index}")),
                        kind: item
                            .get("kind")
                            .and_then(Value::as_str)
                            .unwrap_or("other")
                            .to_string(),
                        reference: item
                            .get("ref")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        summary: item
                            .get("summary")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
            decisions.push(GraphDecision {
                decision_id,
                status: decision_value
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                reason: decision_value
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                confidence: workspace_confidence,
                created_at: row.try_get("created_at")?,
                evidence,
            });
        }

        Ok(TaskGraphSnapshot {
            task_id: task_row.try_get("task_id")?,
            task_input: task_row.try_get("input")?,
            task_created_at: task_row.try_get("created_at")?,
            task_updated_at: task_row.try_get("updated_at")?,
            decisions,
        })
    }
}

#[derive(Debug)]
struct TaskGraphSnapshot {
    task_id: String,
    task_input: String,
    task_created_at: String,
    task_updated_at: String,
    decisions: Vec<GraphDecision>,
}

#[derive(Debug)]
struct GraphDecision {
    decision_id: String,
    status: String,
    reason: String,
    confidence: f64,
    created_at: String,
    evidence: Vec<GraphEvidence>,
}

#[derive(Debug)]
struct GraphEvidence {
    evidence_id: String,
    kind: String,
    reference: String,
    summary: String,
}

#[cfg(not(feature = "lbug"))]
fn project_snapshot_to_lbug(_db_dir: PathBuf, _snapshot: TaskGraphSnapshot) -> Result<()> {
    Ok(())
}

#[cfg(feature = "lbug")]
fn project_snapshot_to_lbug(db_dir: PathBuf, snapshot: TaskGraphSnapshot) -> Result<()> {
    let conn = open_graph_connection(db_dir)?;
    initialize_schema(&conn)?;

    query(
        &conn,
        &format!(
            "MERGE (t:Task {{task_id: {}}}) SET t.title = {}, t.description = {}, t.ref = {}, t.created_at = {}, t.updated_at = {};",
            cypher_string(&snapshot.task_id),
            cypher_string(&task_title(&snapshot.task_input)),
            cypher_string(&snapshot.task_input),
            cypher_string(&format!("sqlite:task:{}", snapshot.task_id)),
            cypher_string(&snapshot.task_created_at),
            cypher_string(&snapshot.task_updated_at)
        ),
    )?;

    if !snapshot.decisions.is_empty() {
        query(
            &conn,
            "MERGE (a:Agent {agent_id: 'agent_planner'}) SET a.name = 'Task Planner', a.role = 'planner', a.capabilities = '[\"workspace_routing\",\"task_planning\"]', a.availability = 'available', a.ref = 'internal:planner', a.created_at = '', a.updated_at = '';",
        )?;
    }

    for decision in snapshot.decisions {
        let work_item_id = format!("wi_{}", decision.decision_id);
        let signal_id = format!("sig_{}", decision.decision_id);
        query(
            &conn,
            &format!(
                "MERGE (w:WorkItem {{work_item_id: {}}}) SET w.title = 'Plan task', w.description = {}, w.kind = 'planning', w.planning_state = 'active', w.execution_state = {}, w.execution_ref = '', w.created_at = {}, w.updated_at = {};",
                cypher_string(&work_item_id),
                cypher_string(&decision.reason),
                cypher_string(decision_execution_state(&decision.status)),
                cypher_string(&decision.created_at),
                cypher_string(&decision.created_at)
            ),
        )?;
        query(
            &conn,
            &format!(
                "MATCH (t:Task {{task_id: {}}}), (w:WorkItem {{work_item_id: {}}}) MERGE (t)-[:HAS_WORK]->(w);",
                cypher_string(&snapshot.task_id),
                cypher_string(&work_item_id)
            ),
        )?;
        query(
            &conn,
            &format!(
                "MATCH (w:WorkItem {{work_item_id: {}}}), (a:Agent {{agent_id: 'agent_planner'}}) MERGE (w)-[:ASSIGNED_TO]->(a);",
                cypher_string(&work_item_id)
            ),
        )?;
        query(
            &conn,
            &format!(
                "MERGE (s:Signal {{signal_id: {}}}) SET s.source_type = 'agent', s.kind = {}, s.summary = {}, s.detail = {}, s.origin_ref = {}, s.created_at = {};",
                cypher_string(&signal_id),
                cypher_string(decision_signal_kind(&decision.status)),
                cypher_string(&decision.reason),
                cypher_string(&format!(
                    "planner status: {}; confidence: {}",
                    decision.status, decision.confidence
                )),
                cypher_string(&format!("sqlite:task:{}", snapshot.task_id)),
                cypher_string(&decision.created_at)
            ),
        )?;
        query(
            &conn,
            &format!(
                "MATCH (t:Task {{task_id: {}}}), (s:Signal {{signal_id: {}}}) MERGE (t)-[:HAS_SIGNAL]->(s);",
                cypher_string(&snapshot.task_id),
                cypher_string(&signal_id)
            ),
        )?;
        query(
            &conn,
            &format!(
                "MATCH (a:Agent {{agent_id: 'agent_planner'}}), (s:Signal {{signal_id: {}}}) MERGE (a)-[:EMITS]->(s);",
                cypher_string(&signal_id)
            ),
        )?;

        for evidence in decision.evidence {
            let artifact_id = format!("art_{}", evidence.evidence_id);
            query(
                &conn,
                &format!(
                    "MERGE (a:Artifact {{artifact_id: {}}}) SET a.kind = {}, a.name = {}, a.summary = {}, a.availability = 'available', a.ref = {}, a.created_at = '', a.updated_at = '';",
                    cypher_string(&artifact_id),
                    cypher_string(&evidence.kind),
                    cypher_string(&evidence.evidence_id),
                    cypher_string(&evidence.summary),
                    cypher_string(&evidence.reference)
                ),
            )?;
            query(
                &conn,
                &format!(
                    "MATCH (w:WorkItem {{work_item_id: {}}}), (a:Artifact {{artifact_id: {}}}) MERGE (w)-[:REQUIRES]->(a);",
                    cypher_string(&work_item_id),
                    cypher_string(&artifact_id)
                ),
            )?;
            query(
                &conn,
                &format!(
                    "MATCH (s:Signal {{signal_id: {}}}), (a:Artifact {{artifact_id: {}}}) MERGE (s)-[:SUPPORTED_BY]->(a);",
                    cypher_string(&signal_id),
                    cypher_string(&artifact_id)
                ),
            )?;
        }
    }

    Ok(())
}

fn snapshot_to_provenance(snapshot: TaskGraphSnapshot) -> TaskProvenance {
    let mut nodes = vec![ProvenanceNode {
        id: snapshot.task_id.clone(),
        kind: "Task".to_string(),
        properties: json!({
            "title": task_title(&snapshot.task_input),
            "description": snapshot.task_input,
            "ref": format!("sqlite:task:{}", snapshot.task_id),
            "created_at": snapshot.task_created_at,
            "updated_at": snapshot.task_updated_at
        }),
    }];
    let mut edges = Vec::new();

    if !snapshot.decisions.is_empty() {
        nodes.push(ProvenanceNode {
            id: "agent_planner".to_string(),
            kind: "Agent".to_string(),
            properties: json!({
                "name": "Task Planner",
                "role": "planner",
                "capabilities": "[\"workspace_routing\",\"task_planning\"]",
                "availability": "available",
                "ref": "internal:planner",
                "created_at": "",
                "updated_at": ""
            }),
        });
    }

    for decision in snapshot.decisions {
        let work_item_id = format!("wi_{}", decision.decision_id);
        let signal_id = format!("sig_{}", decision.decision_id);
        nodes.push(ProvenanceNode {
            id: work_item_id.clone(),
            kind: "WorkItem".to_string(),
            properties: json!({
                "title": "Plan task",
                "description": decision.reason,
                "kind": "planning",
                "planning_state": "active",
                "execution_state": decision_execution_state(&decision.status),
                "execution_ref": "",
                "created_at": decision.created_at,
                "updated_at": decision.created_at
            }),
        });
        edges.push(ProvenanceEdge {
            from: snapshot.task_id.clone(),
            to: work_item_id.clone(),
            kind: "HAS_WORK".to_string(),
            properties: json!({}),
        });
        edges.push(ProvenanceEdge {
            from: work_item_id.clone(),
            to: "agent_planner".to_string(),
            kind: "ASSIGNED_TO".to_string(),
            properties: json!({}),
        });

        nodes.push(ProvenanceNode {
            id: signal_id.clone(),
            kind: "Signal".to_string(),
            properties: json!({
                "source_type": "agent",
                "kind": decision_signal_kind(&decision.status),
                "summary": decision.reason,
                "detail": format!("planner status: {}; confidence: {}", decision.status, decision.confidence),
                "origin_ref": format!("sqlite:task:{}", snapshot.task_id),
                "created_at": decision.created_at
            }),
        });
        edges.push(ProvenanceEdge {
            from: snapshot.task_id.clone(),
            to: signal_id.clone(),
            kind: "HAS_SIGNAL".to_string(),
            properties: json!({}),
        });
        edges.push(ProvenanceEdge {
            from: "agent_planner".to_string(),
            to: signal_id.clone(),
            kind: "EMITS".to_string(),
            properties: json!({}),
        });

        for evidence in decision.evidence {
            let artifact_id = format!("art_{}", evidence.evidence_id);
            nodes.push(ProvenanceNode {
                id: artifact_id.clone(),
                kind: "Artifact".to_string(),
                properties: json!({
                    "kind": evidence.kind,
                    "name": evidence.evidence_id,
                    "summary": evidence.summary,
                    "availability": "available",
                    "ref": evidence.reference,
                    "created_at": "",
                    "updated_at": ""
                }),
            });
            edges.push(ProvenanceEdge {
                from: work_item_id.clone(),
                to: artifact_id.clone(),
                kind: "REQUIRES".to_string(),
                properties: json!({}),
            });
            edges.push(ProvenanceEdge {
                from: signal_id.clone(),
                to: artifact_id,
                kind: "SUPPORTED_BY".to_string(),
                properties: json!({}),
            });
        }
    }

    TaskProvenance { nodes, edges }
}

fn task_title(input: &str) -> String {
    input
        .lines()
        .next()
        .unwrap_or(input)
        .chars()
        .take(80)
        .collect()
}

fn decision_execution_state(status: &str) -> &'static str {
    match status {
        "failed" => "failed",
        _ => "completed",
    }
}

fn decision_signal_kind(status: &str) -> &'static str {
    match status {
        "needs_input" => "constraint",
        "failed" => "failure",
        _ => "finding",
    }
}

#[cfg(feature = "lbug")]
fn query_task_provenance(db_dir: PathBuf, task_id: &str) -> Result<TaskProvenance> {
    let conn = open_graph_connection(db_dir)?;
    initialize_schema(&conn)?;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let mut task_rows = query(
        &conn,
        &format!(
            "MATCH (t:Task {{task_id: {}}}) RETURN t.task_id, t.title, t.description, t.ref, t.created_at, t.updated_at;",
            cypher_string(task_id)
        ),
    )?;
    for row in task_rows.by_ref() {
        push_unique_node(
            &mut nodes,
            ProvenanceNode {
                id: string_value(&row[0]),
                kind: "Task".to_string(),
                properties: json!({
                    "title": string_value(&row[1]),
                    "description": string_value(&row[2]),
                    "ref": string_value(&row[3]),
                    "created_at": string_value(&row[4]),
                    "updated_at": string_value(&row[5])
                }),
            },
        );
    }

    let mut work_rows = query(
        &conn,
        &format!(
            "MATCH (t:Task {{task_id: {}}})-[r:HAS_WORK]->(w:WorkItem) RETURN t.task_id, w.work_item_id, w.title, w.description, w.kind, w.planning_state, w.execution_state, w.execution_ref, w.created_at, w.updated_at;",
            cypher_string(task_id)
        ),
    )?;
    for row in work_rows.by_ref() {
        edges.push(ProvenanceEdge {
            from: string_value(&row[0]),
            to: string_value(&row[1]),
            kind: "HAS_WORK".to_string(),
            properties: json!({}),
        });
        push_unique_node(
            &mut nodes,
            ProvenanceNode {
                id: string_value(&row[1]),
                kind: "WorkItem".to_string(),
                properties: json!({
                    "title": string_value(&row[2]),
                    "description": string_value(&row[3]),
                    "kind": string_value(&row[4]),
                    "planning_state": string_value(&row[5]),
                    "execution_state": string_value(&row[6]),
                    "execution_ref": string_value(&row[7]),
                    "created_at": string_value(&row[8]),
                    "updated_at": string_value(&row[9])
                }),
            },
        );
    }

    let mut signal_rows = query(
        &conn,
        &format!(
            "MATCH (t:Task {{task_id: {}}})-[r:HAS_SIGNAL]->(s:Signal) RETURN t.task_id, s.signal_id, s.source_type, s.kind, s.summary, s.detail, s.origin_ref, s.created_at;",
            cypher_string(task_id)
        ),
    )?;
    for row in signal_rows.by_ref() {
        edges.push(ProvenanceEdge {
            from: string_value(&row[0]),
            to: string_value(&row[1]),
            kind: "HAS_SIGNAL".to_string(),
            properties: json!({}),
        });
        push_unique_node(
            &mut nodes,
            ProvenanceNode {
                id: string_value(&row[1]),
                kind: "Signal".to_string(),
                properties: json!({
                    "source_type": string_value(&row[2]),
                    "kind": string_value(&row[3]),
                    "summary": string_value(&row[4]),
                    "detail": string_value(&row[5]),
                    "origin_ref": string_value(&row[6]),
                    "created_at": string_value(&row[7])
                }),
            },
        );
    }

    let mut assignment_rows = query(
        &conn,
        &format!(
            "MATCH (t:Task {{task_id: {}}})-[:HAS_WORK]->(w:WorkItem)-[r:ASSIGNED_TO]->(a:Agent) RETURN w.work_item_id, a.agent_id, a.name, a.role, a.capabilities, a.availability, a.ref, a.created_at, a.updated_at;",
            cypher_string(task_id)
        ),
    )?;
    for row in assignment_rows.by_ref() {
        edges.push(ProvenanceEdge {
            from: string_value(&row[0]),
            to: string_value(&row[1]),
            kind: "ASSIGNED_TO".to_string(),
            properties: json!({}),
        });
        push_unique_node(
            &mut nodes,
            ProvenanceNode {
                id: string_value(&row[1]),
                kind: "Agent".to_string(),
                properties: json!({
                    "name": string_value(&row[2]),
                    "role": string_value(&row[3]),
                    "capabilities": string_value(&row[4]),
                    "availability": string_value(&row[5]),
                    "ref": string_value(&row[6]),
                    "created_at": string_value(&row[7]),
                    "updated_at": string_value(&row[8])
                }),
            },
        );
    }

    let mut require_rows = query(
        &conn,
        &format!(
            "MATCH (t:Task {{task_id: {}}})-[:HAS_WORK]->(w:WorkItem)-[r:REQUIRES]->(a:Artifact) RETURN w.work_item_id, a.artifact_id, a.kind, a.name, a.summary, a.availability, a.ref, a.created_at, a.updated_at;",
            cypher_string(task_id)
        ),
    )?;
    for row in require_rows.by_ref() {
        edges.push(ProvenanceEdge {
            from: string_value(&row[0]),
            to: string_value(&row[1]),
            kind: "REQUIRES".to_string(),
            properties: json!({}),
        });
        push_unique_node(&mut nodes, artifact_node_from_row(&row, 1));
    }

    let mut emit_rows = query(
        &conn,
        &format!(
            "MATCH (t:Task {{task_id: {}}})-[:HAS_SIGNAL]->(s:Signal)<-[r:EMITS]-(a:Agent) RETURN a.agent_id, s.signal_id, a.name, a.role, a.capabilities, a.availability, a.ref, a.created_at, a.updated_at;",
            cypher_string(task_id)
        ),
    )?;
    for row in emit_rows.by_ref() {
        edges.push(ProvenanceEdge {
            from: string_value(&row[0]),
            to: string_value(&row[1]),
            kind: "EMITS".to_string(),
            properties: json!({}),
        });
        push_unique_node(
            &mut nodes,
            ProvenanceNode {
                id: string_value(&row[0]),
                kind: "Agent".to_string(),
                properties: json!({
                    "name": string_value(&row[2]),
                    "role": string_value(&row[3]),
                    "capabilities": string_value(&row[4]),
                    "availability": string_value(&row[5]),
                    "ref": string_value(&row[6]),
                    "created_at": string_value(&row[7]),
                    "updated_at": string_value(&row[8])
                }),
            },
        );
    }

    let mut support_rows = query(
        &conn,
        &format!(
            "MATCH (t:Task {{task_id: {}}})-[:HAS_SIGNAL]->(s:Signal)-[r:SUPPORTED_BY]->(a:Artifact) RETURN s.signal_id, a.artifact_id, a.kind, a.name, a.summary, a.availability, a.ref, a.created_at, a.updated_at;",
            cypher_string(task_id)
        ),
    )?;
    for row in support_rows.by_ref() {
        edges.push(ProvenanceEdge {
            from: string_value(&row[0]),
            to: string_value(&row[1]),
            kind: "SUPPORTED_BY".to_string(),
            properties: json!({}),
        });
        push_unique_node(&mut nodes, artifact_node_from_row(&row, 1));
    }

    Ok(TaskProvenance { nodes, edges })
}

#[cfg(feature = "lbug")]
fn open_graph_connection(db_dir: PathBuf) -> Result<lbug::Connection<'static>> {
    if let Some(parent) = db_dir
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let config = lbug::SystemConfig::default().enable_multi_writes(true);
    let db = lbug::Database::new(db_dir, config)
        .map_err(|error| Error::Domain(format!("lbug database open failed: {error}")))?;
    let db = Box::leak(Box::new(db));
    lbug::Connection::new(db)
        .map_err(|error| Error::Domain(format!("lbug connection failed: {error}")))
}

#[cfg(feature = "lbug")]
fn initialize_schema<'db>(conn: &lbug::Connection<'db>) -> Result<()> {
    for statement in [
        "CREATE NODE TABLE IF NOT EXISTS Task(task_id STRING, title STRING, description STRING, ref STRING, created_at STRING, updated_at STRING, PRIMARY KEY(task_id));",
        "CREATE NODE TABLE IF NOT EXISTS WorkItem(work_item_id STRING, title STRING, description STRING, kind STRING, planning_state STRING, execution_state STRING, execution_ref STRING, created_at STRING, updated_at STRING, PRIMARY KEY(work_item_id));",
        "CREATE NODE TABLE IF NOT EXISTS Agent(agent_id STRING, name STRING, role STRING, capabilities STRING, availability STRING, ref STRING, created_at STRING, updated_at STRING, PRIMARY KEY(agent_id));",
        "CREATE NODE TABLE IF NOT EXISTS Artifact(artifact_id STRING, kind STRING, name STRING, summary STRING, availability STRING, ref STRING, created_at STRING, updated_at STRING, PRIMARY KEY(artifact_id));",
        "CREATE NODE TABLE IF NOT EXISTS Signal(signal_id STRING, source_type STRING, kind STRING, summary STRING, detail STRING, origin_ref STRING, created_at STRING, PRIMARY KEY(signal_id));",
        "CREATE REL TABLE IF NOT EXISTS HAS_WORK(FROM Task TO WorkItem);",
        "CREATE REL TABLE IF NOT EXISTS HAS_SIGNAL(FROM Task TO Signal);",
        "CREATE REL TABLE IF NOT EXISTS DEPENDS_ON(FROM WorkItem TO WorkItem);",
        "CREATE REL TABLE IF NOT EXISTS SUPERSEDES(FROM WorkItem TO WorkItem);",
        "CREATE REL TABLE IF NOT EXISTS CAUSED_BY(FROM WorkItem TO Signal);",
        "CREATE REL TABLE IF NOT EXISTS ASSIGNED_TO(FROM WorkItem TO Agent);",
        "CREATE REL TABLE IF NOT EXISTS REQUIRES(FROM WorkItem TO Artifact);",
        "CREATE REL TABLE IF NOT EXISTS PRODUCES(FROM WorkItem TO Artifact);",
        "CREATE REL TABLE IF NOT EXISTS EMITS(FROM Agent TO Signal);",
        "CREATE REL TABLE IF NOT EXISTS SUPPORTED_BY(FROM Signal TO Artifact);",
        "CREATE REL TABLE IF NOT EXISTS DERIVED_FROM(FROM Artifact TO Artifact);",
    ] {
        query(conn, statement)?;
    }
    Ok(())
}

#[cfg(feature = "lbug")]
fn query<'db>(conn: &lbug::Connection<'db>, statement: &str) -> Result<lbug::QueryResult<'db>> {
    conn.query(statement)
        .map_err(|error| Error::Domain(format!("lbug query failed: {error}; query: {statement}")))
}

#[cfg(feature = "lbug")]
fn cypher_string(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

#[cfg(feature = "lbug")]
fn string_value(value: &lbug::Value) -> String {
    match value {
        lbug::Value::String(value) => value.clone(),
        lbug::Value::Null(_) => String::new(),
        other => other.to_string(),
    }
}

#[cfg(feature = "lbug")]
fn artifact_node_from_row(row: &[lbug::Value], offset: usize) -> ProvenanceNode {
    ProvenanceNode {
        id: string_value(&row[offset]),
        kind: "Artifact".to_string(),
        properties: json!({
            "kind": string_value(&row[offset + 1]),
            "name": string_value(&row[offset + 2]),
            "summary": string_value(&row[offset + 3]),
            "availability": string_value(&row[offset + 4]),
            "ref": string_value(&row[offset + 5]),
            "created_at": string_value(&row[offset + 6]),
            "updated_at": string_value(&row[offset + 7])
        }),
    }
}

#[cfg(feature = "lbug")]
fn push_unique_node(nodes: &mut Vec<ProvenanceNode>, node: ProvenanceNode) {
    if !nodes
        .iter()
        .any(|existing| existing.id == node.id && existing.kind == node.kind)
    {
        nodes.push(node);
    }
}
