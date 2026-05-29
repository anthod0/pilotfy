use super::*;

impl ExternalQueryService {
    pub async fn get_task_dag(&self, task_id: &str) -> Result<TaskDagView> {
        let summary = self.get_task_dag_summary(task_id).await?;
        let work_items = self.list_work_items(task_id).await?;
        let edges = self.list_work_item_edges(task_id).await?;
        let runs = self.list_work_item_runs(task_id).await?;
        let signals = self.list_dag_signals(task_id).await?;
        Ok(TaskDagView {
            task_id: task_id.to_string(),
            summary,
            work_items,
            edges,
            runs,
            signals,
        })
    }

    pub async fn get_task_dag_summary(&self, task_id: &str) -> Result<DagSummaryView> {
        let graph = self.task_graph_snapshot(task_id).await?;
        let runtime = self.runtime_map(task_id).await?;
        let active_ids: std::collections::HashSet<_> = graph
            .work_items
            .iter()
            .filter(|work_item| work_item.active)
            .map(|work_item| work_item.work_item_id.as_str())
            .collect();

        let mut summary = DagSummaryView {
            total_work_items: active_ids.len() as i64,
            ready_work_items: 0,
            running_work_items: 0,
            completed_work_items: 0,
            blocked_work_items: 0,
            failed_work_items: 0,
            open_signals: 0,
            total_runs: 0,
        };

        for (work_item_id, runtime) in &runtime {
            if !active_ids.contains(work_item_id.as_str()) {
                continue;
            }
            match runtime.current_state.as_str() {
                "ready" => summary.ready_work_items += 1,
                "running" => summary.running_work_items += 1,
                "completed" => summary.completed_work_items += 1,
                "blocked" | "needs_input" => summary.blocked_work_items += 1,
                "failed" => summary.failed_work_items += 1,
                _ => {}
            }
        }

        summary.open_signals = self.count_open_signals(task_id).await?;
        summary.total_runs = self.count_work_item_runs(task_id).await?;
        Ok(summary)
    }

    pub async fn list_work_items(&self, task_id: &str) -> Result<Vec<WorkItemWithRuntimeView>> {
        let graph = self.task_graph_snapshot(task_id).await?;
        let runtime = self.runtime_map(task_id).await?;
        Ok(graph
            .work_items
            .into_iter()
            .map(|node| {
                let runtime = runtime.get(&node.work_item_id).cloned();
                WorkItemWithRuntimeView {
                    work_item: work_item_node_to_record(node),
                    runtime,
                }
            })
            .collect())
    }

    pub async fn list_work_item_edges(&self, task_id: &str) -> Result<Vec<WorkItemEdgeView>> {
        Ok(self
            .task_graph_snapshot(task_id)
            .await?
            .edges
            .into_iter()
            .map(graph_edge_record_to_view)
            .collect())
    }

    pub async fn list_work_item_runs(&self, task_id: &str) -> Result<Vec<WorkItemRunRecord>> {
        let rows = sqlx::query(
            r#"SELECT run_id, work_item_id, task_id, attempt, state, session_id, turn_id,
                      client_type, execution_profile_id, execution_profile_version,
                      rendered_prompt_ref, output_summary, failure, created_at, updated_at,
                      started_at, completed_at
               FROM work_item_runs WHERE task_id = ? ORDER BY created_at, run_id"#,
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_work_item_run_record).collect()
    }

    pub async fn list_dag_signals(&self, task_id: &str) -> Result<Vec<DagSignalRecord>> {
        let rows = sqlx::query(
            r#"SELECT signal_id, task_id, work_item_id, run_id, source_session_id, source, kind,
                      summary, detail, severity, related_refs, state, created_at, updated_at
               FROM dag_signals WHERE task_id = ? ORDER BY created_at, signal_id"#,
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_dag_signal_record).collect()
    }

    async fn runtime_map(
        &self,
        task_id: &str,
    ) -> Result<std::collections::HashMap<String, WorkItemRuntimeView>> {
        let rows = sqlx::query(
            r#"SELECT work_item_id, current_run_id, current_state, current_attempt, ready_at,
                      blocked_reason, outcome_state, outcome_reason, replanned_from_state,
                      retry_count, max_retries, priority, optional,
                      parallelizable, session_id, turn_id, updated_at
               FROM work_item_runtime_projection
               WHERE task_id = ?"#,
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        let mut runtime = std::collections::HashMap::new();
        for row in rows {
            runtime.insert(
                row.try_get("work_item_id")?,
                WorkItemRuntimeView {
                    current_run_id: row.try_get("current_run_id")?,
                    current_state: row.try_get("current_state")?,
                    current_attempt: row.try_get("current_attempt")?,
                    ready_at: row.try_get("ready_at")?,
                    blocked_reason: row.try_get("blocked_reason")?,
                    outcome_state: row.try_get("outcome_state")?,
                    outcome_reason: row.try_get("outcome_reason")?,
                    replanned_from_state: row.try_get("replanned_from_state")?,
                    retry_count: row.try_get("retry_count")?,
                    max_retries: row.try_get("max_retries")?,
                    priority: row.try_get("priority")?,
                    optional: row.try_get("optional")?,
                    parallelizable: row.try_get("parallelizable")?,
                    session_id: row.try_get("session_id")?,
                    turn_id: row.try_get("turn_id")?,
                    updated_at: row.try_get("updated_at")?,
                },
            );
        }
        Ok(runtime)
    }

    async fn count_open_signals(&self, task_id: &str) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM dag_signals WHERE task_id = ? AND state = 'open'",
        )
        .bind(task_id)
        .fetch_one(&self.pool)
        .await?)
    }

    async fn count_work_item_runs(&self, task_id: &str) -> Result<i64> {
        Ok(
            sqlx::query_scalar("SELECT COUNT(*) FROM work_item_runs WHERE task_id = ?")
                .bind(task_id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    async fn task_graph_snapshot(&self, task_id: &str) -> Result<TaskGraphSnapshot> {
        GraphProjectionService::new(self.pool.clone(), self.graph.clone())
            .task_graph(task_id)
            .await
    }
}
