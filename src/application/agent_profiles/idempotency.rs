use super::*;

impl AgentProfileService {
    pub(super) async fn idempotency_response(
        &self,
        operation: &str,
        key: &str,
    ) -> Result<Option<Value>> {
        let row =
            sqlx::query("SELECT response FROM idempotency_keys WHERE operation = ? AND key = ?")
                .bind(operation)
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;

        row.map(|row| {
            let response: String = row.try_get("response")?;
            Ok(serde_json::from_str(&response)?)
        })
        .transpose()
    }

    pub(super) async fn store_idempotency_response(
        &self,
        operation: &str,
        key: &str,
        response: &Value,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO idempotency_keys (operation, key, response)
               VALUES (?, ?, ?)
               ON CONFLICT(operation, key) DO NOTHING"#,
        )
        .bind(operation)
        .bind(key)
        .bind(serde_json::to_string(response)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
