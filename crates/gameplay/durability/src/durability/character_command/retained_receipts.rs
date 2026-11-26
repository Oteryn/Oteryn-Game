//! Fenced v1 create publication after retirement of its intent feed.
use crate::durability::db::commit_semantic_read;
use super::super::db::DB_PASS_DEADLINE;
use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::native_admission_source::SourceError;
use crate::native_admission_source::character_commands::{
    CreateSendability, RetainedCreateKey, RetainedCreatePage, RetainedCreateScan,
};
use sqlx::postgres::PgRow;
use tokio::time::Instant;

const PAGE_SIZE: i64 = 2;

/// One publisher-owned connection slot on the verified durability endpoint.
pub struct RetainedCreatePublisherStore<'a, 'f, 's> {
    pool: &'a sqlx::PgPool,
    root: &'a DurabilityRoot,
    authority: &'a ReconciledCharacterAuthority<'f, 's>,
    proof: &'a NodeIncarnationProof,
    source: &'a str,
}

impl<'a, 'f, 's> RetainedCreatePublisherStore<'a, 'f, 's> {
    pub fn new(
        root: &'a DurabilityRoot,
        authority: &'a ReconciledCharacterAuthority<'f, 's>,
        proof: &'a NodeIncarnationProof,
        source: &'a str,
    ) -> Result<Self, DurabilityError> {
        let pool = root.retained_receipt_publisher_pool()?;
        Ok(Self {
            pool,
            root,
            authority,
            proof,
            source,
        })
    }

    async fn run<T>(
        &self,
        operation: impl for<'c> FnOnce(
            &'c mut sqlx::pool::PoolConnection<sqlx::Postgres>,
            Instant,
            CharacterRecoveryFenceV1,
        ) -> std::pin::Pin<
            Box<dyn Future<Output = Result<T, DurabilityError>> + Send + 'c>,
        > + Send,
    ) -> Result<T, SourceError> {
        let recovery = self
            .authority
            .record_for(self.root)
            .map_err(|_| SourceError::Unavailable)?;
        let deadline = Instant::now() + DB_PASS_DEADLINE;
        let mut holder = self
            .pool
            .acquire()
            .await
            .map_err(|_| SourceError::Unavailable)?;
        let result =
            tokio::time::timeout_at(deadline, operation(&mut holder, deadline, recovery)).await;
        if matches!(
            result,
            Err(_) | Ok(Err(DurabilityError::CommitOutcomeUnknown))
        ) {
            // Never reuse a session that might still be completing a timed-out pass.
            holder.close_on_drop();
        }
        result
            .map_err(|_| SourceError::Unavailable)?
            .map_err(|_| SourceError::Unavailable)
    }
}

impl RetainedCreateScan for RetainedCreatePublisherStore<'_, '_, '_> {
    async fn scan_retained_creates(
        &mut self,
        after: i64,
        through: Option<i64>,
    ) -> Result<RetainedCreatePage, SourceError> {
        let proof = self.proof.clone();
        self.run(move |holder, deadline, recovery| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_publication_fences(&mut tx, &recovery, &proof).await?;
                let through = match through {
                Some(through) => through,
                None => sqlx::query_scalar(
                    "SELECT COALESCE(max(scan_key), 0) FROM game_character_retained_create_scan",
                )
                .fetch_one(&mut *tx).await?,
            };
                let rows = sqlx::query(
                    "SELECT scan_key, operation_id::text AS operation
                 FROM game_character_retained_create_scan
                 WHERE scan_key > $1 AND scan_key <= $2
                 ORDER BY scan_key LIMIT $3",
                )
                .bind(after)
                .bind(through)
                .bind(PAGE_SIZE)
                .fetch_all(&mut *tx)
                .await?;
                let keys = rows
                    .into_iter()
                    .map(|row| {
                        Ok(RetainedCreateKey {
                            scan_key: row.try_get("scan_key")?,
                            operation_id: row.try_get("operation")?,
                        })
                    })
                    .collect::<Result<_, sqlx::Error>>()?;
                commit_semantic_read(tx, deadline).await?;
                Ok(RetainedCreatePage { through, keys })
            })
        })
        .await
    }

    async fn sendable_create(
        &mut self,
        operation_id: &str,
    ) -> Result<CreateSendability, SourceError> {
        let proof = self.proof.clone();
        let source = self.source.to_owned();
        let operation = operation_id.to_owned();
        self.run(move |holder, deadline, recovery| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                let result = sendable(&mut tx, &recovery, &proof, &source, &operation).await?;
                commit_semantic_read(tx, deadline).await?;
                Ok(result)
            })
        })
        .await
    }

    async fn stop_retained_create_receipt(
        &mut self,
        operation_id: &str,
    ) -> Result<(), SourceError> {
        let proof = self.proof.clone();
        let operation = operation_id.to_owned();
        self.run(move |holder, deadline, recovery| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                stop(&mut tx, &recovery, &proof, &operation).await?;
                commit_semantic_transaction(tx, deadline).await
            })
        })
        .await
    }
}

impl DurabilityRoot {
    /// Shared store gate used by the pending consumer on its normal holder.
    pub async fn sendable_create_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        source: &str,
        operation_id: &str,
    ) -> Result<CreateSendability, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let proof = proof.clone();
        let source = source.to_owned();
        let operation = operation_id.to_owned();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = sendable(&mut tx, &recovery, &proof, &source, &operation).await?;
                    commit_semantic_read(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }

    pub async fn stop_retained_create_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        operation_id: &str,
    ) -> Result<(), DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let proof = proof.clone();
        let operation = operation_id.to_owned();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    stop(&mut tx, &recovery, &proof, &operation).await?;
                    commit_semantic_transaction(tx, deadline).await
                })
            })
            .await
    }
}

async fn assert_publication_fences(
    tx: &mut Tx<'_>,
    recovery: &CharacterRecoveryFenceV1,
    proof: &NodeIncarnationProof,
) -> Result<(), DurabilityError> {
    assert_recovery_fence(tx, recovery).await?;
    if !prove_current_incarnation(tx, proof).await? {
        return Err(DurabilityError::Unavailable);
    }
    Ok(())
}

async fn sendable(
    tx: &mut Tx<'_>,
    recovery: &CharacterRecoveryFenceV1,
    proof: &NodeIncarnationProof,
    source: &str,
    operation: &str,
) -> Result<CreateSendability, DurabilityError> {
    if !source_authority_valid(source) || !uuid_v7(operation) {
        return Err(DurabilityError::InvalidStoredState);
    }
    assert_publication_fences(tx, recovery, proof).await?;
    let row = sqlx::query(
        "SELECT receipt, character_id::text AS character, account_id::text AS account,
                world_id::text AS world, recovery_generation::text AS generation, decided_at
         FROM game_character_command_receipts AS r
         WHERE operation_id = $1::uuid AND command = 'CreateCharacter' AND outcome = 1
           AND NOT EXISTS (
               SELECT 1 FROM game_character_retained_create_receipt_stops AS s
               WHERE s.operation_id = r.operation_id
           )",
    )
    .bind(operation)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(match row {
        Some(row) => match verify_receipt(row, source, operation, recovery.recovery_generation) {
            Ok(body) => CreateSendability::Ready(body),
            Err(_) => CreateSendability::Invalid,
        },
        None => CreateSendability::StoppedOrMissing,
    })
}

fn verify_receipt(
    row: PgRow,
    source: &str,
    operation: &str,
    current_generation: u64,
) -> Result<String, DurabilityError> {
    let bytes: Vec<u8> = row.try_get("receipt")?;
    let generation: String = row.try_get("generation")?;
    let decided: i64 = row.try_get("decided_at")?;
    let expected = [
        ("operation", RECEIPT_OPERATION.to_owned()),
        ("source_authority", source.to_owned()),
        ("command", CREATE_COMMAND.to_owned()),
        ("operation_id", operation.to_owned()),
        ("outcome", "COMMITTED".to_owned()),
        ("result_code", OK.to_owned()),
        ("character_id", row.try_get("character")?),
        ("account_id", row.try_get("account")?),
        ("world_id", row.try_get("world")?),
        ("recovery_generation", generation),
        ("decided_at", decided.to_string()),
    ];
    let verify = || -> Result<(), DurabilityError> {
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
        if bytes.len() > MAX_RECEIPT_BYTES
            || value["contract_version"] != 1
            || expected
                .iter()
                .any(|(key, expected)| value[*key].as_str() != Some(expected.as_str()))
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let text = |key: &str| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .ok_or(DurabilityError::InvalidStoredState)
        };
        let count = |key: &str| {
            text(key).and_then(|v| counter(v).ok_or(DurabilityError::InvalidStoredState))
        };
        let uuid = |key: &str| {
            text(key).and_then(|v| uuid_bytes(v).ok_or(DurabilityError::InvalidStoredState))
        };
        let name = text("name")?;
        CharacterName::parse(name).map_err(|_| DurabilityError::InvalidStoredState)?;
        let generation = count("recovery_generation")?;
        if generation > current_generation
            || committed_create_receipt(
                source,
                operation,
                Committed {
                    character_id: uuid("character_id")?,
                    account_id: uuid("account_id")?,
                    world_id: uuid("world_id")?,
                    name,
                    projection_epoch: count("projection_epoch")?,
                    projection_revision: count("projection_revision")?,
                },
                generation,
                count("decided_at")?,
            ) != bytes
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(())
    };
    verify()?;
    String::from_utf8(bytes).map_err(|_| DurabilityError::InvalidStoredState)
}

async fn stop(
    tx: &mut Tx<'_>,
    recovery: &CharacterRecoveryFenceV1,
    proof: &NodeIncarnationProof,
    operation: &str,
) -> Result<(), DurabilityError> {
    if !uuid_v7(operation) {
        return Err(DurabilityError::InvalidStoredState);
    }
    assert_publication_fences(tx, recovery, proof).await?;
    // The immutable source row distinguishes an idempotent stop from a
    // nonexistent or ineligible operation, even with concurrent stop writers.
    let eligible: bool = sqlx::query_scalar(
        "WITH eligible AS (
             SELECT operation_id FROM game_character_command_receipts
             WHERE operation_id = $1::uuid AND command = 'CreateCharacter' AND outcome = 1
         ), disposition AS (
             INSERT INTO game_character_retained_create_receipt_stops (operation_id)
             SELECT operation_id FROM eligible ON CONFLICT (operation_id) DO NOTHING
         )
         SELECT EXISTS (SELECT 1 FROM eligible)",
    )
    .bind(operation)
    .fetch_one(&mut **tx)
    .await?;
    if !eligible {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
