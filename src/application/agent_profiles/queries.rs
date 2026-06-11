use super::rows::row_to_execution_profile_view;
use super::*;

impl AgentProfileService {
    pub async fn list_latest(&self) -> Result<Vec<ExecutionProfileView>> {
        let rows = sqlx::query(
            r#"SELECT profile_id, version, name, description, supported_client_types, agent_kind,
                      system_prompt_template, turn_prompt_template, default_session_role,
                      default_session_description, handle_prefix,
                      expected_output_schema, artifact_contract, default_execution_policy,
                      default_review_policy, metadata, active, archived_at, archived_reason,
                      created_at, updated_at
               FROM execution_profiles ep
               WHERE active = 1 AND archived_at IS NULL AND rowid = (
                   SELECT max(rowid) FROM execution_profiles latest
                   WHERE latest.profile_id = ep.profile_id
                     AND latest.active = 1
                     AND latest.archived_at IS NULL
               )
               ORDER BY profile_id"#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(row_to_execution_profile_view)
            .collect()
    }

    pub async fn list_latest_including_archived(&self) -> Result<Vec<ExecutionProfileView>> {
        let rows = sqlx::query(
            r#"SELECT profile_id, version, name, description, supported_client_types, agent_kind,
                      system_prompt_template, turn_prompt_template, default_session_role,
                      default_session_description, handle_prefix,
                      expected_output_schema, artifact_contract, default_execution_policy,
                      default_review_policy, metadata, active, archived_at, archived_reason,
                      created_at, updated_at
               FROM execution_profiles ep
               WHERE rowid = (
                   SELECT max(rowid) FROM execution_profiles latest
                   WHERE latest.profile_id = ep.profile_id
               )
               ORDER BY profile_id"#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(row_to_execution_profile_view)
            .collect()
    }

    pub async fn get_latest(&self, profile_id: &str) -> Result<Option<ExecutionProfileView>> {
        let row = sqlx::query(
            r#"SELECT profile_id, version, name, description, supported_client_types, agent_kind,
                      system_prompt_template, turn_prompt_template, default_session_role,
                      default_session_description, handle_prefix,
                      expected_output_schema, artifact_contract, default_execution_policy,
                      default_review_policy, metadata, active, archived_at, archived_reason,
                      created_at, updated_at
               FROM execution_profiles
               WHERE profile_id = ? AND active = 1 AND archived_at IS NULL
               ORDER BY rowid DESC
               LIMIT 1"#,
        )
        .bind(profile_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_execution_profile_view).transpose()
    }

    pub async fn list_versions(
        &self,
        profile_id: &str,
        include_archived: bool,
    ) -> Result<Vec<ExecutionProfileView>> {
        let rows = if include_archived {
            sqlx::query(
                r#"SELECT profile_id, version, name, description, supported_client_types, agent_kind,
                          system_prompt_template, turn_prompt_template, default_session_role,
                          default_session_description, handle_prefix,
                          expected_output_schema, artifact_contract, default_execution_policy,
                          default_review_policy, metadata, active, archived_at, archived_reason,
                          created_at, updated_at
                   FROM execution_profiles
                   WHERE profile_id = ?
                   ORDER BY rowid"#,
            )
            .bind(profile_id)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"SELECT profile_id, version, name, description, supported_client_types, agent_kind,
                          system_prompt_template, turn_prompt_template, default_session_role,
                          default_session_description, handle_prefix,
                          expected_output_schema, artifact_contract, default_execution_policy,
                          default_review_policy, metadata, active, archived_at, archived_reason,
                          created_at, updated_at
                   FROM execution_profiles
                   WHERE profile_id = ? AND active = 1 AND archived_at IS NULL
                   ORDER BY rowid"#,
            )
            .bind(profile_id)
            .fetch_all(&self.pool)
            .await?
        };

        rows.into_iter()
            .map(row_to_execution_profile_view)
            .collect()
    }

    pub(crate) async fn get_version(
        &self,
        profile_id: &str,
        version: &str,
    ) -> Result<Option<ExecutionProfileView>> {
        let row = sqlx::query(
            r#"SELECT profile_id, version, name, description, supported_client_types, agent_kind,
                      system_prompt_template, turn_prompt_template, default_session_role,
                      default_session_description, handle_prefix,
                      expected_output_schema, artifact_contract, default_execution_policy,
                      default_review_policy, metadata, active, archived_at, archived_reason,
                      created_at, updated_at
               FROM execution_profiles
               WHERE profile_id = ? AND version = ?"#,
        )
        .bind(profile_id)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_execution_profile_view).transpose()
    }

    pub(super) async fn profile_exists(&self, profile_id: &str) -> Result<bool> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM execution_profiles WHERE profile_id = ?)",
        )
        .bind(profile_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(exists != 0)
    }
}
