use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentToolRequest {
    pub session_id: String,
    pub turn_id: String,
    pub runtime_instance_id: String,
    #[serde(default = "default_agent_tool_input")]
    pub input: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub enum AgentToolResponse {
    Skeleton { context: AgentToolContext },
    GetContext(GetContextToolResponse),
    SubmitPlan(SubmitPlanToolResponse),
    SubmitResult(SubmitResultToolResponse),
    RaiseSignal(RaiseSignalToolResponse),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GetContextToolResponse {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SubmitPlanToolResponse {
    pub proposal_id: String,
    pub validation: Value,
    pub apply: Value,
    pub scheduler: DagSchedulerOutcome,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SubmitResultToolResponse {
    pub task_id: String,
    pub work_item_id: String,
    pub run_id: String,
    pub state: String,
    pub scheduler: DagSchedulerOutcome,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RaiseSignalToolResponse {
    pub signal_id: String,
    pub task_id: String,
    pub work_item_id: Option<String>,
    pub run_id: Option<String>,
    pub kind: String,
    pub state: String,
    pub policy: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AgentToolContext {
    pub session_id: String,
    pub turn_id: String,
    pub client_type: String,
    pub runtime_instance_id: String,
    pub task_id: String,
    pub mode: AgentToolMode,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentToolMode {
    Planning {
        role: AgentPlanningRole,
    },
    Execution {
        run_id: String,
        work_item_id: String,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentPlanningRole {
    Planner,
    Replanner,
}

#[derive(Clone)]
pub struct AgentToolService {
    pool: SqlitePool,
    resolver: AgentToolContextResolver,
    queries: ExternalQueryService,
    profiles: AgentProfileService,
}

impl AgentToolService {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool: pool.clone(),
            resolver: AgentToolContextResolver::new(pool.clone()),
            queries: ExternalQueryService::new(pool.clone()),
            profiles: AgentProfileService::new(pool),
        }
    }

    pub async fn call(
        &self,
        tool_name: &str,
        request: AgentToolRequest,
    ) -> Result<AgentToolResponse> {
        if !is_known_tool(tool_name) {
            return Err(Error::NotFound(format!("agent tool {tool_name} not found")));
        }
        let context = self.resolver.resolve(&request).await?;
        match tool_name {
            "getContext" => self.get_context(context).await,
            "submitPlan" => self.submit_plan(context, request.input).await,
            "submitResult" => self.submit_result(context, request.input).await,
            "raiseSignal" => self.raise_signal(context, request.input).await,
            _ => Ok(AgentToolResponse::Skeleton { context }),
        }
    }

    async fn submit_result(
        &self,
        context: AgentToolContext,
        input: Value,
    ) -> Result<AgentToolResponse> {
        if !matches!(&context.mode, AgentToolMode::Execution { .. }) {
            return Err(Error::StateConflict(
                "submitResult requires a DAG execution turn".to_string(),
            ));
        }
        let payload: SubmitResultPayload = serde_json::from_value(input)
            .map_err(|err| Error::Domain(format!("invalid submitResult input: {err}")))?;
        let outcome = DagRunResultService::new(self.pool.clone())
            .submit_tool_result(&context, payload)
            .await?;
        Ok(AgentToolResponse::SubmitResult(SubmitResultToolResponse {
            task_id: outcome.task_id,
            work_item_id: outcome.work_item_id,
            run_id: outcome.run_id,
            state: outcome.state,
            scheduler: outcome.scheduler,
        }))
    }

    async fn raise_signal(
        &self,
        context: AgentToolContext,
        input: Value,
    ) -> Result<AgentToolResponse> {
        let payload: RaiseSignalPayload = serde_json::from_value(input)
            .map_err(|err| Error::Domain(format!("invalid raiseSignal input: {err}")))?;
        let outcome = DagRunResultService::new(self.pool.clone())
            .raise_tool_signal(&context, payload)
            .await?;
        Ok(AgentToolResponse::RaiseSignal(RaiseSignalToolResponse {
            signal_id: outcome.signal_id,
            task_id: outcome.task_id,
            work_item_id: outcome.work_item_id,
            run_id: outcome.run_id,
            kind: outcome.kind,
            state: outcome.state,
            policy: json!({ "replanner_started": outcome.replanner_started }),
        }))
    }

    async fn submit_plan(
        &self,
        context: AgentToolContext,
        input: Value,
    ) -> Result<AgentToolResponse> {
        let AgentToolMode::Planning { role } = &context.mode else {
            return Err(Error::StateConflict(
                "submitPlan requires a DAG planning turn".to_string(),
            ));
        };

        reject_duplicate_successful_submit_plan(&self.pool, &context).await?;

        let mode = input
            .get("mode")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Domain("submitPlan input missing mode".to_string()))?;
        let planning = DagPlanningService::new(self.pool.clone());
        let outcome = match (role, mode) {
            (AgentPlanningRole::Planner, "initial_dag") => {
                let payload = parse_submit_plan_initial_input(input)?;
                planning
                    .submit_initial_plan_payload(&context.task_id, &context.session_id, payload)
                    .await?
            }
            (AgentPlanningRole::Planner, "patch") => {
                return Err(Error::StateConflict(
                    "Planner can only submit initial_dag plans".to_string(),
                ));
            }
            (AgentPlanningRole::Replanner, "patch") => {
                let (summary, patch) = parse_submit_plan_patch_input(input)?;
                planning
                    .submit_patch_payload(&context.task_id, &context.session_id, summary, patch)
                    .await?
            }
            (AgentPlanningRole::Replanner, "initial_dag") => {
                return Err(Error::StateConflict(
                    "RePlanner can only submit patch plans".to_string(),
                ));
            }
            (_, other) => {
                return Err(Error::Domain(format!(
                    "submitPlan mode must be initial_dag or patch, got {other}"
                )));
            }
        };

        Ok(AgentToolResponse::SubmitPlan(SubmitPlanToolResponse {
            proposal_id: outcome.proposal.proposal_id.clone(),
            validation: json!({"ok": true}),
            apply: json!({
                "applied": true,
                "proposal_state": outcome.proposal.state,
                "mode": outcome.proposal.mode,
            }),
            scheduler: outcome.scheduler,
        }))
    }

    async fn get_context(&self, context: AgentToolContext) -> Result<AgentToolResponse> {
        let task = self
            .queries
            .get_task(&context.task_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("task {} not found", context.task_id)))?;

        match context.mode.clone() {
            AgentToolMode::Planning { role } => {
                let dag = self.queries.get_task_dag(&context.task_id).await?;
                let open_signals: Vec<_> = dag
                    .signals
                    .iter()
                    .filter(|signal| signal.state == "open")
                    .cloned()
                    .collect();
                let relevant_proposals = self
                    .queries
                    .list_relevant_dag_proposals(&context.task_id)
                    .await?;
                let execution_profiles = self.profiles.list_latest().await?;

                Ok(AgentToolResponse::GetContext(GetContextToolResponse {
                    text: render_planning_context(
                        role,
                        &task,
                        &dag,
                        &open_signals,
                        &relevant_proposals,
                        &execution_profiles,
                    ),
                }))
            }
            AgentToolMode::Execution {
                run_id,
                work_item_id,
            } => {
                let work_items = self.queries.list_work_items(&context.task_id).await?;
                let work_item = work_items
                    .iter()
                    .find(|item| item.work_item.work_item_id == *work_item_id)
                    .cloned()
                    .ok_or_else(|| {
                        Error::NotFound(format!("work item {work_item_id} not found"))
                    })?;
                let work_item_run = self
                    .queries
                    .list_work_item_runs(&context.task_id)
                    .await?
                    .into_iter()
                    .find(|run| run.run_id == *run_id)
                    .ok_or_else(|| Error::NotFound(format!("work item run {run_id} not found")))?;
                let edges = self.queries.list_work_item_edges(&context.task_id).await?;
                let dependencies: Vec<_> = edges
                    .into_iter()
                    .filter(|edge| edge.to_work_item_id == *work_item_id)
                    .collect();
                let upstream_completed_items: Vec<_> = dependencies
                    .iter()
                    .filter_map(|edge| {
                        work_items.iter().find(|item| {
                            item.work_item.work_item_id == edge.from_work_item_id
                                && item
                                    .runtime
                                    .as_ref()
                                    .map(|runtime| runtime.current_state.as_str())
                                    == Some("completed")
                        })
                    })
                    .cloned()
                    .collect();
                let open_signals: Vec<_> = self
                    .queries
                    .list_dag_signals(&context.task_id)
                    .await?
                    .into_iter()
                    .filter(|signal| {
                        signal.state == "open"
                            && (signal.work_item_id.as_deref().is_none()
                                || signal.work_item_id.as_deref() == Some(work_item_id.as_str())
                                || signal.run_id.as_deref() == Some(run_id.as_str()))
                    })
                    .collect();
                let acceptance_criteria = work_item.work_item.acceptance_criteria.clone();

                Ok(AgentToolResponse::GetContext(GetContextToolResponse {
                    text: render_execution_context(
                        &task,
                        &work_item,
                        &work_item_run,
                        &upstream_completed_items,
                        &acceptance_criteria,
                        &open_signals,
                    ),
                }))
            }
        }
    }
}

fn render_planning_context(
    role: AgentPlanningRole,
    task: &TaskView,
    dag: &TaskDagView,
    open_signals: &[DagSignalRecord],
    relevant_proposals: &[DagProposal],
    execution_profiles: &[ExecutionProfileView],
) -> String {
    let mut lines = vec![
        "llmparty context: planning".to_string(),
        format!("Role: {}", planning_role_text(&role)),
        String::new(),
        "Task:".to_string(),
        format!("- Goal: {}", task.input),
        format!("- State: {}", task.state),
    ];
    if let Some(workspace_id) = non_empty(task.workspace_id.as_deref()) {
        lines.push(format!("- Workspace: {workspace_id}"));
    }

    lines.push(String::new());
    if dag.work_items.is_empty() {
        lines.push("Current DAG: none yet.".to_string());
    } else {
        lines.push("Current DAG:".to_string());
        lines.push(format!(
            "- Summary: total {}, ready {}, running {}, completed {}, blocked {}, failed {}, open signals {}",
            dag.summary.total_work_items,
            dag.summary.ready_work_items,
            dag.summary.running_work_items,
            dag.summary.completed_work_items,
            dag.summary.blocked_work_items,
            dag.summary.failed_work_items,
            dag.summary.open_signals
        ));
        lines.push("- Work items:".to_string());
        for item in &dag.work_items {
            let state = item
                .runtime
                .as_ref()
                .map(|runtime| runtime.current_state.as_str())
                .unwrap_or("unknown");
            lines.push(format!(
                "  - {} [{}] {}",
                item.work_item.work_item_id, state, item.work_item.title
            ));
            push_optional(
                &mut lines,
                "    Description",
                non_empty(Some(&item.work_item.description)),
            );
            push_optional(
                &mut lines,
                "    Action",
                non_empty(Some(&item.work_item.action)),
            );
            lines.push(format!(
                "    Profile: {}",
                item.work_item.execution_profile_id
            ));
            let depends_on: Vec<_> = dag
                .edges
                .iter()
                .filter(|edge| edge.to_work_item_id == item.work_item.work_item_id)
                .map(|edge| edge.from_work_item_id.as_str())
                .collect();
            if !depends_on.is_empty() {
                lines.push(format!("    Depends on: {}", depends_on.join(", ")));
            }
            push_value_list(
                &mut lines,
                "    Acceptance",
                &item.work_item.acceptance_criteria,
                None,
            );
        }
    }

    lines.push(String::new());
    push_signals(&mut lines, "Open signals", open_signals);

    lines.push(String::new());
    if relevant_proposals.is_empty() {
        lines.push("Relevant proposals: none.".to_string());
    } else {
        lines.push("Relevant proposals:".to_string());
        for proposal in relevant_proposals {
            lines.push(format!(
                "- {} [{} / {}]: {}",
                proposal.proposal_id, proposal.state, proposal.mode, proposal.summary
            ));
        }
    }

    lines.push(String::new());
    if execution_profiles.is_empty() {
        lines.push("Available execution profiles: none.".to_string());
    } else {
        lines.push("Available execution profiles:".to_string());
        for profile in execution_profiles {
            let mut line = format!("- {}: {}", profile.profile_id, profile.name);
            if let Some(description) = non_empty(profile.description.as_deref()) {
                line.push_str(&format!(" — {description}"));
            }
            lines.push(line);
            if !profile.supported_client_types.is_empty() {
                lines.push(format!(
                    "  Clients: {}",
                    profile.supported_client_types.join(", ")
                ));
            }
            push_optional(
                &mut lines,
                "  Expected output",
                non_empty(profile.expected_output_schema.as_deref()),
            );
        }
    }

    lines.push(String::new());
    lines.push("Next:".to_string());
    match role {
        AgentPlanningRole::Planner => {
            lines.push("- Submit an initial DAG with submitPlan.".to_string())
        }
        AgentPlanningRole::Replanner => {
            lines.push("- Submit a DAG patch with submitPlan.".to_string())
        }
    }
    lines.push(
        "- Do not include task_id, work_item_id, run_id, session_id, or turn_id.".to_string(),
    );

    lines.join("\n")
}

fn render_execution_context(
    task: &TaskView,
    work_item: &WorkItemWithRuntimeView,
    work_item_run: &WorkItemRunRecord,
    upstream_completed_items: &[WorkItemWithRuntimeView],
    acceptance_criteria: &Value,
    open_signals: &[DagSignalRecord],
) -> String {
    let mut lines = vec![
        "llmparty context: execution".to_string(),
        String::new(),
        "Task:".to_string(),
        format!("- Goal: {}", task.input),
        format!("- State: {}", task.state),
    ];
    if let Some(workspace_id) = non_empty(task.workspace_id.as_deref()) {
        lines.push(format!("- Workspace: {workspace_id}"));
    }

    lines.push(String::new());
    lines.push("Current WorkItem:".to_string());
    lines.push(format!("- ID: {}", work_item.work_item.work_item_id));
    lines.push(format!("- Title: {}", work_item.work_item.title));
    push_optional(
        &mut lines,
        "- Description",
        non_empty(Some(&work_item.work_item.description)),
    );
    push_optional(
        &mut lines,
        "- Action",
        non_empty(Some(&work_item.work_item.action)),
    );
    lines.push(format!(
        "- Profile: {}",
        work_item.work_item.execution_profile_id
    ));
    lines.push(format!("- Attempt: {}", work_item_run.attempt));
    lines.push(format!("- Run state: {}", work_item_run.state));
    push_value_list(
        &mut lines,
        "- Acceptance criteria",
        acceptance_criteria,
        Some("none specified."),
    );

    lines.push(String::new());
    if upstream_completed_items.is_empty() {
        lines.push("Completed dependencies: none.".to_string());
    } else {
        lines.push("Completed dependencies:".to_string());
        for item in upstream_completed_items {
            let state = item
                .runtime
                .as_ref()
                .map(|runtime| runtime.current_state.as_str())
                .unwrap_or("completed");
            lines.push(format!(
                "- {} [{}] {}",
                item.work_item.work_item_id, state, item.work_item.title
            ));
        }
    }

    lines.push(String::new());
    push_signals(&mut lines, "Open related signals", open_signals);

    lines.push(String::new());
    lines.push("Next:".to_string());
    lines.push("- Execute only this WorkItem.".to_string());
    lines.push("- Call submitResult when finished.".to_string());
    lines
        .push("- Call raiseSignal if blocked, missing input, or replanning is needed.".to_string());

    lines.join("\n")
}

fn planning_role_text(role: &AgentPlanningRole) -> &'static str {
    match role {
        AgentPlanningRole::Planner => "planner",
        AgentPlanningRole::Replanner => "replanner",
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn push_optional(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {value}"));
    }
}

fn push_value_list(lines: &mut Vec<String>, label: &str, value: &Value, empty_text: Option<&str>) {
    let items = value
        .as_array()
        .map(|array| {
            array
                .iter()
                .filter_map(|item| item.as_str().and_then(|text| non_empty(Some(text))))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if items.is_empty() {
        if let Some(empty_text) = empty_text {
            lines.push(format!("{label}: {empty_text}"));
        }
        return;
    }

    lines.push(format!("{label}:"));
    for item in items {
        lines.push(format!("  - {item}"));
    }
}

fn push_signals(lines: &mut Vec<String>, label: &str, signals: &[DagSignalRecord]) {
    if signals.is_empty() {
        lines.push(format!("{label}: none."));
        return;
    }

    lines.push(format!("{label}:"));
    for signal in signals {
        lines.push(format!(
            "- {} [{} / {}]: {}",
            signal.signal_id, signal.severity, signal.kind, signal.summary
        ));
        push_optional(lines, "  Detail", non_empty(signal.detail.as_deref()));
    }
}

#[derive(Clone)]
pub struct AgentToolContextResolver {
    pool: SqlitePool,
}

impl AgentToolContextResolver {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn resolve(&self, request: &AgentToolRequest) -> Result<AgentToolContext> {
        validate_required("session_id", &request.session_id)?;
        validate_required("turn_id", &request.turn_id)?;
        validate_required("runtime_instance_id", &request.runtime_instance_id)?;

        let session = self.load_session(&request.session_id).await?;
        if matches!(session.state.as_str(), "exited" | "error") {
            return Err(Error::StateConflict(format!(
                "session {} is terminal",
                request.session_id
            )));
        }

        let turn = self.load_turn(&request.turn_id).await?;
        if turn.session_id != request.session_id {
            return Err(Error::StateConflict(format!(
                "turn {} does not belong to session {}",
                request.turn_id, request.session_id
            )));
        }

        let runtime_instance_id = self.runtime_instance_id(&request.session_id).await?;
        if runtime_instance_id != request.runtime_instance_id {
            return Err(Error::StateConflict(format!(
                "runtime_instance_id does not match session {}",
                request.session_id
            )));
        }

        if !turn
            .metadata
            .get("dag_managed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Err(Error::StateConflict(format!(
                "turn {} is not DAG-managed",
                request.turn_id
            )));
        }

        let mode = if let Some(role) = turn
            .metadata
            .get("dag_planning_role")
            .and_then(Value::as_str)
        {
            AgentToolMode::Planning {
                role: parse_planning_role(role)?,
            }
        } else {
            let run = self.execution_run_for_turn(&request.turn_id).await?;
            AgentToolMode::Execution {
                run_id: run.run_id,
                work_item_id: run.work_item_id,
            }
        };

        let task_id = match &mode {
            AgentToolMode::Planning { .. } => turn
                .metadata
                .get("task_id")
                .or_else(|| session.metadata.get("task_id"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    Error::StateConflict(format!(
                        "DAG-managed planning turn {} is missing task_id",
                        request.turn_id
                    ))
                })?,
            AgentToolMode::Execution { run_id, .. } => self
                .task_id_for_run(run_id)
                .await?
                .ok_or_else(|| Error::NotFound(format!("work item run {run_id} not found")))?,
        };

        Ok(AgentToolContext {
            session_id: request.session_id.clone(),
            turn_id: request.turn_id.clone(),
            client_type: session.client_type,
            runtime_instance_id,
            task_id,
            mode,
        })
    }

    async fn load_session(&self, session_id: &str) -> Result<SessionForAgentTool> {
        let row = sqlx::query(
            "SELECT session_id, client_type, state, metadata FROM sessions WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| Error::NotFound(format!("session {session_id} not found")))?;
        let metadata: String = row.try_get("metadata")?;
        Ok(SessionForAgentTool {
            client_type: row.try_get("client_type")?,
            state: row.try_get("state")?,
            metadata: serde_json::from_str(&metadata)?,
        })
    }

    async fn load_turn(&self, turn_id: &str) -> Result<TurnForAgentTool> {
        let row =
            sqlx::query("SELECT turn_id, session_id, state, metadata FROM turns WHERE turn_id = ?")
                .bind(turn_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| Error::NotFound(format!("turn {turn_id} not found")))?;
        let metadata: String = row.try_get("metadata")?;
        Ok(TurnForAgentTool {
            session_id: row.try_get("session_id")?,
            metadata: serde_json::from_str(&metadata)?,
        })
    }

    async fn runtime_instance_id(&self, session_id: &str) -> Result<String> {
        let metadata: String =
            sqlx::query_scalar("SELECT metadata FROM runtime_bindings WHERE session_id = ?")
                .bind(session_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| {
                    Error::StateConflict(format!("session {session_id} has no runtime binding"))
                })?;
        let metadata: Value = serde_json::from_str(&metadata)?;
        metadata
            .get("runtime_instance_id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .ok_or_else(|| {
                Error::StateConflict(format!(
                    "session {session_id} runtime binding missing runtime_instance_id"
                ))
            })
    }

    async fn execution_run_for_turn(&self, turn_id: &str) -> Result<ExecutionRunForAgentTool> {
        let row = sqlx::query(
            r#"SELECT run_id, work_item_id
               FROM work_item_runs
               WHERE turn_id = ?
               ORDER BY created_at DESC, run_id DESC LIMIT 1"#,
        )
        .bind(turn_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            Error::StateConflict(format!(
                "DAG-managed turn {turn_id} is not an execution turn"
            ))
        })?;
        Ok(ExecutionRunForAgentTool {
            run_id: row.try_get("run_id")?,
            work_item_id: row.try_get("work_item_id")?,
        })
    }

    async fn task_id_for_run(&self, run_id: &str) -> Result<Option<String>> {
        sqlx::query_scalar("SELECT task_id FROM work_item_runs WHERE run_id = ?")
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::from)
    }
}

struct SessionForAgentTool {
    client_type: String,
    state: String,
    metadata: Value,
}

struct TurnForAgentTool {
    session_id: String,
    metadata: Value,
}

struct ExecutionRunForAgentTool {
    run_id: String,
    work_item_id: String,
}

fn default_agent_tool_input() -> Value {
    json!({})
}

fn is_known_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "getContext" | "submitPlan" | "submitResult" | "raiseSignal"
    )
}

fn validate_required(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(Error::Domain(format!("{field} is required")))
    } else {
        Ok(())
    }
}

fn parse_planning_role(role: &str) -> Result<AgentPlanningRole> {
    match role {
        "planner" => Ok(AgentPlanningRole::Planner),
        "replanner" => Ok(AgentPlanningRole::Replanner),
        other => Err(Error::StateConflict(format!(
            "unsupported DAG planning role {other}"
        ))),
    }
}

fn parse_submit_plan_initial_input(input: Value) -> Result<SubmitPlanPayload> {
    let mode = input
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("initial_dag");
    if mode != "initial_dag" {
        return Err(Error::Domain(format!(
            "submitPlan initial payload mode must be initial_dag, got {mode}"
        )));
    }
    let dag = input.get("dag").unwrap_or(&input);
    Ok(SubmitPlanPayload {
        mode: "initial_dag".to_string(),
        summary: required_input_string(&input, "summary")?,
        work_items: serde_json::from_value(
            dag.get("work_items").cloned().unwrap_or_else(|| json!([])),
        )?,
        edges: serde_json::from_value(dag.get("edges").cloned().unwrap_or_else(|| json!([])))?,
        assumptions: serde_json::from_value(
            input
                .get("assumptions")
                .cloned()
                .unwrap_or_else(|| json!([])),
        )?,
        risks: serde_json::from_value(input.get("risks").cloned().unwrap_or_else(|| json!([])))?,
    })
}

fn parse_submit_plan_patch_input(input: Value) -> Result<(String, DagPatch)> {
    let mode = input.get("mode").and_then(Value::as_str).unwrap_or("patch");
    if mode != "patch" {
        return Err(Error::Domain(format!(
            "submitPlan patch payload mode must be patch, got {mode}"
        )));
    }
    let summary = required_input_string(&input, "summary")?;
    let mut patch_value = input.get("patch").cloned().unwrap_or_else(
        || json!({"operations": input.get("operations").cloned().unwrap_or_else(|| json!([]))}),
    );
    if patch_value.get("summary").is_none()
        && let Some(object) = patch_value.as_object_mut()
    {
        object.insert("summary".to_string(), Value::String(summary.clone()));
    }
    let mut patch: DagPatch = serde_json::from_value(patch_value)?;
    if patch.summary.is_empty() {
        patch.summary = summary.clone();
    }
    Ok((summary, patch))
}

fn required_input_string(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| Error::Domain(format!("submitPlan input missing string field {key}")))
}

async fn reject_duplicate_successful_submit_plan(
    pool: &SqlitePool,
    context: &AgentToolContext,
) -> Result<()> {
    let existing: Option<String> = sqlx::query_scalar(
        r#"SELECT proposal_id FROM dag_proposals
           WHERE task_id = ? AND created_by_session_id = ? AND state = 'applied'
           ORDER BY created_at DESC, proposal_id DESC LIMIT 1"#,
    )
    .bind(&context.task_id)
    .bind(&context.session_id)
    .fetch_optional(pool)
    .await?;
    if let Some(proposal_id) = existing {
        Err(Error::StateConflict(format!(
            "submitPlan already applied proposal {proposal_id} for this planning session"
        )))
    } else {
        Ok(())
    }
}
