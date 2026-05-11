use super::*;

impl TaskCommandService {
    pub(super) async fn run_initial_planner_attempt(
        &self,
        task_id: &str,
        request: &CreateTaskRequest,
    ) -> Result<Value> {
        let planner = TaskPlannerService::new(self.pool.clone(), FakeTaskPlanner);
        let input = planner
            .build_input(
                task_id,
                request.input.clone(),
                request.metadata.clone(),
                None,
            )
            .await?;
        self.apply_planner_attempt(task_id, &request.client_type, input)
            .await
    }

    pub(super) async fn apply_planner_attempt(
        &self,
        task_id: &str,
        client_type: &str,
        input: PlannerInput,
    ) -> Result<Value> {
        sqlx::query(
            r#"UPDATE tasks
               SET state = 'routing', routing_state = 'pending', routing_reason = NULL,
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE task_id = ?"#,
        )
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        self.record_task_event(
            task_id,
            "task.planning_started",
            json!({"planner_client_type": self.planner.client_type}),
        )
        .await?;

        let decision = match self.plan_with_config(input).await {
            Ok(decision) => decision,
            Err(error) => {
                self.apply_planner_failed(task_id, &error.to_string(), None)
                    .await?;
                return self.task_data(task_id).await;
            }
        };

        self.record_task_event(
            task_id,
            "task.planning_completed",
            json!({"decision": decision}),
        )
        .await?;

        match decision.status {
            PlannerDecisionStatus::Resolved => {
                self.apply_planner_resolved(task_id, client_type, &decision)
                    .await?;
            }
            PlannerDecisionStatus::NeedsInput => {
                let question = decision
                    .needs_input
                    .as_ref()
                    .map(|needs_input| needs_input.question.clone())
                    .unwrap_or_else(|| "Planner needs more input".to_string());
                sqlx::query(
                    r#"UPDATE tasks
                       SET state = 'needs_confirmation', routing_state = 'ambiguous',
                           routing_reason = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                       WHERE task_id = ?"#,
                )
                .bind(&question)
                .bind(task_id)
                .execute(&self.pool)
                .await?;
                self.record_task_event(
                    task_id,
                    "task.planning_needs_input",
                    json!({"decision": decision, "question": question}),
                )
                .await?;
            }
            PlannerDecisionStatus::Failed => {
                let reason = decision
                    .reason
                    .as_deref()
                    .unwrap_or("planner failed")
                    .to_string();
                self.apply_planner_failed(task_id, &reason, Some(decision))
                    .await?;
            }
        }

        self.task_data(task_id).await
    }

    pub(super) async fn plan_with_config(&self, input: PlannerInput) -> Result<PlannerDecision> {
        if self.planner.client_type == "pi" {
            TaskPlannerService::new(
                self.pool.clone(),
                PiTaskPlanner::new(std::time::Duration::from_millis(self.planner.timeout_ms)),
            )
            .plan(input)
            .await
        } else {
            TaskPlannerService::new(self.pool.clone(), FakeTaskPlanner)
                .plan(input)
                .await
        }
    }

    pub(super) async fn apply_planner_resolved(
        &self,
        task_id: &str,
        client_type: &str,
        decision: &PlannerDecision,
    ) -> Result<()> {
        let workspace = decision.workspace.as_ref().ok_or_else(|| {
            Error::Domain("resolved planner decision missing workspace".to_string())
        })?;
        let workspace_record = if let Some(workspace_id) = workspace.workspace_id.as_deref() {
            let row = sqlx::query(
                "SELECT workspace_id, canonical_path FROM workspaces WHERE workspace_id = ?",
            )
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await?;
            if let Some(row) = row {
                WorkspaceRecord {
                    workspace_id: row.try_get("workspace_id")?,
                    canonical_path: row.try_get("canonical_path")?,
                }
            } else if let Some(canonical_path) = workspace.canonical_path.as_deref() {
                upsert_workspace(&self.pool, canonical_path).await?
            } else {
                return Err(Error::Domain(format!(
                    "planner resolved unknown workspace_id {workspace_id}"
                )));
            }
        } else {
            upsert_workspace(
                &self.pool,
                workspace.canonical_path.as_deref().ok_or_else(|| {
                    Error::Domain("resolved planner decision missing canonical_path".to_string())
                })?,
            )
            .await?
        };

        let confidence = workspace.confidence.unwrap_or(1.0).clamp(0.0, 1.0);
        let reason = workspace
            .reason
            .as_deref()
            .or(decision.reason.as_deref())
            .unwrap_or("planner resolved workspace");
        sqlx::query(
            r#"UPDATE tasks
               SET state = 'routing', workspace_id = ?, routing_state = 'matched',
                   routing_confidence = ?, routing_reason = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE task_id = ?"#,
        )
        .bind(&workspace_record.workspace_id)
        .bind(confidence)
        .bind(reason)
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        self.record_task_event(
            task_id,
            "task.planning_resolved",
            json!({"decision": decision, "workspace_id": workspace_record.workspace_id.clone(), "canonical_path": workspace_record.canonical_path.clone()}),
        )
        .await?;

        let handoff_id = format!("handoff_{}", uuid::Uuid::now_v7());
        self.record_task_event(
            task_id,
            "task.dispatch_handoff_created",
            json!({
                "handoff_id": handoff_id,
                "decision_id": decision.decision_id.clone(),
                "task_id": task_id,
                "workspace_id": workspace_record.workspace_id.clone(),
                "canonical_path": workspace_record.canonical_path.clone(),
                "client_type": client_type,
                "planner_status": "resolved",
                "reason": reason
            }),
        )
        .await?;

        if self.planner.compatibility_direct_dispatch {
            let task = ExternalQueryService::new(self.pool.clone())
                .get_task(task_id)
                .await?
                .ok_or_else(|| Error::Domain("planned task missing".to_string()))?;
            self.dispatch_task(
                task_id,
                &workspace_record.canonical_path,
                client_type,
                task.input,
                task.metadata,
                DispatchRoutingUpdate::Matched,
            )
            .await?;
        }
        Ok(())
    }

    pub(super) async fn apply_planner_failed(
        &self,
        task_id: &str,
        reason: &str,
        decision: Option<PlannerDecision>,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE tasks
               SET state = 'needs_confirmation', routing_state = 'failed', routing_reason = ?,
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE task_id = ?"#,
        )
        .bind(reason)
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        self.record_task_event(
            task_id,
            "task.planning_failed",
            json!({"reason": reason, "decision": decision}),
        )
        .await?;
        Ok(())
    }

    pub(super) async fn task_data(&self, task_id: &str) -> Result<Value> {
        let task = ExternalQueryService::new(self.pool.clone())
            .get_task(task_id)
            .await?
            .ok_or_else(|| Error::Domain("task missing".to_string()))?;
        Ok(json!({ "task": task }))
    }
}
