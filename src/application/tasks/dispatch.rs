use super::*;

impl TaskCommandService {
    pub async fn sync_task_from_turn_event(&self, event: &DomainEvent) -> Result<()> {
        let Some(turn_id) = event.turn_id.as_deref() else {
            return Ok(());
        };
        let Some((task_id, current_state)) = sqlx::query_as::<_, (String, String)>(
            "SELECT task_id, state FROM tasks WHERE turn_id = ?",
        )
        .bind(turn_id)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(());
        };

        if is_terminal_task_state(&current_state) {
            return Ok(());
        }

        let transition = match event.event_type {
            EventType::TurnStarted => Some(("running", "task.running")),
            EventType::TurnCompleted => Some(("completed", "task.completed")),
            EventType::TurnFailed => Some(("failed", "task.failed")),
            EventType::TurnInterrupted | EventType::TurnCancelled => {
                Some(("cancelled", "task.cancelled"))
            }
            _ => None,
        };
        let Some((next_state, task_event_type)) = transition else {
            return Ok(());
        };

        sqlx::query(
            r#"UPDATE tasks
               SET state = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE task_id = ? AND turn_id = ?"#,
        )
        .bind(next_state)
        .bind(&task_id)
        .bind(turn_id)
        .execute(&self.pool)
        .await?;
        self.record_task_event(
            &task_id,
            task_event_type,
            json!({"turn_id": turn_id, "domain_event_id": event.event_id}),
        )
        .await?;
        Ok(())
    }

    pub(super) async fn dispatch_task(
        &self,
        task_id: &str,
        workspace: &str,
        client_type: &str,
        input: String,
        metadata: Value,
        routing_update: DispatchRoutingUpdate,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE tasks
               SET state = 'routing', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE task_id = ?"#,
        )
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        self.record_task_event(task_id, "task.routing_started", json!({}))
            .await?;

        let workspace_record = upsert_workspace(&self.pool, workspace).await?;
        let routing_state = match routing_update {
            DispatchRoutingUpdate::Matched => "matched",
            DispatchRoutingUpdate::Confirmed => "confirmed",
        };
        sqlx::query(
            r#"UPDATE tasks
               SET workspace_id = ?, routing_state = ?, routing_confidence = 1.0,
                   routing_reason = NULL,
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE task_id = ?"#,
        )
        .bind(&workspace_record.workspace_id)
        .bind(routing_state)
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        let workspace_event = match routing_update {
            DispatchRoutingUpdate::Matched => "task.workspace_matched",
            DispatchRoutingUpdate::Confirmed => "task.workspace_confirmed",
        };
        self.record_task_event(
            task_id,
            workspace_event,
            json!({"workspace_id": workspace_record.workspace_id, "canonical_path": workspace_record.canonical_path}),
        )
        .await?;

        let session_id = self
            .find_idle_session(&workspace_record.workspace_id, client_type)
            .await?;
        let session_id = if let Some(session_id) = session_id {
            self.record_task_event(
                task_id,
                "task.session_selected",
                json!({"session_id": session_id}),
            )
            .await?;
            session_id
        } else {
            let session_outcome = SessionCommandService::new(self.pool.clone())
                .create_session(
                    CreateSessionRequest {
                        client_type: client_type.to_string(),
                        workspace: Some(workspace_record.canonical_path.clone()),
                        workspace_id: None,
                        handle: None,
                        role: None,
                        description: None,
                        execution_profile_id: None,
                        execution_profile_version: None,
                        metadata: json!({"created_for_task_id": task_id}),
                        initial_task: None,
                    },
                    None,
                )
                .await?;
            let session_id = session_outcome.data["session"]["session_id"]
                .as_str()
                .ok_or_else(|| {
                    Error::Domain("created session response missing session_id".to_string())
                })?
                .to_string();
            self.record_task_event(
                task_id,
                "task.session_created",
                json!({"session_id": session_id}),
            )
            .await?;
            session_id
        };

        sqlx::query(
            r#"UPDATE tasks
               SET state = 'queued', session_id = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE task_id = ?"#,
        )
        .bind(&session_id)
        .bind(task_id)
        .execute(&self.pool)
        .await?;

        let turn = TurnCommandService::new(self.pool.clone())
            .create_and_dispatch_turn(&session_id, input, metadata)
            .await?;
        let turn_id = turn.turn_id;
        let turn_state = turn.state.as_str();
        let task_state = if turn_state == "running" {
            "running"
        } else {
            "queued"
        };
        sqlx::query(
            r#"UPDATE tasks
               SET state = ?, turn_id = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE task_id = ?"#,
        )
        .bind(task_state)
        .bind(&turn_id)
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        self.record_task_event(task_id, "task.turn_created", json!({"turn_id": turn_id}))
            .await?;
        Ok(())
    }

    pub(super) async fn find_idle_session(
        &self,
        workspace_id: &str,
        client_type: &str,
    ) -> Result<Option<String>> {
        sqlx::query_scalar(
            r#"SELECT session_id FROM sessions
               WHERE workspace_id = ? AND client_type = ? AND state IN ('idle', 'interrupted')
                 AND current_turn_id IS NULL
               ORDER BY updated_at DESC, session_id LIMIT 1"#,
        )
        .bind(workspace_id)
        .bind(client_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub(super) async fn mark_task_failed(&self, task_id: &str, reason: &str) -> Result<()> {
        sqlx::query(
            r#"UPDATE tasks
               SET state = 'failed', routing_state = 'failed', routing_reason = ?,
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               WHERE task_id = ?"#,
        )
        .bind(reason)
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        self.record_task_event(task_id, "task.failed", json!({"reason": reason}))
            .await?;
        Ok(())
    }
}
