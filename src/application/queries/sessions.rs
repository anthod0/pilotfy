use super::*;

impl ExternalQueryService {
    pub async fn list_sessions(&self) -> Result<Vec<SessionView>> {
        let rows = sqlx::query(
            r#"SELECT s.session_id, s.client_type, s.title, s.handle, s.role, s.description,
                      s.execution_profile_id, s.execution_profile_version,
                      s.state, s.current_turn_id, s.workspace_id,
                      COALESCE(w.canonical_path, s.workspace_ref) AS workspace_ref,
                      s.metadata, s.created_at, s.updated_at
               FROM sessions s
               LEFT JOIN workspaces w ON w.workspace_id = s.workspace_id
               ORDER BY s.created_at, s.session_id"#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = rows
            .into_iter()
            .map(row_to_session_view)
            .collect::<Result<Vec<_>>>()?;
        for session in &mut sessions {
            self.enrich_session_view(session).await?;
        }
        Ok(sessions)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionView>> {
        let row = sqlx::query(
            r#"SELECT s.session_id, s.client_type, s.title, s.handle, s.role, s.description,
                      s.execution_profile_id, s.execution_profile_version,
                      s.state, s.current_turn_id, s.workspace_id,
                      COALESCE(w.canonical_path, s.workspace_ref) AS workspace_ref,
                      s.metadata, s.created_at, s.updated_at
               FROM sessions s
               LEFT JOIN workspaces w ON w.workspace_id = s.workspace_id
               WHERE s.session_id = ?"#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        let mut session = row_to_session_view(row)?;
        self.enrich_session_view(&mut session).await?;
        Ok(Some(session))
    }

    async fn enrich_session_view(&self, session: &mut SessionView) -> Result<()> {
        let row = sqlx::query("SELECT metadata FROM runtime_bindings WHERE session_id = ?")
            .bind(&session.session_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let metadata: String = row.try_get("metadata")?;
            let metadata: Value = serde_json::from_str(&metadata)?;
            if let Some(capabilities) = metadata.get("capabilities") {
                session.capabilities = serde_json::from_value(capabilities.clone())?;
            }
        }

        Ok(())
    }
}
