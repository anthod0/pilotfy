use super::*;

impl RuntimeControlService {
    pub(super) async fn runtime_ref(&self, session_id: &str) -> Result<Option<String>> {
        sqlx::query_scalar("SELECT runtime_ref FROM runtime_bindings WHERE session_id = ?")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub(super) async fn restart_count(&self, session_id: &str) -> Result<Option<i64>> {
        let metadata: Option<String> =
            sqlx::query_scalar("SELECT metadata FROM runtime_bindings WHERE session_id = ?")
                .bind(session_id)
                .fetch_optional(&self.pool)
                .await?;
        metadata
            .map(|metadata| {
                serde_json::from_str::<Value>(&metadata)
                    .map(|value| value["restart_count"].as_i64().unwrap_or(0))
            })
            .transpose()
            .map_err(Into::into)
    }

    pub(super) async fn upsert_runtime_binding(
        &self,
        session_id: &str,
        runtime: &RuntimeStartResult,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO runtime_bindings (session_id, runtime_kind, runtime_ref, metadata)
               VALUES (?, ?, ?, ?)
               ON CONFLICT(session_id) DO UPDATE SET
                   runtime_kind = excluded.runtime_kind,
                   runtime_ref = excluded.runtime_ref,
                   metadata = excluded.metadata,
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')"#,
        )
        .bind(session_id)
        .bind(&runtime.runtime_kind)
        .bind(&runtime.runtime_ref)
        .bind(serde_json::to_string(&runtime.binding_metadata())?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
