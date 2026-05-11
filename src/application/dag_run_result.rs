use super::*;

#[derive(Debug, Clone)]
struct RunForTurn {
    run_id: String,
    work_item_id: String,
    task_id: String,
    session_id: Option<String>,
    state: String,
}

#[derive(Debug, Clone)]
struct ParsedRunResult {
    state: String,
    summary: String,
    failure: Option<Value>,
    signals: Vec<RaiseSignalPayload>,
}

#[derive(Clone)]
pub struct DagRunResultService {
    pool: SqlitePool,
}

impl DagRunResultService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn sync_from_turn_event(&self, event: &DomainEvent) -> Result<()> {
        let Some(turn_id) = event.turn_id.as_deref() else {
            return Ok(());
        };
        let Some(run) = self.run_for_turn(turn_id).await? else {
            return Ok(());
        };

        match event.event_type {
            EventType::TurnStarted => {
                self.mark_started(&run).await?;
                Ok(())
            }
            EventType::TurnCompleted => {
                self.handle_terminal(event, &run, self.completed_result(event)?)
                    .await
            }
            EventType::TurnFailed => {
                let summary = failure_summary(&event.payload);
                self.handle_terminal(
                    event,
                    &run,
                    ParsedRunResult {
                        state: "failed".to_string(),
                        summary: summary.clone(),
                        failure: Some(json!({ "message": summary })),
                        signals: Vec::new(),
                    },
                )
                .await
            }
            EventType::TurnCancelled | EventType::TurnInterrupted => {
                self.handle_terminal(
                    event,
                    &run,
                    ParsedRunResult {
                        state: "cancelled".to_string(),
                        summary: terminal_summary(&event.payload)
                            .unwrap_or_else(|| event.event_type.to_string()),
                        failure: None,
                        signals: Vec::new(),
                    },
                )
                .await
            }
            _ => Ok(()),
        }
    }

    async fn run_for_turn(&self, turn_id: &str) -> Result<Option<RunForTurn>> {
        let row = sqlx::query(
            r#"SELECT run_id, work_item_id, task_id, session_id, state
               FROM work_item_runs WHERE turn_id = ?
               ORDER BY created_at DESC, run_id DESC LIMIT 1"#,
        )
        .bind(turn_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| {
            Ok(RunForTurn {
                run_id: row.try_get("run_id")?,
                work_item_id: row.try_get("work_item_id")?,
                task_id: row.try_get("task_id")?,
                session_id: row.try_get("session_id")?,
                state: row.try_get("state")?,
            })
        })
        .transpose()
    }

    async fn mark_started(&self, run: &RunForTurn) -> Result<()> {
        sqlx::query(
            r#"UPDATE work_item_runs
               SET state = 'running', started_at = COALESCE(started_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE run_id = ? AND state IN ('queued', 'running')"#,
        )
        .bind(&run.run_id)
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"UPDATE work_item_runtime_projection
               SET current_state = 'running', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE current_run_id = ?"#,
        )
        .bind(&run.run_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn handle_terminal(
        &self,
        event: &DomainEvent,
        run: &RunForTurn,
        result: ParsedRunResult,
    ) -> Result<()> {
        if is_terminal_run_state(&run.state) {
            return Ok(());
        }

        let failure_json = result
            .failure
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let blocked_reason = if matches!(
            result.state.as_str(),
            "blocked" | "needs_input" | "cancelled"
        ) {
            Some(result.summary.as_str())
        } else {
            None
        };

        let mut signal_ids = Vec::new();
        let mut replan_signal_ids = Vec::new();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"UPDATE work_item_runs
               SET state = ?, output_summary = ?, failure = ?,
                   completed_at = COALESCE(completed_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE run_id = ?"#,
        )
        .bind(&result.state)
        .bind(&result.summary)
        .bind(failure_json)
        .bind(&run.run_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            r#"UPDATE work_item_runtime_projection
               SET current_state = ?, blocked_reason = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE work_item_id = ? AND current_run_id = ?"#,
        )
        .bind(&result.state)
        .bind(blocked_reason)
        .bind(&run.work_item_id)
        .bind(&run.run_id)
        .execute(&mut *tx)
        .await?;

        for signal in &result.signals {
            let signal_id = new_dag_run_result_id("dagsig");
            if signal.kind == "replan_requested" {
                replan_signal_ids.push(signal_id.clone());
            }
            signal_ids.push(signal_id.clone());
            sqlx::query(
                r#"INSERT INTO dag_signals (
                        signal_id, task_id, work_item_id, run_id, source_session_id,
                        kind, summary, detail, severity, related_refs
                   ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&signal_id)
            .bind(&run.task_id)
            .bind(&run.work_item_id)
            .bind(&run.run_id)
            .bind(run.session_id.as_deref())
            .bind(&signal.kind)
            .bind(&signal.summary)
            .bind(&signal.detail)
            .bind(normalize_severity(&signal.severity))
            .bind(serde_json::to_string(&signal.related_refs)?)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        self.record_task_event(
            &run.task_id,
            "dag.run_completed",
            json!({
                "run_id": run.run_id,
                "work_item_id": run.work_item_id,
                "turn_id": event.turn_id,
                "state": result.state,
                "signals": signal_ids,
                "domain_event_id": event.event_id,
            }),
        )
        .await?;

        if !replan_signal_ids.is_empty() {
            for signal_id in replan_signal_ids {
                Box::pin(
                    DagPlanningService::new(self.pool.clone())
                        .start_replanning_for_signal(&run.task_id, &signal_id),
                )
                .await?;
            }
            return Ok(());
        }

        if result.state == "completed" {
            Box::pin(DagSchedulerService::new(self.pool.clone()).schedule_task(&run.task_id))
                .await?;
        }

        self.aggregate_task_state(&run.task_id).await
    }

    fn completed_result(&self, event: &DomainEvent) -> Result<ParsedRunResult> {
        if let Ok(payload) = serde_json::from_value::<SubmitResultPayload>(event.payload.clone()) {
            return Ok(parsed_payload_to_result(payload));
        }
        if let Some(output) = event.payload.get("output")
            && let Ok(payload) = serde_json::from_value::<SubmitResultPayload>(output.clone())
        {
            return Ok(parsed_payload_to_result(payload));
        }

        let raw = terminal_summary(&event.payload).unwrap_or_else(|| event.payload.to_string());
        match serde_json::from_str::<SubmitResultPayload>(&raw) {
            Ok(payload) => Ok(parsed_payload_to_result(payload)),
            Err(_) => Ok(ParsedRunResult {
                state: "completed".to_string(),
                summary: raw,
                failure: None,
                signals: Vec::new(),
            }),
        }
    }

    async fn aggregate_task_state(&self, task_id: &str) -> Result<()> {
        let rows = sqlx::query(
            r#"SELECT p.current_state, wi.optional
               FROM work_items wi
               JOIN work_item_runtime_projection p ON p.work_item_id = wi.work_item_id
               WHERE wi.task_id = ? AND wi.active = 1"#,
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        if rows.is_empty() {
            return Ok(());
        }

        let mut required = Vec::new();
        for row in rows {
            let optional: bool = row.try_get("optional")?;
            if !optional {
                required.push(row.try_get::<String, _>("current_state")?);
            }
        }
        if required.is_empty() {
            return Ok(());
        }

        let next_state = if required.iter().all(|state| state == "completed") {
            "completed"
        } else if required.iter().any(|state| state == "failed") {
            "failed"
        } else if required
            .iter()
            .any(|state| matches!(state.as_str(), "blocked" | "needs_input" | "cancelled"))
        {
            "blocked"
        } else {
            "running"
        };

        sqlx::query(
            r#"UPDATE tasks
               SET state = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE task_id = ? AND state NOT IN ('completed', 'failed', 'cancelled', 'replanning')"#,
        )
        .bind(next_state)
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        self.record_task_event(
            task_id,
            match next_state {
                "completed" => "task.completed",
                "failed" => "task.failed",
                "blocked" => "task.blocked",
                _ => "task.running",
            },
            json!({ "source": "dag_aggregate" }),
        )
        .await?;
        Ok(())
    }

    async fn record_task_event(
        &self,
        task_id: &str,
        event_type: &str,
        payload: Value,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO task_events (event_id, task_id, event_type, payload) VALUES (?, ?, ?, ?)",
        )
        .bind(new_event_id().to_string())
        .bind(task_id)
        .bind(event_type)
        .bind(serde_json::to_string(&payload)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn terminal_summary(payload: &Value) -> Option<String> {
    nested_string(payload, &["output", "summary"])
        .or_else(|| nested_string(payload, &["output_summary"]))
        .or_else(|| nested_string(payload, &["summary"]))
        .or_else(|| nested_string(payload, &["output", "text"]))
        .or_else(|| nested_string(payload, &["output", "content"]))
        .or_else(|| {
            payload
                .get("output")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn failure_summary(payload: &Value) -> String {
    nested_string(payload, &["failure", "message"])
        .or_else(|| nested_string(payload, &["message"]))
        .unwrap_or_else(|| "turn failed".to_string())
}

fn parsed_payload_to_result(payload: SubmitResultPayload) -> ParsedRunResult {
    ParsedRunResult {
        state: normalize_result_status(&payload.status),
        summary: payload.summary,
        failure: payload.failure,
        signals: payload.signals,
    }
}

fn normalize_result_status(status: &str) -> String {
    match status {
        "completed" | "failed" | "blocked" | "needs_input" => status.to_string(),
        _ => "completed".to_string(),
    }
}

fn normalize_severity(severity: &str) -> &str {
    match severity {
        "low" | "medium" | "high" => severity,
        _ => "medium",
    }
}

fn is_terminal_run_state(state: &str) -> bool {
    matches!(
        state,
        "completed" | "failed" | "blocked" | "needs_input" | "cancelled"
    )
}

fn new_dag_run_result_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::now_v7())
}
