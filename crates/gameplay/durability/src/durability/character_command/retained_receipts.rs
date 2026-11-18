//! Fenced replay of immutable receipts after retirement of their intent feed.
use super::*;
use crate::native_admission_source::character_commands::RetainedCreateReceipt;

impl DurabilityRoot {
    /// Read the next committed create receipt under current recovery and node fences.
    /// Keep the operation identity when validation fails so replay can skip that row.
    pub async fn next_retained_create_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        source: &str,
        after: Option<&str>,
    ) -> Result<Option<RetainedCreateReceipt>, DurabilityError> {
        if !source_authority_valid(source) || after.is_some_and(|id| !uuid_v7(id)) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let proof = proof.clone();
        let source = source.to_owned();
        let after = after.map(str::to_owned);
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                if !prove_current_incarnation(&mut tx, &proof).await? {
                    return Err(DurabilityError::Unavailable);
                }
                let row = sqlx::query("SELECT operation_id::text AS operation,receipt,character_id::text AS character,account_id::text AS account,world_id::text AS world,recovery_generation::text AS generation,decided_at FROM game_character_command_receipts WHERE command='CreateCharacter' AND outcome=1 AND ($1::text IS NULL OR operation_id>$1::uuid) ORDER BY operation_id LIMIT 1")
                    .bind(after).fetch_optional(&mut *tx).await?;
                let result = if let Some(row) = row {
                    let operation: String = row.try_get("operation")?;
                    let bytes: Vec<u8> = row.try_get("receipt")?;
                    let generation: String = row.try_get("generation")?;
                    let decided: i64 = row.try_get("decided_at")?;
                    let expected = [
                        ("operation", RECEIPT_OPERATION.to_owned()),
                        ("source_authority", source.clone()),
                        ("command", CREATE_COMMAND.to_owned()),
                        ("operation_id", operation.clone()),
                        ("outcome", "COMMITTED".to_owned()),
                        ("result_code", OK.to_owned()),
                        ("character_id", row.try_get("character")?),
                        ("account_id", row.try_get("account")?),
                        ("world_id", row.try_get("world")?),
                        ("recovery_generation", generation),
                        ("decided_at", decided.to_string()),
                    ];
                    let verify = || -> Result<(), DurabilityError> {
                        let value: serde_json::Value = serde_json::from_slice(&bytes)
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                        if bytes.len()>MAX_RECEIPT_BYTES || value["contract_version"]!=1
                            || expected.iter().any(|(key, expected)| value[*key].as_str()!=Some(expected.as_str())) {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        let text = |key: &str| value.get(key).and_then(serde_json::Value::as_str).ok_or(DurabilityError::InvalidStoredState);
                        let count = |key: &str| text(key).and_then(|v| counter(v).ok_or(DurabilityError::InvalidStoredState));
                        let uuid = |key: &str| text(key).and_then(|v| uuid_bytes(v).ok_or(DurabilityError::InvalidStoredState));
                        let name = text("name")?;
                        CharacterName::parse(name).map_err(|_| DurabilityError::InvalidStoredState)?;
                        let generation = count("recovery_generation")?;
                        if generation > recovery.recovery_generation || committed_create_receipt(&source, &operation, Committed {
                            character_id: uuid("character_id")?, account_id: uuid("account_id")?, world_id: uuid("world_id")?, name,
                            projection_epoch: count("projection_epoch")?, projection_revision: count("projection_revision")?,
                        }, generation, count("decided_at")?) != bytes {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        Ok(())
                    };
                    Some(RetainedCreateReceipt {
                        body: verify().ok().map(|()| bytes),
                        operation_id: operation,
                    })
                } else { None };
                commit_semantic_transaction(tx,deadline).await?;
                Ok(result)
            })
        }).await
    }
}
