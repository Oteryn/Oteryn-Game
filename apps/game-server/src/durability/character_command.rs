//! Character Authority decision of Platform character command intents
//! (`docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` §5, §6.1, §7).
//!
//! One transaction decides one `operation_id` and stores its receipt, or stores nothing:
//! a stored receipt is republished byte-identical, a deterministic answer is a new stored
//! receipt (migration 0087), and an unavailable dependency is transient. Only
//! `CreateCharacter` / `PLATFORM_USER_CREATE` is decided; any other command is a stored
//! `CHAR_CMD_UNSUPPORTED` receipt. Nothing here logs an identifier or a name (§8).

use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, Security, account_security,
    assert_recovery_fence, current_interpretation, uuid_text,
};
use super::character_authority_audit::{
    EVENT_SCHEMA_REVISION, EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED, RETENTION_PROFILE,
    encode_bootstrap,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::domain::character_name::CharacterName;
use crate::native_admission_source::character_commands::{
    CREATE_COMMAND, Decision, ISSUER_AUTHORITY, Input, command_valid, counter,
    source_authority_valid, uuid_v7,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;

pub const CREATE_VARIANT: &str = "PLATFORM_USER_CREATE";
pub const AUDIENCE: &str = "OTERYN_GAME_CHARACTER_AUTHORITY";
pub const RECEIPT_OPERATION: &str = "PublishCharacterCommandReceiptV1";
/// Intent technical TTL and permitted future clock skew (§4.2, §5.3), seconds.
pub const MAX_TTL_SECONDS: u64 = 300;
pub const MAX_FUTURE_SKEW_SECONDS: u64 = 5;
/// Listed Characters per account, the LCFA wire bound (§6.1).
pub const MAX_ACCOUNT_CHARACTERS: i64 = 64;
/// Receipt bytes bound of migration 0087 and of the receipt request (§12).
pub const MAX_RECEIPT_BYTES: usize = 1024;
const MAX_INTENT_BYTES: usize = 4096;
/// `variant` column value of `PLATFORM_USER_CREATE`.
const VARIANT_CREATE: i16 = 1;
/// Audit expiry of the Character authority event (P90D), milliseconds.
const AUDIT_RETENTION_MS: i64 = 7_776_000_000;

pub const OK: &str = "CHAR_CMD_OK";
pub const NAME_INVALID: &str = "CHAR_CMD_NAME_INVALID";
pub const NAME_UNAVAILABLE: &str = "CHAR_CMD_NAME_UNAVAILABLE";
pub const WORLD_NOT_ACCEPTED: &str = "CHAR_CMD_WORLD_NOT_ACCEPTED";
pub const ACCOUNT_CHARACTER_LIMIT: &str = "CHAR_CMD_ACCOUNT_CHARACTER_LIMIT";
pub const ACCOUNT_NOT_ELIGIBLE: &str = "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE";
pub const INTENT_EXPIRED: &str = "CHAR_CMD_INTENT_EXPIRED";
pub const INTENT_INVALID: &str = "CHAR_CMD_INTENT_INVALID";
pub const OPERATION_CONFLICT: &str = "CHAR_CMD_OPERATION_CONFLICT";
pub const UNSUPPORTED: &str = "CHAR_CMD_UNSUPPORTED";

/// The exact §4.2 `CreateCharacter` intent, in wire member order.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CreateWire {
    contract_version: u8,
    command: String,
    variant: String,
    issuer_authority: String,
    issuer_decision_id: String,
    source_revision: String,
    operation_id: String,
    account_id: String,
    target_world_id: String,
    requested_name: String,
    issued_at_source: String,
    expires_at_source: String,
    audience: String,
}

/// The members that classify an intent before its strict decode.
#[derive(Deserialize)]
struct Head {
    contract_version: Option<serde_json::Value>,
    command: Option<serde_json::Value>,
    variant: Option<serde_json::Value>,
}

/// A decoded `PLATFORM_USER_CREATE` intent. The name is checked by the decision (§7
/// `CHAR_CMD_NAME_INVALID`), not by the decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateIntent {
    pub source_revision: u64,
    pub account_id: [u8; 16],
    pub target_world_id: [u8; 16],
    pub requested_name: String,
    pub issued_at_source: u64,
    pub expires_at_source: u64,
}

/// What a read intent is, before any state is consulted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classified {
    Create(CreateIntent),
    /// A definite rejection decided from the bytes alone.
    Rejected(&'static str),
}

/// Classify the intent read for `(command, operation_id)` (§4.2, §5.1, §7): another
/// `command` is an operation conflict, another version or variant is unsupported, and
/// anything not exactly the §4.2 wire is invalid.
#[must_use]
pub fn classify_intent(raw: &[u8], command: &str, operation_id: &str) -> Classified {
    use Classified::Rejected;
    if raw.len() > MAX_INTENT_BYTES {
        return Rejected(INTENT_INVALID);
    }
    let Ok(head) = serde_json::from_slice::<Head>(raw) else {
        return Rejected(INTENT_INVALID);
    };
    match &head.command {
        Some(serde_json::Value::String(stored)) if stored == command => {}
        Some(serde_json::Value::String(_)) => return Rejected(OPERATION_CONFLICT),
        _ => return Rejected(INTENT_INVALID),
    }
    if head.contract_version != Some(serde_json::Value::from(1))
        || head.variant != Some(serde_json::Value::from(CREATE_VARIANT))
        || command != CREATE_COMMAND
    {
        return Rejected(UNSUPPORTED);
    }
    decode_create(raw, operation_id).map_or(Rejected(INTENT_INVALID), Classified::Create)
}

fn decode_create(raw: &[u8], operation_id: &str) -> Option<CreateIntent> {
    let wire: CreateWire = serde_json::from_slice(raw).ok()?;
    // Exact members in wire order, no whitespace and no escapes: the canonical
    // re-encoding is the received body.
    if serde_json::to_vec(&wire).ok()? != raw
        || wire.issuer_authority != ISSUER_AUTHORITY
        || wire.audience != AUDIENCE
        || wire.operation_id != operation_id
        || !uuid_v7(&wire.operation_id)
        || uuid_bytes(&wire.issuer_decision_id).is_none()
        || !uuid_v7(&wire.account_id)
        || !uuid_v7(&wire.target_world_id)
    {
        return None;
    }
    let issued_at_source = decimal(&wire.issued_at_source)?;
    let expires_at_source = decimal(&wire.expires_at_source)?;
    if expires_at_source <= issued_at_source
        || expires_at_source - issued_at_source > MAX_TTL_SECONDS
        || i64::try_from(expires_at_source).is_err()
    {
        return None;
    }
    Some(CreateIntent {
        source_revision: counter(&wire.source_revision)?,
        account_id: uuid_bytes(&wire.account_id)?,
        target_world_id: uuid_bytes(&wire.target_world_id)?,
        requested_name: wire.requested_name,
        issued_at_source,
        expires_at_source,
    })
}

/// Canonical decimal `0|[1-9][0-9]*` within u64.
fn decimal(value: &str) -> Option<u64> {
    if value == "0" {
        return Some(0);
    }
    counter(value)
}

/// Canonical lower-case UUID text of RFC version 1-8, as bytes.
fn uuid_bytes(value: &str) -> Option<[u8; 16]> {
    let b = value.as_bytes();
    if b.len() != 36
        || !matches!(b[14], b'1'..=b'8')
        || !matches!(b[19], b'8' | b'9' | b'a' | b'b')
        || [8, 13, 18, 23].iter().any(|&i| b[i] != b'-')
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 8 | 13 | 18 | 23) || matches!(c, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }
    uuid_text(value).ok()
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

/// Canonical binding of what was decided (§5.2): the complete intent bytes, or the
/// command of an intent that could not be read.
fn binding(tag: u8, data: &[u8]) -> Vec<u8> {
    let mut value = vec![tag];
    value.extend_from_slice(data);
    value
}

#[must_use]
pub fn intent_binding(raw: &[u8]) -> Vec<u8> {
    binding(0x03, &Sha256::digest(raw))
}

#[must_use]
pub fn read_conflict_binding(command: &str) -> Vec<u8> {
    binding(0x04, command.as_bytes())
}

#[must_use]
pub fn unsupported_binding(command: &str) -> Vec<u8> {
    binding(0x05, command.as_bytes())
}

#[derive(Serialize)]
struct RejectedReceipt<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    command: &'a str,
    operation_id: &'a str,
    outcome: &'static str,
    result_code: &'a str,
    recovery_generation: String,
    decided_at: String,
}

#[derive(Serialize)]
struct CommittedCreateReceipt<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    command: &'static str,
    operation_id: &'a str,
    outcome: &'static str,
    result_code: &'static str,
    character_id: String,
    account_id: String,
    world_id: String,
    name: &'a str,
    character_revision: &'static str,
    projection_epoch: String,
    projection_revision: String,
    recovery_generation: String,
    decided_at: String,
}

/// The exact rejected receipt of §4.3.
#[must_use]
pub fn rejected_receipt(
    source_authority: &str,
    command: &str,
    operation_id: &str,
    result_code: &str,
    recovery_generation: u64,
    decided_at: u64,
) -> Vec<u8> {
    serde_json::to_vec(&RejectedReceipt {
        contract_version: 1,
        operation: RECEIPT_OPERATION,
        source_authority,
        command,
        operation_id,
        outcome: "REJECTED",
        result_code,
        recovery_generation: recovery_generation.to_string(),
        decided_at: decided_at.to_string(),
    })
    .unwrap_or_default()
}

/// What a committed create assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Committed<'a> {
    pub character_id: [u8; 16],
    pub account_id: [u8; 16],
    pub world_id: [u8; 16],
    pub name: &'a str,
    pub projection_epoch: u64,
    pub projection_revision: u64,
}

/// The exact committed `CreateCharacter` receipt of §4.3.
#[must_use]
pub fn committed_create_receipt(
    source_authority: &str,
    operation_id: &str,
    committed: Committed<'_>,
    recovery_generation: u64,
    decided_at: u64,
) -> Vec<u8> {
    serde_json::to_vec(&CommittedCreateReceipt {
        contract_version: 1,
        operation: RECEIPT_OPERATION,
        source_authority,
        command: CREATE_COMMAND,
        operation_id,
        outcome: "COMMITTED",
        result_code: OK,
        character_id: uuid_string(&committed.character_id),
        account_id: uuid_string(&committed.account_id),
        world_id: uuid_string(&committed.world_id),
        name: committed.name,
        character_revision: "1",
        projection_epoch: committed.projection_epoch.to_string(),
        projection_revision: committed.projection_revision.to_string(),
        recovery_generation: recovery_generation.to_string(),
        decided_at: decided_at.to_string(),
    })
    .unwrap_or_default()
}

/// The owned decision input carried into the transaction.
enum Owned {
    Intent(Vec<u8>),
    ReadConflict,
    Unsupported,
}

/// One decision's stored outcome, before publication.
struct Stored {
    receipt: Vec<u8>,
    result_code: String,
}

type Tx<'c> = sqlx::Transaction<'c, sqlx::Postgres>;

impl DurabilityRoot {
    /// Decide one character command (§6) under the reconciled recovery fence and the
    /// current process incarnation. A stored receipt for the same binding is returned
    /// unchanged; a stored receipt for another binding, or a creation receipt of the
    /// operator bootstrap holding the `operation_id`, is a conflict with no write.
    pub async fn decide_character_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        source_authority: &str,
        command: &str,
        operation_id: &str,
        input: Input<'_>,
    ) -> Decision {
        let Some(operation) = uuid_v7(operation_id)
            .then(|| uuid_bytes(operation_id))
            .flatten()
        else {
            return Decision::Transient;
        };
        if !command_valid(command) || !source_authority_valid(source_authority) {
            return Decision::Transient;
        }
        let Ok(recovery) = authority.record_for(self) else {
            return Decision::Transient;
        };
        let input = match input {
            Input::Intent(raw) => Owned::Intent(raw.to_vec()),
            Input::ReadConflict => Owned::ReadConflict,
            Input::Unsupported => Owned::Unsupported,
        };
        let proof = proof.clone();
        let source_authority = source_authority.to_owned();
        let command = command.to_owned();
        let operation_text = operation_id.to_owned();
        let Ok(pass) = self.try_issue_semantic_pass() else {
            return Decision::Transient;
        };
        let decided = pass
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    if !prove_current_incarnation(&mut tx, &proof).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    let request = Request {
                        source_authority: &source_authority,
                        command: &command,
                        operation_id: &operation_text,
                        operation,
                        recovery_generation: recovery.recovery_generation,
                    };
                    let decided = decide(&mut tx, &request, &input).await?;
                    if decided.is_some() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(decided)
                })
            })
            .await;
        match decided {
            Ok(Some(Some(stored))) => Decision::Publish {
                receipt: stored.receipt,
                result_code: stored.result_code,
            },
            Ok(Some(None)) => Decision::Conflict,
            Ok(None) | Err(_) => Decision::Transient,
        }
    }
}

struct Request<'a> {
    source_authority: &'a str,
    command: &'a str,
    operation_id: &'a str,
    operation: [u8; 16],
    recovery_generation: u64,
}

/// `None`: transient, roll back. `Some(None)`: conflict with a stored receipt, nothing
/// written. `Some(Some(_))`: the stored receipt to publish.
async fn decide(
    tx: &mut Tx<'_>,
    request: &Request<'_>,
    input: &Owned,
) -> Result<Option<Option<Stored>>, DurabilityError> {
    let (binding, classified) = match input {
        Owned::Intent(raw) => (
            intent_binding(raw),
            classify_intent(raw, request.command, request.operation_id),
        ),
        Owned::ReadConflict => (
            read_conflict_binding(request.command),
            Classified::Rejected(OPERATION_CONFLICT),
        ),
        Owned::Unsupported => (
            unsupported_binding(request.command),
            Classified::Rejected(UNSUPPORTED),
        ),
    };
    // Operation identity first, then the account portfolio lock (§6.3), so a
    // concurrent reuse observes the committed receipt.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-operation:' || encode($1, 'hex'), 0))")
        .bind(request.operation.as_slice()).execute(&mut **tx).await?;
    if let Classified::Create(intent) = &classified {
        sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES (encode($1, 'hex')::uuid) ON CONFLICT DO NOTHING")
            .bind(intent.account_id.as_slice()).execute(&mut **tx).await?;
        sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id = encode($1, 'hex')::uuid FOR UPDATE")
            .bind(intent.account_id.as_slice()).fetch_one(&mut **tx).await?;
    }
    if let Some(row) = sqlx::query("SELECT command, command_binding, result_code, receipt FROM game_character_command_receipts WHERE operation_id = encode($1, 'hex')::uuid")
        .bind(request.operation.as_slice()).fetch_optional(&mut **tx).await? {
        let same = row.try_get::<String, _>("command")? == request.command
            && row.try_get::<Vec<u8>, _>("command_binding")? == binding;
        if !same {
            return Ok(Some(None));
        }
        return Ok(Some(Some(Stored {
            receipt: row.try_get("receipt")?,
            result_code: row.try_get("result_code")?,
        })));
    }
    if sqlx::query("SELECT 1 FROM game_character_operation_receipts WHERE operation_id = encode($1, 'hex')::uuid")
        .bind(request.operation.as_slice()).fetch_optional(&mut **tx).await?.is_some() {
        return Ok(Some(None));
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let now = u64::try_from(now).map_err(|_| DurabilityError::Unavailable)?;
    let intent = match classified {
        Classified::Rejected(code) => {
            return store_rejected(tx, request, None, &binding, code, now)
                .await
                .map(Some);
        }
        Classified::Create(intent) => intent,
    };
    let interpretation = match check_create(tx, &intent, now).await? {
        Check::Transient => return Ok(None),
        Check::Rejected(code) => {
            let revision = Some(intent.source_revision);
            return store_rejected(tx, request, revision, &binding, code, now)
                .await
                .map(Some);
        }
        Check::Accepted(interpretation) => interpretation,
    };
    commit_create(tx, request, &intent, &binding, interpretation, now)
        .await
        .map(|stored| Some(Some(stored)))
}

enum Check {
    Transient,
    Rejected(&'static str),
    Accepted([String; 4]),
}

/// The §5.3 and §6 checks of a decoded create, in order, holding the portfolio lock.
async fn check_create(
    tx: &mut Tx<'_>,
    intent: &CreateIntent,
    now: u64,
) -> Result<Check, DurabilityError> {
    if intent.expires_at_source <= now {
        return Ok(Check::Rejected(INTENT_EXPIRED));
    }
    if intent.issued_at_source > now.saturating_add(MAX_FUTURE_SKEW_SECONDS) {
        return Ok(Check::Rejected(INTENT_INVALID));
    }
    // §5.3: one non-conflict receipt per (variant, source_revision); a lower
    // revision than one already decided is allowed.
    let revision = intent.source_revision.to_string();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-command-revision:' || $1, 0))")
        .bind(&revision).execute(&mut **tx).await?;
    if sqlx::query("SELECT 1 FROM game_character_command_receipts WHERE variant = $1 AND source_revision = $2::numeric AND result_code <> $3")
        .bind(VARIANT_CREATE).bind(&revision).bind(OPERATION_CONFLICT)
        .fetch_optional(&mut **tx).await?.is_some() {
        return Ok(Check::Rejected(OPERATION_CONFLICT));
    }
    let Ok(name) = CharacterName::parse(&intent.requested_name) else {
        return Ok(Check::Rejected(NAME_INVALID));
    };
    let now_signed = i64::try_from(now).map_err(|_| DurabilityError::Unavailable)?;
    match account_security(tx, &intent.account_id, now_signed).await? {
        Security::Allowed => {}
        Security::Refused => return Ok(Check::Rejected(ACCOUNT_NOT_ELIGIBLE)),
        Security::Unknown => return Ok(Check::Transient),
    }
    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:character-interpretation', 0))")
        .execute(&mut **tx).await?;
    let Some((_, interpretation)) = current_interpretation(tx).await? else {
        return Ok(Check::Transient);
    };
    // A World with a currently assigned Channel accepts creation; a World
    // whose Channels are all unassigned is a transient dependency, and a
    // World Game never assigned is refused.
    if sqlx::query("SELECT scope_key FROM game_runtime_scope_assignments WHERE world_id = encode($1, 'hex')::uuid AND scope_kind = 1 AND state = 1 LIMIT 1 FOR SHARE")
        .bind(intent.target_world_id.as_slice()).fetch_optional(&mut **tx).await?.is_none() {
        let known = sqlx::query("SELECT 1 FROM game_runtime_scope_assignments WHERE world_id = encode($1, 'hex')::uuid AND scope_kind = 1 LIMIT 1")
            .bind(intent.target_world_id.as_slice()).fetch_optional(&mut **tx).await?.is_some();
        return Ok(if known { Check::Transient } else { Check::Rejected(WORLD_NOT_ACCEPTED) });
    }
    let listed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_roots WHERE account_id = encode($1, 'hex')::uuid",
    )
    .bind(intent.account_id.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if listed >= MAX_ACCOUNT_CHARACTERS {
        return Ok(Check::Rejected(ACCOUNT_CHARACTER_LIMIT));
    }
    let name_key = name.comparison_key();
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-name:' || $1, 0))",
    )
    .bind(&name_key)
    .execute(&mut **tx)
    .await?;
    if sqlx::query("SELECT 1 FROM game_character_name_reservations WHERE name_key = $1")
        .bind(&name_key)
        .fetch_optional(&mut **tx)
        .await?
        .is_some()
    {
        return Ok(Check::Rejected(NAME_UNAVAILABLE));
    }
    Ok(Check::Accepted(
        interpretation.revisions().map(str::to_owned),
    ))
}

async fn store_rejected(
    tx: &mut Tx<'_>,
    request: &Request<'_>,
    source_revision: Option<u64>,
    binding: &[u8],
    code: &'static str,
    now: u64,
) -> Result<Option<Stored>, DurabilityError> {
    let receipt = rejected_receipt(
        request.source_authority,
        request.command,
        request.operation_id,
        code,
        request.recovery_generation,
        now,
    );
    if receipt.is_empty() || receipt.len() > MAX_RECEIPT_BYTES {
        return Err(DurabilityError::Unavailable);
    }
    sqlx::query("INSERT INTO game_character_command_receipts(operation_id, command, variant, source_revision, command_binding, outcome, result_code, recovery_generation, decided_at, receipt) VALUES (encode($1,'hex')::uuid, $2, $3, $4::numeric, $5, 2, $6, $7::numeric, $8, $9)")
        .bind(request.operation.as_slice()).bind(request.command)
        .bind(source_revision.map(|_| VARIANT_CREATE)).bind(source_revision.map(|r| r.to_string()))
        .bind(binding).bind(code).bind(request.recovery_generation.to_string())
        .bind(i64::try_from(now).map_err(|_| DurabilityError::Unavailable)?).bind(&receipt)
        .execute(&mut **tx).await?;
    Ok(Some(Stored {
        receipt,
        result_code: code.to_owned(),
    }))
}

/// §6.1: the root, its audit event and the committed receipt, in the deciding
/// transaction. The root insert reserves the name and advances the account's LCFA
/// projection revision and outbox (migration 0024 trigger).
async fn commit_create(
    tx: &mut Tx<'_>,
    request: &Request<'_>,
    intent: &CreateIntent,
    binding: &[u8],
    interpretation: [String; 4],
    now: u64,
) -> Result<Stored, DurabilityError> {
    let ids = sqlx::query("SELECT game_character_uuid_v7()::text AS character_id, game_character_uuid_v7()::text AS event_id, game_character_uuid_v7()::text AS transaction_id, floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at")
        .fetch_one(&mut **tx).await?;
    let character = uuid_text(ids.try_get("character_id")?)?;
    let event = uuid_text(ids.try_get("event_id")?)?;
    let transaction = uuid_text(ids.try_get("transaction_id")?)?;
    let occurred_at: i64 = ids.try_get("occurred_at")?;
    let account = intent.account_id;
    let world = intent.target_world_id;
    let [profile, ruleset, content, starter] = interpretation;
    let payload = encode_bootstrap(&account, &character, &world);
    sqlx::query("INSERT INTO game_character_roots(character_id, account_id, world_id, lifecycle, character_revision, profile_revision, ruleset_revision, content_revision, starter_template_revision, name) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, 1, 1, $4, $5, $6, $7, $8)")
        .bind(character.as_slice()).bind(account.as_slice()).bind(world.as_slice()).bind(profile).bind(ruleset).bind(content).bind(starter).bind(&intent.requested_name).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_character_audit_outbox(event_id, transaction_id, transaction_ordinal, transaction_count, event_type_id, schema_revision, retention_profile_id, character_id, occurred_at, expires_at, payload, payload_sha256, server_build_id, publication_state) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, encode($6,'hex')::uuid, $9, $9 + $10, $7, sha256($7), $8, 1)")
        .bind(event.as_slice()).bind(transaction.as_slice()).bind(i64::from(EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED)).bind(i64::from(EVENT_SCHEMA_REVISION)).bind(RETENTION_PROFILE).bind(character.as_slice()).bind(&payload).bind(SERVER_BUILD_ID).bind(occurred_at).bind(AUDIT_RETENTION_MS).execute(&mut **tx).await?;
    let projection = sqlx::query("SELECT p.projection_revision, e.projection_epoch FROM game_character_account_projections p, game_character_account_projection_epoch e WHERE p.account_id = encode($1,'hex')::uuid AND e.epoch_scope = 1")
        .bind(account.as_slice()).fetch_one(&mut **tx).await?;
    let positive = |value: i64| {
        u64::try_from(value)
            .ok()
            .filter(|v| *v > 0)
            .ok_or(DurabilityError::InvalidStoredState)
    };
    let projection_revision = positive(projection.try_get("projection_revision")?)?;
    let projection_epoch = positive(projection.try_get("projection_epoch")?)?;
    let receipt = committed_create_receipt(
        request.source_authority,
        request.operation_id,
        Committed {
            character_id: character,
            account_id: account,
            world_id: world,
            name: &intent.requested_name,
            projection_epoch,
            projection_revision,
        },
        request.recovery_generation,
        now,
    );
    if receipt.is_empty() || receipt.len() > MAX_RECEIPT_BYTES {
        return Err(DurabilityError::Unavailable);
    }
    sqlx::query("INSERT INTO game_character_command_receipts(operation_id, command, variant, source_revision, command_binding, outcome, result_code, recovery_generation, decided_at, receipt, character_id, account_id, world_id, event_id, transaction_id, server_build_id, occurred_at) VALUES (encode($1,'hex')::uuid, $2, $3, $4::numeric, $5, 1, $6, $7::numeric, $8, $9, encode($10,'hex')::uuid, encode($11,'hex')::uuid, encode($12,'hex')::uuid, encode($13,'hex')::uuid, encode($14,'hex')::uuid, $15, $16)")
        .bind(request.operation.as_slice()).bind(request.command).bind(VARIANT_CREATE)
        .bind(intent.source_revision.to_string()).bind(binding).bind(OK)
        .bind(request.recovery_generation.to_string())
        .bind(i64::try_from(now).map_err(|_| DurabilityError::Unavailable)?).bind(&receipt)
        .bind(character.as_slice()).bind(account.as_slice()).bind(world.as_slice())
        .bind(event.as_slice()).bind(transaction.as_slice()).bind(SERVER_BUILD_ID).bind(occurred_at)
        .execute(&mut **tx).await?;
    Ok(Stored {
        receipt,
        result_code: OK.to_owned(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "character_command_tests.rs"]
mod tests;
