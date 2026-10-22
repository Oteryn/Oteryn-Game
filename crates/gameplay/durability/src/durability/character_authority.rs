//! Canonical first-slice Character owner/world authority.
mod bootstrap_character;
mod creation_integrity;

use oteryn_gameplay_characters::character_authority_audit::{EVENT_SCHEMA_REVISION, EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED, RETENTION_PROFILE, encode_bootstrap};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::{
    CharacterRecoveryFenceV1, CharacterRecoveryTransition, SealedCharacterRecoveryFence,
    recovery_record_digest,
};
use crate::admission_evidence::{Facts, Request, Response, decode_response};
use crate::character_bootstrap_intent::{
    CharacterBootstrapIntentV1, CharacterInterpretationV1,
};
use crate::domain::{AccountId, CharacterId, CharacterRevision, WorldId};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterAuthorityError>;
const MAX_CONTEXT_BYTES: usize = 128;
const MAX_OPERATION_BINDING_BYTES: usize = 1024;
/// Current S1/S2 Platform account-security evidence consumed as an independent
/// bootstrap prerequisite: the fresh-admission account observation.
const ACCOUNT_SECURITY_OPERATION: &str = "ReadAccountSecurityV1";
const ACCOUNT_SECURITY_PURPOSE: &str = "platform_security";
const ACCOUNT_SECURITY_SCOPE: &str = "fresh_admission";
/// Existing FND-04 fresh security freshness window (seconds, less the source's
/// declared clock uncertainty); see `fnd04_verifier::fresh_source_deadline`.
const ACCOUNT_SECURITY_FRESH_SECONDS: i64 = 5;
const MAX_AUDIT_BATCH: u16 = 64;
/// Bounded identity of this server build, recorded with every audit event so a
/// redelivery after an upgrade keeps the originating EventEnvelope value.
pub const SERVER_BUILD_ID: &str = match option_env!("OTERYN_SERVER_BUILD_ID") {
    Some(build) => build,
    None => concat!("oteryn-game-server/", env!("CARGO_PKG_VERSION")),
};

#[derive(Debug)]
pub enum CharacterAuthorityError {
    Rejected,
    Conflict,
    /// The requested name's comparison key is already reserved (CHAR-NAME-1).
    NameUnavailable,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for CharacterAuthorityError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

/// Authority capability issued only after the sealed recovery fence matched the
/// database and the complete Character integrity check passed in this process.
pub struct ReconciledCharacterAuthority<'f, 's> {
    record: CharacterRecoveryFenceV1,
    root_identity: usize,
    root_liveness: std::sync::Weak<dyn std::any::Any + Send + Sync>,
    _seal: &'f SealedCharacterRecoveryFence<'s>,
}

impl ReconciledCharacterAuthority<'_, '_> {
    /// The capability is valid only on the exact root whose database was checked.
    pub fn record_for(&self, root: &DurabilityRoot) -> Result<CharacterRecoveryFenceV1> {
        if self.root_identity != root.root_identity() || self.root_liveness.strong_count() == 0 {
            return Err(CharacterAuthorityError::Rejected);
        }
        Ok(self.record.clone())
    }
}

fn uuid_v7_shape(value: &[u8; 16]) -> bool {
    value[6] >> 4 == 7 && value[8] & 0xc0 == 0x80
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterAuthorityRecord {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub revision: CharacterRevision,
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    /// Registered audit payload bytes. They are the deterministic encoding of
    /// the retained authority identities, pinned byte for byte for schema
    /// revision 1 by `registered_payload_bytes_are_pinned_for_schema_revision_one`.
    /// An exact retry therefore returns the same bytes even after ordinary
    /// expiry deleted the event, without retaining the payload past P90D.
    pub payload: Vec<u8>,
}

/// One committed audit event awaiting at-least-once publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterAuditDelivery {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at: i64,
    /// Originating server build for the EventEnvelope, captured at commit.
    pub server_build_id: String,
    pub payload: Vec<u8>,
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_CONTEXT_BYTES
}

impl DurabilityRoot {
    /// Reconcile an uncertain bootstrap outcome by its operation identity alone,
    /// without re-authorizing: the committed result, or `None` if nothing
    /// committed. An expired intent can therefore still be reconciled.
    pub async fn reconcile_character_bootstrap(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        operation_id: [u8; 16],
    ) -> Result<Option<CharacterAuthorityRecord>> {
        if !uuid_v7_shape(&operation_id) {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT o.account_id::text, o.world_id::text, o.character_id::text, o.event_id::text, o.transaction_id::text FROM game_character_operation_receipts o WHERE o.operation_id = encode($1, 'hex')::uuid")
                .bind(operation_id.as_slice()).fetch_optional(&mut *tx).await?;
            let record = match row {
                Some(row) => Some(read_record_row(&row, uuid_text(row.try_get("account_id")?)?, uuid_text(row.try_get("world_id")?)?)?),
                None => None,
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(record))
        })).await?
    }

    pub async fn read_current_character(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<CharacterAuthorityRecord> {
        let character = *character_id.as_bytes();
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT r.account_id::text, r.character_id::text, r.world_id::text, r.character_revision::text, o.event_id::text, o.transaction_id::text FROM game_character_roots r JOIN game_character_creation_receipts o USING (character_id) WHERE r.character_id = encode($1,'hex')::uuid AND r.lifecycle = 1").bind(character.as_slice()).fetch_optional(&mut *tx).await?;
            let Some(row) = row else { return Ok(Err(CharacterAuthorityError::Rejected)); };
            let record = decode_current_row(&row)?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(record))
        })).await?
    }

    /// The interpretation stored on the active Character root `character_id`, under the
    /// reconciled recovery fence: its profile, ruleset, content and starter template revisions.
    /// A progression binding is built from the root's own revisions, never from the newest
    /// global interpretation, which may have advanced since the Character was created
    /// (ARCH-PROGRESSION-SOURCE-0 §1.3-§1.5). `Ok(None)` when the root's interpretation is not
    /// the newest global interpretation (or none is configured): the gameplay fence of every
    /// durable progression write refuses such a root with `ProgressionContextMismatch`
    /// (§0.2), so it can hold no binding. `Rejected` when no active root exists. Read only.
    pub async fn read_character_root_interpretation(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<Option<CharacterInterpretationV1>> {
        let character = *character_id.as_bytes();
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT profile_revision, ruleset_revision, content_revision, \
                                starter_template_revision \
                           FROM game_character_roots \
                          WHERE character_id = encode($1,'hex')::uuid AND lifecycle = 1",
                    )
                    .bind(character.as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(row) = row else {
                        return Ok(Err(CharacterAuthorityError::Rejected));
                    };
                    let text = |column: &str| -> std::result::Result<String, DurabilityError> {
                        row.try_get(column)
                            .map_err(|_| DurabilityError::InvalidStoredState)
                    };
                    let interpretation = CharacterInterpretationV1::new(
                        &text("profile_revision")?,
                        &text("ruleset_revision")?,
                        &text("content_revision")?,
                        &text("starter_template_revision")?,
                    )
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                    // The same comparison as the gameplay fence (`assert_gameplay_fence`): all
                    // four revisions against the newest global interpretation.
                    let current = current_interpretation(&mut tx).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(current
                        .is_some_and(|(_, current)| current == interpretation)
                        .then_some(interpretation)))
                })
            })
            .await?
    }

    /// Issue the authority capability for a sealed recovery fence: the database's
    /// latest admission must equal the fence and the full integrity check must pass.
    pub async fn open_character_authority<'f, 's>(
        &self,
        seal: &'f SealedCharacterRecoveryFence<'s>,
    ) -> Result<ReconciledCharacterAuthority<'f, 's>> {
        self.open_character_authority_with_proficiency_definitions(seal, None)
            .await
    }
    /// Same sealed recovery authority; the optional source resolves historical maps only.
    pub async fn open_character_authority_with_proficiency_definitions<'f, 's>(
        &self,
        seal: &'f SealedCharacterRecoveryFence<'s>,
        definitions: Option<
            std::sync::Arc<dyn super::character_proficiency::ProficiencyDefinitions>,
        >,
    ) -> Result<ReconciledCharacterAuthority<'f, 's>> {
        let record = seal.record().clone();
        let checked = record.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &checked).await?;
                    verify_character_integrity(&mut tx, definitions.as_deref()).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok::<(), CharacterAuthorityError>(()))
                })
            })
            .await??;
        Ok(ReconciledCharacterAuthority {
            record,
            root_identity: self.root_identity(),
            root_liveness: self.root_liveness(),
            _seal: seal,
        })
    }

    /// Admit a fresh store only after Operations has explicitly created generation one.
    pub async fn admit_fresh_character_recovery(
        &self,
        recovery: &CharacterRecoveryTransition<'_>,
    ) -> Result<()> {
        if recovery.record().recovery_generation != 1
            || recovery.record().predecessor_generation != 0
        {
            return Err(CharacterAuthorityError::Rejected);
        }
        let record = recovery.record().clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            // The definer admits generation one only into an empty Character
            // store; any surviving Character row is evidence of prior state,
            // except exactly this admission re-run after a lost acknowledgement
            // (migration 0007).
            let admitted: bool = sqlx::query_scalar("SELECT game_character_admit_fresh_recovery($1, encode($2,'hex')::uuid, $3::numeric, $4)")
                .bind(&record.authority_scope_id)
                .bind(record.recovery_event_id.as_slice())
                .bind(record.issued_at.to_string())
                .bind(&record.issuer_identity)
                .fetch_one(&mut *tx).await?;
            if !admitted {
                return Ok(Err(CharacterAuthorityError::Conflict));
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }

    /// Control-plane: configure the current Character interpretation. It is
    /// refused until the fresh generation-one recovery store is admitted,
    /// because that admission requires empty Character tables (OPS-NODE-BOOT-01
    /// D2). Returns the current interpretation revision.
    pub async fn configure_character_interpretation(
        &self,
        profile: &str,
        ruleset: &str,
        content: &str,
        starter: &str,
    ) -> Result<i64> {
        let values = [profile, ruleset, content, starter].map(str::to_owned);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let admitted: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_character_recovery_admissions)",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    if !admitted {
                        return Ok(Err(CharacterAuthorityError::Rejected));
                    }
                    let revision: i64 = sqlx::query_scalar(
                        "SELECT game_character_configure_interpretation($1, $2, $3, $4)",
                    )
                    .bind(&values[0])
                    .bind(&values[1])
                    .bind(&values[2])
                    .bind(&values[3])
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(revision))
                })
            })
            .await?
    }

    /// Reconcile a strict external successor while the exclusive generation guard is held.
    pub async fn reconcile_character_recovery(
        &self,
        recovery: &CharacterRecoveryTransition<'_>,
    ) -> Result<()> {
        self.reconcile_character_recovery_with_proficiency_definitions(recovery, None)
            .await
    }
    /// Same exclusive successor proof and admission transaction, with retained map verification.
    pub async fn reconcile_character_recovery_with_proficiency_definitions(
        &self,
        recovery: &CharacterRecoveryTransition<'_>,
        definitions: Option<
            std::sync::Arc<dyn super::character_proficiency::ProficiencyDefinitions>,
        >,
    ) -> Result<()> {
        let record = recovery.record().clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let current: Option<String> = sqlx::query_scalar("SELECT recovery_generation::text FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR UPDATE")
                .fetch_optional(&mut *tx).await?;
            let current = current.ok_or(DurabilityError::Unavailable)?.parse::<u64>().map_err(|_| DurabilityError::Unavailable)?;
            if current == record.recovery_generation {
                assert_recovery_fence(&mut tx, &record).await?;
            } else if current == record.predecessor_generation {
                assert_predecessor_admission(&mut tx, &record).await?;
                insert_recovery_admission(&mut tx, &record).await?;
            } else {
                return Ok(Err(CharacterAuthorityError::Conflict));
            }
            // This is the explicit reconciliation point. Domain-specific integrity checks grow
            // here; restored receipts/audit/outbox never replace the external predecessor proof.
            verify_character_integrity(&mut tx, definitions.as_deref()).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }

    /// Committed audit events not yet acknowledged, oldest first. Publication is
    /// at-least-once: an event stays pending until its exact acknowledgement.
    pub async fn pending_character_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        limit: u16,
    ) -> Result<Vec<CharacterAuditDelivery>> {
        if limit == 0 || limit > MAX_AUDIT_BATCH {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let rows = sqlx::query("SELECT event_id::text, transaction_id::text, occurred_at, server_build_id, payload FROM game_character_audit_outbox WHERE publication_state = 1 ORDER BY occurred_at, event_id LIMIT $1")
                .bind(i64::from(limit)).fetch_all(&mut *tx).await?;
            let mut deliveries = Vec::with_capacity(rows.len());
            for row in &rows {
                deliveries.push(CharacterAuditDelivery {
                    event_id: uuid_text(row.try_get("event_id")?)?,
                    transaction_id: uuid_text(row.try_get("transaction_id")?)?,
                    occurred_at: row.try_get("occurred_at")?,
                    server_build_id: row.try_get("server_build_id")?,
                    payload: row.try_get("payload")?,
                });
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(deliveries))
        })).await?
    }

    /// Record the exact published event. Replaying an acknowledgement is a no-op.
    pub async fn acknowledge_character_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        event_id: [u8; 16],
    ) -> Result<()> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let state: Option<i16> = sqlx::query_scalar("SELECT publication_state FROM game_character_audit_outbox WHERE event_id = encode($1,'hex')::uuid FOR UPDATE")
                .bind(event_id.as_slice()).fetch_optional(&mut *tx).await?;
            match state {
                None => return Ok(Err(CharacterAuthorityError::Rejected)),
                Some(1) => {
                    sqlx::query("UPDATE game_character_audit_outbox SET publication_state = 2, published_at = greatest(occurred_at, floor(extract(epoch FROM statement_timestamp())*1000)::bigint) WHERE event_id = encode($1,'hex')::uuid")
                        .bind(event_id.as_slice()).execute(&mut *tx).await?;
                }
                Some(_) => {}
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }

    /// Ordinary expiry: delete unheld events (envelope and payload) past the
    /// registered P90D ceiling. No pseudonymous copy is retained.
    pub async fn expire_character_audit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        batch: u16,
    ) -> Result<u64> {
        if batch == 0 || batch > MAX_AUDIT_BATCH {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    // The definer boundary serializes with holds on the retention lock
                    // and deletes only unheld records past expiry (migration 0006).
                    let deleted: i64 = sqlx::query_scalar("SELECT game_character_expire_audit($1)")
                        .bind(i32::from(batch))
                        .fetch_one(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(
                        u64::try_from(deleted).map_err(|_| DurabilityError::InvalidStoredState)?
                    ))
                })
            })
            .await?
    }
}

async fn insert_recovery_admission(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &CharacterRecoveryFenceV1,
) -> std::result::Result<(), DurabilityError> {
    let predecessor_digest = predecessor_evidence(record);
    sqlx::query("INSERT INTO game_character_recovery_admissions(authority_scope_id, recovery_generation, recovery_event_id, predecessor_generation, predecessor_digest, issued_at, issuer_identity, reconciled_at) VALUES ($1, $2::numeric, encode($3,'hex')::uuid, $4::numeric, $7, $5::numeric, $6, floor(extract(epoch FROM statement_timestamp())*1000)::bigint)")
        .bind(&record.authority_scope_id)
        .bind(record.recovery_generation.to_string())
        .bind(record.recovery_event_id.as_slice())
        .bind(record.predecessor_generation.to_string())
        .bind(record.issued_at.to_string())
        .bind(&record.issuer_identity)
        .bind(predecessor_digest)
        .execute(&mut **tx).await?;
    Ok(())
}

/// Compare the latest admission with `record`, holding it FOR SHARE.
pub(super) async fn assert_recovery_fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &CharacterRecoveryFenceV1,
) -> std::result::Result<(), DurabilityError> {
    let sql = "SELECT authority_scope_id, recovery_generation::text, recovery_event_id::text, predecessor_generation::text, predecessor_digest, issued_at::text, issuer_identity FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR SHARE";
    let row = sqlx::query(sql)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(DurabilityError::Unavailable)?;
    let generation = row
        .try_get::<String, _>("recovery_generation")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::Unavailable)?;
    let predecessor = row
        .try_get::<String, _>("predecessor_generation")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::Unavailable)?;
    let issued_at = row
        .try_get::<String, _>("issued_at")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::Unavailable)?;
    if row.try_get::<String, _>("authority_scope_id")? != record.authority_scope_id
        || generation != record.recovery_generation
        || uuid_text(row.try_get("recovery_event_id")?)? != record.recovery_event_id
        || predecessor != record.predecessor_generation
        || issued_at != record.issued_at
        || row.try_get::<String, _>("issuer_identity")? != record.issuer_identity
        || stored_predecessor_evidence(&row)? != predecessor_evidence(record)
    {
        return Err(DurabilityError::Unavailable);
    }
    Ok(())
}

/// Predecessor evidence as stored: absent for generation one.
fn predecessor_evidence(record: &CharacterRecoveryFenceV1) -> Option<Vec<u8>> {
    (record.predecessor_generation != 0).then(|| record.predecessor_digest.to_vec())
}

fn stored_predecessor_evidence(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<Option<Vec<u8>>, DurabilityError> {
    Ok(row.try_get("predecessor_digest")?)
}

pub fn uuid_text(value: &str) -> std::result::Result<[u8; 16], DurabilityError> {
    let hex: String = value.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        return Err(DurabilityError::Unavailable);
    }
    let mut out = [0; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| DurabilityError::Unavailable)?;
    }
    Ok(out)
}
/// In-transaction current read for a composition that already holds the
/// reconciled capability: asserts the sealed recovery fence in the caller's
/// transaction, then returns the active Character (lifecycle 1), if any.
pub(super) async fn load_current_character(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    character_id: [u8; 16],
) -> std::result::Result<Option<CharacterAuthorityRecord>, DurabilityError> {
    assert_recovery_fence(tx, recovery).await?;
    let row = sqlx::query("SELECT r.account_id::text, r.character_id::text, r.world_id::text, r.character_revision::text, o.event_id::text, o.transaction_id::text FROM game_character_roots r JOIN game_character_creation_receipts o USING (character_id) WHERE r.character_id = encode($1,'hex')::uuid AND r.lifecycle = 1 FOR SHARE OF r")
        .bind(character_id.as_slice())
        .fetch_optional(&mut **tx)
        .await?;
    row.as_ref().map(decode_current_row).transpose()
}

fn decode_current_row(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CharacterAuthorityRecord, DurabilityError> {
    let account = uuid_text(row.try_get("account_id")?)?;
    let character = uuid_text(row.try_get("character_id")?)?;
    let world = uuid_text(row.try_get("world_id")?)?;
    let revision: String = row.try_get("character_revision")?;
    let revision = revision
        .parse::<u64>()
        .map_err(|_| DurabilityError::Unavailable)?;
    Ok(CharacterAuthorityRecord {
        account_id: AccountId::from_bytes(account).map_err(|_| DurabilityError::Unavailable)?,
        character_id: CharacterId::from_bytes(character)
            .map_err(|_| DurabilityError::Unavailable)?,
        world_id: WorldId::from_bytes(world).map_err(|_| DurabilityError::Unavailable)?,
        revision: CharacterRevision::new(revision).map_err(|_| DurabilityError::Unavailable)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        payload: encode_bootstrap(&account, &character, &world),
    })
}
fn read_record_row(
    row: &sqlx::postgres::PgRow,
    account: [u8; 16],
    world: [u8; 16],
) -> std::result::Result<CharacterAuthorityRecord, DurabilityError> {
    let character = uuid_text(row.try_get("character_id")?)?;
    Ok(CharacterAuthorityRecord {
        account_id: AccountId::from_bytes(account).map_err(|_| DurabilityError::Unavailable)?,
        character_id: CharacterId::from_bytes(character)
            .map_err(|_| DurabilityError::Unavailable)?,
        world_id: WorldId::from_bytes(world).map_err(|_| DurabilityError::Unavailable)?,
        revision: CharacterRevision::new(1).map_err(|_| DurabilityError::Unavailable)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        payload: encode_bootstrap(&account, &character, &world),
    })
}

/// Current (highest-revision) Game-owned Character interpretation, if any.
pub(super) async fn current_interpretation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> std::result::Result<Option<(i64, CharacterInterpretationV1)>, DurabilityError> {
    let Some(row) = sqlx::query(
        "SELECT interpretation_revision, profile_revision, ruleset_revision, content_revision, starter_template_revision \
           FROM game_character_interpretations ORDER BY interpretation_revision DESC LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };
    let text = |column: &str| -> std::result::Result<String, DurabilityError> {
        row.try_get(column)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let interpretation = CharacterInterpretationV1::new(
        &text("profile_revision")?,
        &text("ruleset_revision")?,
        &text("content_revision")?,
        &text("starter_template_revision")?,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let revision: i64 = row
        .try_get("interpretation_revision")
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Some((revision, interpretation)))
}

fn uuid_string(value: &[u8; 16]) -> String {
    let hex: String = value.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Current allowed S1/S2 Platform account-security evidence for this exact
/// AccountId. The S2 floor retains the exact authenticated S1 body, which is
/// re-decoded against the registered source authority and the exact request
/// binding; it must allow the account inside the existing fresh window at
/// trusted Game time. The floor is share-locked, so a concurrent S2 advance
/// serializes with this commit. Absent, other-purpose or stale evidence is
/// not current evidence and never a fallback authorization.
async fn current_account_security_allows(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account: &[u8; 16],
    now: i64,
) -> std::result::Result<bool, DurabilityError> {
    Ok(account_security(tx, account, now).await? == Security::Allowed)
}

/// Current account-security evidence for one AccountId at trusted Game time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Security {
    /// Current evidence allows the account.
    Allowed,
    /// Current evidence refuses the account: a definite answer.
    Refused,
    /// No current evidence (absent, other-purpose or stale): not an answer.
    Unknown,
}

/// [`current_account_security_allows`] with the definite refusal kept apart from
/// missing or stale evidence, for deciders that store a definite answer.
pub(super) async fn account_security(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account: &[u8; 16],
    now: i64,
) -> std::result::Result<Security, DurabilityError> {
    let account_id = uuid_string(account);
    let Some(registered) = sqlx::query_scalar::<_, String>(
        "SELECT source_authority FROM game_durability_native_source_registration WHERE registration_id = 1 FOR SHARE",
    )
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(Security::Unknown);
    };
    let Some(row) = sqlx::query(
        "SELECT operation, source_revision::text, decision_identity, observed_at, semantic_facts \
           FROM game_durability_native_source_floors \
          WHERE registration_id = 1 AND source_authority = $1 AND floor_subject = $2 FOR SHARE",
    )
    .bind(&registered)
    .bind(format!("account:{account_id}"))
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(Security::Unknown);
    };
    let invalid = |_| DurabilityError::InvalidStoredState;
    if row.try_get::<String, _>(0).map_err(invalid)? != ACCOUNT_SECURITY_OPERATION {
        return Ok(Security::Unknown);
    }
    let revision: String = row.try_get(1).map_err(invalid)?;
    let decision: String = row.try_get(2).map_err(invalid)?;
    let observed_at: i64 = row.try_get(3).map_err(invalid)?;
    let body: Vec<u8> = row.try_get(4).map_err(invalid)?;
    let request = Request::Account {
        recovery: false,
        account_id: &account_id,
        purpose: ACCOUNT_SECURITY_PURPOSE,
        scope: ACCOUNT_SECURITY_SCOPE,
    };
    let Ok(Response::Observed(observation)) = decode_response(&request, &registered, &body) else {
        return Ok(Security::Unknown);
    };
    if observation.source_revision.to_string() != revision
        || observation.decision_identity.as_str() != decision
        || observation.source_observed_at != observed_at
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let Facts::Account { allowed, .. } = observation.facts else {
        return Ok(Security::Unknown);
    };
    let deadline = i64::try_from(observation.clock_uncertainty_seconds)
        .ok()
        .and_then(|uncertainty| {
            observed_at
                .checked_add(ACCOUNT_SECURITY_FRESH_SECONDS)?
                .checked_sub(uncertainty)
        });
    if observed_at > now || deadline.is_none_or(|deadline| now > deadline) {
        return Ok(Security::Unknown);
    }
    Ok(if allowed {
        Security::Allowed
    } else {
        Security::Refused
    })
}

/// Complete first-slice integrity: each root has exactly one creation receipt, from the
/// operator bootstrap with its reconstructed intent binding or from a committed create
/// command (migration 0087), and no operation identity is in both receipt tables; every retained audit event binds to that
/// receipt and root with an intact payload hash. Expired events may be absent.
/// The retained intent source high-water exists exactly when a receipt exists
/// and equals the newest receipt's decision, so a restore can neither drop nor
/// regress it (absence is never initialized to zero).
async fn verify_character_integrity(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    definitions: Option<&dyn super::character_proficiency::ProficiencyDefinitions>,
) -> std::result::Result<(), DurabilityError> {
    creation_integrity::verify(tx).await?;
    // P03 progression is optional at revision one because initialization is a
    // separate owner.  Once a Character advances, typed state and a complete,
    // gap-free immutable receipt chain must explain the global revision and the
    // current state.  The chain has one receipt per revision of any kind: XP
    // award (0009), death (0016), stance (0017), Bestiary kill (0019), charm
    // command (0020), monk state save (0026), build change (0030), proficiency
    // (0032), familiar state change (0033) or quest transition (0056), each
    // `before` equal to its predecessor's `after` across kinds.  The bootstrap
    // receipt remains bound to initial revision 1.
    sqlx::query(
        "WITH chain AS ( \
           SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_xp_receipts \
           UNION ALL SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_death_receipts \
           UNION ALL SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_stance_receipts \
           UNION ALL SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_bestiary_kill_receipts \
           UNION ALL SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_charm_receipts \
           UNION ALL SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_monk_state_receipts \
           UNION ALL SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_build_receipts \
           UNION ALL SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_proficiency_receipts \
           UNION ALL SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_familiar_receipts \
           UNION ALL SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_quest_receipts \
           UNION ALL SELECT character_id, original_character_revision, committed_character_revision, \
                  level_before, level_after, experience_before, experience_after, \
                  profile_revision, ruleset_revision, content_revision, simulation_revision, \
                  evidence_revision, declaration_revision, policy_revision, reward_revision \
              FROM game_character_appearance_receipts) \
         SELECT 1 FROM game_character_roots r \
           LEFT JOIN game_character_progression_state s USING (character_id) \
          WHERE (r.character_revision <> 1 AND s.character_id IS NULL) \
             OR (s.character_id IS NOT NULL AND s.character_revision <> r.character_revision) \
         UNION ALL \
         SELECT 1 FROM game_character_progression_state s \
           LEFT JOIN game_character_roots r USING (character_id) \
          WHERE r.character_id IS NULL \
         UNION ALL \
         SELECT 1 FROM game_character_roots r \
           LEFT JOIN chain x USING (character_id) \
          GROUP BY r.character_id, r.character_revision \
         HAVING count(x.committed_character_revision)::numeric <> r.character_revision - 1 \
             OR count(DISTINCT x.committed_character_revision)::numeric \
                <> r.character_revision - 1 \
         UNION ALL \
         SELECT 1 FROM chain x \
           LEFT JOIN chain p \
             ON p.character_id = x.character_id \
            AND p.committed_character_revision = x.original_character_revision \
          WHERE (x.original_character_revision <> 1 AND p.committed_character_revision IS NULL) \
             OR (p.committed_character_revision IS NOT NULL AND \
                 (p.experience_after <> x.experience_before OR p.level_after <> x.level_before)) \
         UNION ALL \
         SELECT 1 FROM game_character_progression_state s \
           JOIN game_character_roots r USING (character_id) \
           LEFT JOIN chain x \
             ON x.character_id = s.character_id \
            AND x.committed_character_revision = s.character_revision \
          WHERE s.character_revision > 1 AND (x.committed_character_revision IS NULL \
             OR x.experience_after <> s.total_experience OR x.level_after <> s.level \
             OR x.profile_revision <> s.profile_revision \
             OR x.ruleset_revision <> s.ruleset_revision \
             OR x.content_revision <> s.content_revision \
             OR x.simulation_revision <> s.simulation_revision \
             OR x.evidence_revision <> s.evidence_revision \
             OR x.declaration_revision <> s.declaration_revision \
             OR x.policy_revision <> s.policy_revision \
             OR x.reward_revision <> s.reward_revision) \
         UNION ALL \
         SELECT 1 FROM chain x \
           LEFT JOIN game_character_progression_state s USING (character_id) \
          WHERE s.character_id IS NULL \
             OR x.committed_character_revision > s.character_revision \
             OR x.profile_revision <> s.profile_revision \
             OR x.ruleset_revision <> s.ruleset_revision \
             OR x.content_revision <> s.content_revision \
             OR x.simulation_revision <> s.simulation_revision \
             OR x.evidence_revision <> s.evidence_revision \
             OR x.declaration_revision <> s.declaration_revision \
             OR x.policy_revision <> s.policy_revision \
             OR x.reward_revision <> s.reward_revision \
         LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await?
    .map_or(Ok(()), |_| Err(DurabilityError::Unavailable))?;
    super::character_appearance::verify_character_appearance_integrity(tx)
        .await
        .map_err(|error| match error {
            super::character_appearance::AppearanceReadError::InvalidStoredState => {
                DurabilityError::InvalidStoredState
            }
            super::character_appearance::AppearanceReadError::Storage(error) => error.into(),
        })?;
    verify_character_build_chain(tx).await?;
    verify_all_character_proficiency_histories(tx, definitions).await?;
    super::character_familiar::verify_familiar_chain(tx).await?;
    super::spell_familiar_group::verify_familiar_group(tx).await?;
    super::spell_item_transaction::verify_spell_item_chain(tx).await?;
    // A self-consistent hash is not semantic evidence: every retained payload
    // must be exactly the registered encoding of its root's identities.
    let mut after = String::from("00000000-0000-0000-0000-000000000000");
    loop {
        let rows = sqlx::query(
            "SELECT a.event_id::text, a.payload, r.account_id::text, r.character_id::text, r.world_id::text \
               FROM game_character_audit_outbox a JOIN game_character_roots r USING (character_id) \
              WHERE a.event_id > $1::uuid ORDER BY a.event_id LIMIT 256",
        )
        .bind(&after)
        .fetch_all(&mut **tx)
        .await?;
        let Some(last) = rows.last() else {
            return Ok(());
        };
        after = last.try_get("event_id")?;
        for row in &rows {
            // Typed identities: UUIDv7 version and RFC variant, not raw bytes.
            let account = AccountId::from_bytes(uuid_text(row.try_get("account_id")?)?)
                .map_err(|_| DurabilityError::Unavailable)?;
            let character = CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
                .map_err(|_| DurabilityError::Unavailable)?;
            let world = WorldId::from_bytes(uuid_text(row.try_get("world_id")?)?)
                .map_err(|_| DurabilityError::Unavailable)?;
            let expected =
                encode_bootstrap(account.as_bytes(), character.as_bytes(), world.as_bytes());
            if row.try_get::<Vec<u8>, _>("payload")? != expected {
                return Err(DurabilityError::Unavailable);
            }
        }
    }
}

// Private complete-gate candidate only. Admission must account for every WP relation,
// including orphan lines/current rows, rather than inferring membership from headers alone.
async fn verify_all_character_proficiency_histories(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    definitions: Option<&dyn super::character_proficiency::ProficiencyDefinitions>,
) -> std::result::Result<(), DurabilityError> {
    let mut after: Option<String> = None;
    loop {
        let rows = sqlx::query(
            "WITH characters AS ( \
               SELECT character_id FROM game_character_proficiency_receipts \
               UNION SELECT character_id FROM game_character_proficiency_receipt_lines \
               UNION SELECT character_id FROM game_character_proficiency \
               UNION SELECT character_id FROM game_character_proficiency_modification_terminals) \
             SELECT character_id::text AS character FROM characters \
              WHERE ($1::uuid IS NULL OR character_id > $1::uuid) ORDER BY character_id LIMIT 256",
        )
        .bind(after.as_deref())
        .fetch_all(&mut **tx)
        .await?;
        let Some(last) = rows.last() else {
            return Ok(());
        };
        after = Some(last.try_get("character")?);
        for row in &rows {
            let character = CharacterId::from_bytes(uuid_text(row.try_get("character")?)?)
                .map_err(|_| DurabilityError::Unavailable)?;
            verify_character_proficiency_history_with_definitions(tx, character, definitions)
                .await
                .map_err(|error| {
                    if matches!(error, DurabilityError::InvalidStoredState) {
                        DurabilityError::Unavailable
                    } else {
                        error
                    }
                })?;
        }
    }
}

/// PROF-1 retained storage/history verifier for the local complete-gate candidate.
/// Checks retained storage/history only. Neither canonical content nor current authority is
/// inferred from receipts. Missing retained migration mappings fail Unavailable in the codec.
pub(super) async fn verify_character_proficiency_history_with_definitions(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    definitions: Option<&dyn super::character_proficiency::ProficiencyDefinitions>,
) -> std::result::Result<Vec<super::character_proficiency::StoredProficiencyTrack>, DurabilityError>
{
    let tracks = verify_character_proficiency_receipt_history(tx, character, definitions).await?;
    // PROFICIENCY-1B §9: modification rows, lines and terminal records.
    super::character_proficiency_modification::verify_modification_history(tx, character).await?;
    Ok(tracks)
}

/// The 0032 receipt, line and track history alone, without 0055's modification tables.
pub(super) async fn verify_character_proficiency_receipt_history(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    definitions: Option<&dyn super::character_proficiency::ProficiencyDefinitions>,
) -> std::result::Result<Vec<super::character_proficiency::StoredProficiencyTrack>, DurabilityError>
{
    let invalid = sqlx::query(
        "SELECT 1 FROM game_character_proficiency_receipts h \
         LEFT JOIN game_character_roots r USING (character_id) \
         WHERE h.character_id=encode($1,'hex')::uuid AND (r.character_id IS NULL \
          OR h.original_character_revision<1 OR h.committed_character_revision<>h.original_character_revision+1 \
          OR h.committed_character_revision>r.character_revision \
          OR (get_byte(uuid_send(h.proficiency_occurrence_id),6)>>4)<>7 \
          OR (get_byte(uuid_send(h.proficiency_occurrence_id),8)&192)<>128 \
          OR h.cause NOT IN ('training','perk_selection','migration','perk_modification') \
          OR octet_length(h.command_binding) NOT BETWEEN 1 AND 1024 OR octet_length(h.policy_digest)<>32 \
          OR h.level_before NOT BETWEEN 1 AND 4294967295 OR h.level_after<>h.level_before \
          OR h.experience_before<0 OR h.experience_after<>h.experience_before OR h.committed_at<0 \
          OR EXISTS (SELECT 1 FROM unnest(ARRAY[h.profile_revision,h.ruleset_revision,h.content_revision, \
              h.simulation_revision,h.evidence_revision,h.declaration_revision,h.policy_revision,h.reward_revision]) v \
              WHERE v IS NULL OR v !~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$') \
          OR (SELECT count(*) FROM game_character_proficiency_receipt_lines l \
              WHERE l.proficiency_occurrence_id=h.proficiency_occurrence_id)=0 \
          OR (h.cause IN ('perk_selection','perk_modification') AND (SELECT count(*) FROM game_character_proficiency_receipt_lines l \
              WHERE l.proficiency_occurrence_id=h.proficiency_occurrence_id)<>1)) \
         UNION ALL SELECT 1 FROM game_character_proficiency_receipt_lines l \
          LEFT JOIN game_character_proficiency_receipts h USING (proficiency_occurrence_id) \
          WHERE (l.character_id=encode($1,'hex')::uuid OR h.character_id=encode($1,'hex')::uuid) \
           AND (h.proficiency_occurrence_id IS NULL OR l.character_id IS DISTINCT FROM h.character_id \
             OR l.committed_character_revision IS DISTINCT FROM h.committed_character_revision \
             OR l.cause IS DISTINCT FROM h.cause) LIMIT 1"
    ).bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    if invalid.is_some() {
        return Err(DurabilityError::InvalidStoredState);
    }
    super::character_proficiency::read::verify_track_history(tx, character, definitions).await
}

/// CHAR-BUILD-1 (A13 §4.2, #1271 F4) build state, checked by name at admission:
/// build-chain continuity (each build receipt, and each death receipt with build
/// fields, starts from its predecessor's `after`, or from the seed `none`, magic
/// level (0, 0) and every skill at (10, 0) for the first), and the build row equal
/// to the latest build-carrying receipt (values, revision, occurrence), absent when
/// there is none. The death receipt's build fields are covered by both checks.
async fn verify_character_build_chain(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "WITH build_chain AS ( \
           SELECT character_id, build_occurrence_id AS occurrence_id, \
                  committed_character_revision, \
                  vocation_before, magic_level_before, mana_spent_before, \
                  fist_level_before, fist_tries_before, club_level_before, \
                  club_tries_before, sword_level_before, sword_tries_before, \
                  axe_level_before, axe_tries_before, distance_level_before, \
                  distance_tries_before, shielding_level_before, \
                  shielding_tries_before, fishing_level_before, fishing_tries_before, \
                  vocation_after, magic_level_after, mana_spent_after, \
                  fist_level_after, fist_tries_after, club_level_after, \
                  club_tries_after, sword_level_after, sword_tries_after, \
                  axe_level_after, axe_tries_after, distance_level_after, \
                  distance_tries_after, shielding_level_after, shielding_tries_after, \
                  fishing_level_after, fishing_tries_after \
             FROM game_character_build_receipts \
           UNION ALL SELECT character_id, death_occurrence_id, committed_character_revision, \
                  vocation, magic_level_before, mana_spent_before, fist_level_before, \
                  fist_tries_before, club_level_before, club_tries_before, \
                  sword_level_before, sword_tries_before, axe_level_before, \
                  axe_tries_before, distance_level_before, distance_tries_before, \
                  shielding_level_before, shielding_tries_before, fishing_level_before, \
                  fishing_tries_before, \
                  vocation, magic_level_after, mana_spent_after, fist_level_after, \
                  fist_tries_after, club_level_after, club_tries_after, \
                  sword_level_after, sword_tries_after, axe_level_after, \
                  axe_tries_after, distance_level_after, distance_tries_after, \
                  shielding_level_after, shielding_tries_after, fishing_level_after, \
                  fishing_tries_after \
             FROM game_character_death_receipts WHERE vocation IS NOT NULL), \
         ordered AS ( \
           SELECT c.*, row_number() OVER w AS position, \
                  count(*) OVER (PARTITION BY c.character_id) AS transitions \
             FROM build_chain c \
           WINDOW w AS (PARTITION BY c.character_id ORDER BY c.committed_character_revision)) \
         SELECT 1 FROM ordered o \
           LEFT JOIN ordered p ON p.character_id = o.character_id \
                              AND p.position = o.position - 1 \
          WHERE (p.position IS NULL AND (o.vocation_before, o.magic_level_before, o.mana_spent_before, \
                  o.fist_level_before, o.fist_tries_before, o.club_level_before, \
                  o.club_tries_before, o.sword_level_before, o.sword_tries_before, \
                  o.axe_level_before, o.axe_tries_before, o.distance_level_before, \
                  o.distance_tries_before, o.shielding_level_before, \
                  o.shielding_tries_before, o.fishing_level_before, \
                  o.fishing_tries_before) \
                 IS DISTINCT FROM ('none', 0, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0)) \
             OR (p.position IS NOT NULL AND (o.vocation_before, o.magic_level_before, o.mana_spent_before, \
                  o.fist_level_before, o.fist_tries_before, o.club_level_before, \
                  o.club_tries_before, o.sword_level_before, o.sword_tries_before, \
                  o.axe_level_before, o.axe_tries_before, o.distance_level_before, \
                  o.distance_tries_before, o.shielding_level_before, \
                  o.shielding_tries_before, o.fishing_level_before, \
                  o.fishing_tries_before) \
                 IS DISTINCT FROM (p.vocation_after, p.magic_level_after, p.mana_spent_after, \
                  p.fist_level_after, p.fist_tries_after, p.club_level_after, \
                  p.club_tries_after, p.sword_level_after, p.sword_tries_after, \
                  p.axe_level_after, p.axe_tries_after, p.distance_level_after, \
                  p.distance_tries_after, p.shielding_level_after, \
                  p.shielding_tries_after, p.fishing_level_after, \
                  p.fishing_tries_after)) \
         UNION ALL \
         SELECT 1 FROM ordered o \
           LEFT JOIN game_character_build_state r USING (character_id) \
          WHERE o.position = o.transitions \
            AND (r.character_id IS NULL \
                 OR r.committed_character_revision <> o.committed_character_revision \
                 OR r.last_build_occurrence_id <> o.occurrence_id \
                 OR (r.vocation, r.magic_level, r.mana_spent, r.fist_level, r.fist_tries, \
                  r.club_level, r.club_tries, r.sword_level, r.sword_tries, \
                  r.axe_level, r.axe_tries, r.distance_level, r.distance_tries, \
                  r.shielding_level, r.shielding_tries, r.fishing_level, \
                  r.fishing_tries) \
                    IS DISTINCT FROM (o.vocation_after, o.magic_level_after, o.mana_spent_after, \
                  o.fist_level_after, o.fist_tries_after, o.club_level_after, \
                  o.club_tries_after, o.sword_level_after, o.sword_tries_after, \
                  o.axe_level_after, o.axe_tries_after, o.distance_level_after, \
                  o.distance_tries_after, o.shielding_level_after, \
                  o.shielding_tries_after, o.fishing_level_after, \
                  o.fishing_tries_after)) \
         UNION ALL \
         SELECT 1 FROM game_character_build_state r \
          WHERE NOT EXISTS (SELECT 1 FROM build_chain c WHERE c.character_id = r.character_id) \
         LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await?
    .map_or(Ok(()), |_| Err(DurabilityError::Unavailable))
}

/// The database's current admission must be exactly the predecessor the external
/// successor retains: the digest covers every field, including the admission's
/// own predecessor digest, so the whole chain is bound.
async fn assert_predecessor_admission(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &CharacterRecoveryFenceV1,
) -> std::result::Result<(), DurabilityError> {
    let row = sqlx::query("SELECT authority_scope_id, recovery_generation::text, recovery_event_id::text, predecessor_generation::text, predecessor_digest, issued_at::text, issuer_identity FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1")
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(DurabilityError::Unavailable)?;
    let number = |name: &str| -> std::result::Result<u64, DurabilityError> {
        row.try_get::<String, _>(name)?
            .parse::<u64>()
            .map_err(|_| DurabilityError::Unavailable)
    };
    let digest: Option<Vec<u8>> = row.try_get("predecessor_digest")?;
    let predecessor = CharacterRecoveryFenceV1 {
        authority_scope_id: row.try_get("authority_scope_id")?,
        recovery_generation: number("recovery_generation")?,
        recovery_event_id: uuid_text(row.try_get("recovery_event_id")?)?,
        predecessor_generation: number("predecessor_generation")?,
        predecessor_digest: match digest {
            None => [0; 32],
            Some(bytes) => bytes.try_into().map_err(|_| DurabilityError::Unavailable)?,
        },
        issued_at: number("issued_at")?,
        issuer_identity: row.try_get("issuer_identity")?,
    };
    if predecessor.recovery_generation != record.predecessor_generation
        || predecessor.authority_scope_id != record.authority_scope_id
        || predecessor.issuer_identity != record.issuer_identity
        || recovery_record_digest(&predecessor).map_err(|_| DurabilityError::Unavailable)?
            != record.predecessor_digest
    {
        return Err(DurabilityError::Unavailable);
    }
    Ok(())
}
