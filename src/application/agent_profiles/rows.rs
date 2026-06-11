use super::*;

pub(super) fn row_to_execution_profile_view(
    row: sqlx::sqlite::SqliteRow,
) -> Result<ExecutionProfileView> {
    Ok(ExecutionProfileView {
        profile_id: row.try_get("profile_id")?,
        version: row.try_get("version")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        supported_client_types: json_field(&row, "supported_client_types")?,
        agent_kind: row.try_get("agent_kind")?,
        system_prompt_template: row.try_get("system_prompt_template")?,
        turn_prompt_template: row.try_get("turn_prompt_template")?,
        default_session_role: row.try_get("default_session_role")?,
        default_session_description: row.try_get("default_session_description")?,
        handle_prefix: row.try_get("handle_prefix")?,
        expected_output_schema: row.try_get("expected_output_schema")?,
        artifact_contract: json_field(&row, "artifact_contract")?,
        default_execution_policy: json_field(&row, "default_execution_policy")?,
        default_review_policy: json_field(&row, "default_review_policy")?,
        metadata: json_field(&row, "metadata")?,
        active: row.try_get::<i64, _>("active")? != 0,
        archived_at: row.try_get("archived_at")?,
        archived_reason: row.try_get("archived_reason")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn json_field<T>(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let raw: String = row.try_get(column)?;
    Ok(serde_json::from_str(&raw)?)
}
