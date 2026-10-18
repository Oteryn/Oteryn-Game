//! D1088: one participant in the existing semantic transaction owner.
//! D1343: migration 0116 installs the staff audit schema.
//! Initial provisioning imports the independently reviewed signed activation
//! as link 1 plus its signed COMMITTED receipt before seeding the head.
//! No genesis=0, runtime ROSTER_ACTIVATE, or automatic trust bootstrapping.
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};
use crate::staff::{
    audit::{self, Contract, ResultCode},
    signature::{
        self, Disposition, Error, History, Outcome, SignedBytes, VerifiedReceipt, VerifiedRequest,
        VerifierTrust,
    },
};
use sqlx::{PgConnection, Row};
use std::sync::Arc;

#[derive(Debug)]
pub enum StaffError {
    Rejected(Error),
    Conflict,
    OperationConflict,
    UnsupportedAction,
    Database(DurabilityError),
}
impl From<sqlx::Error> for StaffError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.into())
    }
}
impl From<DurabilityError> for StaffError {
    fn from(e: DurabilityError) -> Self {
        Self::Database(e)
    }
}
impl From<Error> for StaffError {
    fn from(e: Error) -> Self {
        Self::Rejected(e)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub sequence: u64,
    pub digest: String,
    pub activation_digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredOutcome {
    pub bytes: Vec<u8>,
    pub outcome: Outcome,
}
pub struct EvidenceRow {
    pub link: signature::ChainLink,
    pub operation: Option<OperationEvidence>,
}
pub struct OperationEvidence {
    pub record: audit::StoredOperation,
    pub envelope: Option<audit::StoredEvent>,
    pub receipt: Option<SignedBytes>,
    pub receipt_link: Option<Vec<u8>>,
    pub receipt_sequence: Option<u64>,
}
pub(crate) enum Begin {
    Replay(StoredOutcome),
    Fresh(Box<Pending>),
}
pub(crate) struct Pending {
    request: VerifiedRequest,
    physical_transaction: i64,
    occurred_at: i64,
}

async fn physical_transaction(connection: &mut PgConnection) -> Result<i64, StaffError> {
    Ok(sqlx::query_scalar("SELECT txid_current()::bigint")
        .fetch_one(connection)
        .await?)
}
pub(crate) async fn lock_head(
    connection: &mut PgConnection,
    environment: &str,
) -> Result<Head, StaffError> {
    let row=sqlx::query("SELECT control_seq::text, link_sha256, activation_sha256 FROM game_staff_control_heads WHERE environment=$1 FOR UPDATE")
        .bind(environment).fetch_optional(connection).await?.ok_or(Error::MissingMaterial)?;
    Ok(Head {
        sequence: row
            .try_get::<String, _>(0)?
            .parse()
            .map_err(|_| Error::Invalid)?,
        digest: row.try_get(1)?,
        activation_digest: row.try_get(2)?,
    })
}

/// The caller holds the locked head in this same transaction and has checked
/// that it names `trust`'s activation. The head link itself must verify under
/// that activation before any new link may name it as predecessor, so links
/// written directly with runtime credentials are never extended.
async fn verify_head_in_transaction(
    connection: &mut PgConnection,
    trust: &VerifierTrust,
    environment: &str,
    head: &Head,
) -> Result<(), StaffError> {
    let row=sqlx::query("SELECT previous_link_sha256,link_sha256,roster_sha256,key_fingerprint,signed_bytes,principal,signature,link_kind FROM game_staff_control_links WHERE environment=$1 AND control_seq=$2::text::numeric")
        .bind(environment).bind(head.sequence.to_string()).fetch_optional(&mut *connection).await?.ok_or(Error::Chain)?;
    let link = signature::ChainLink {
        sequence: head.sequence,
        previous: row.try_get(0)?,
        digest: row.try_get(1)?,
        roster: row.try_get(2)?,
        fingerprint: row.try_get(3)?,
        signed: SignedBytes {
            bytes: row.try_get(4)?,
            principal: row.try_get(5)?,
            signature: row.try_get(6)?,
        },
        kind: match row.try_get::<String, _>(7)?.as_str() {
            "REQUEST" => signature::LinkKind::Request,
            "RECEIPT" => signature::LinkKind::Receipt,
            "ACTIVATION" => signature::LinkKind::Activation,
            _ => return Err(Error::Invalid.into()),
        },
    };
    if head.activation_digest != trust.activation_digest() || link.digest != head.digest {
        return Err(Error::Binding.into());
    }
    match signature::verify_head_link(trust, &link)? {
        signature::VerifiedLink::Request(request) => {
            verify_head_operation_in_transaction(connection, &request, link.kind).await
        }
        signature::VerifiedLink::Receipt(_) => Ok(()),
    }
}

/// A signed REQUEST link authenticates only the request. Its outcome, receipt
/// requirement and audit event are unsigned rows written beside it, so before
/// the head is extended they must bind to that signed request: the receipt
/// requirement is derived from the signed action, never read from SQL.
async fn verify_head_operation_in_transaction(
    connection: &mut PgConnection,
    request: &VerifiedRequest,
    kind: signature::LinkKind,
) -> Result<(), StaffError> {
    let row=sqlx::query("SELECT environment,operation_id::text,request_sha256,request_bytes,outcome_bytes,outcome_sha256,principal,key_fingerprint,action_id::text,transaction_id::text,control_seq::text,requires_receipt,occurred_at FROM game_staff_operations WHERE environment=$1 AND request_sha256=$2")
        .bind(&request.request().environment).bind(request.digest()).fetch_optional(&mut *connection).await?;
    let Some(row) = row else {
        // Owner-provisioned activations carry no operation row.
        return if kind == signature::LinkKind::Activation {
            Ok(())
        } else {
            Err(Error::MissingMaterial.into())
        };
    };
    let record = stored_operation(&row)?;
    let outcome = audit::verify_stored_operation(request, &record)?;
    let event=sqlx::query("SELECT environment,operation_id::text,envelope,event_id::text,transaction_id::text,event_type_id,schema_revision,retention_profile_id,occurred_at,expires_at,envelope_sha256,publication_state FROM game_staff_action_outbox WHERE event_id=$1::text::uuid")
        .bind(&record.action_id).fetch_optional(&mut *connection).await?;
    if let Some(row) = event {
        let event = audit::StoredEvent {
            environment: row.try_get(0)?,
            operation_id: row.try_get(1)?,
            envelope: row.try_get(2)?,
            event_id: row.try_get(3)?,
            transaction_id: row.try_get(4)?,
            event_type: row.try_get(5)?,
            schema_revision: row.try_get(6)?,
            retention_profile: row.try_get(7)?,
            occurred_at: row.try_get(8)?,
            expires_at: row.try_get(9)?,
            envelope_sha256: row.try_get(10)?,
            publication_state: row.try_get(11)?,
        };
        audit::verify_stored_event(Contract::registered()?, request, &outcome, &event)?;
    } else if signature::synchronized_now_ms()? < audit::evidence_expiry(request, &outcome, None)? {
        // Only an audit event past its signed retention bound may be absent.
        return Err(Error::MissingMaterial.into());
    }
    Ok(())
}

/// Retry lookup precedes freshness and predecessor checks. It returns evidence,
/// never a fresh authority capability or another domain execution permission.
pub(crate) async fn begin_in_transaction(
    connection: &mut PgConnection,
    trust: &VerifierTrust,
    history: &History,
    request: VerifiedRequest,
) -> Result<Begin, StaffError> {
    begin_with_clock(
        connection,
        trust,
        history,
        request,
        signature::synchronized_now_ms,
    )
    .await
}
#[cfg(test)]
#[allow(
    dead_code,
    reason = "Used by the source-included PostgreSQL fixture, not library unit tests"
)]
pub(crate) async fn fixture_begin_in_transaction(
    connection: &mut PgConnection,
    trust: &VerifierTrust,
    history: &History,
    request: VerifiedRequest,
    now: Result<u64, Error>,
) -> Result<Begin, StaffError> {
    begin_with_clock(connection, trust, history, request, || now).await
}
async fn begin_with_clock(
    connection: &mut PgConnection,
    trust: &VerifierTrust,
    history: &History,
    request: VerifiedRequest,
    clock: impl FnOnce() -> Result<u64, Error>,
) -> Result<Begin, StaffError> {
    if request.request().environment != trust.environment()
        || request.request().roster_sha256 != trust.roster_digest()
        || request.activation_digest() != trust.activation_digest()
    {
        return Err(Error::Binding.into());
    }
    let head = lock_head(connection, trust.environment()).await?;
    if let Some(stored) = historical_outcome_in_transaction(connection, &request).await? {
        return Ok(Begin::Replay(stored));
    }
    if head.activation_digest != trust.activation_digest() {
        return Err(Error::Binding.into());
    }
    if head.sequence.checked_add(1) != Some(request.request().control_seq.0)
        || head.digest != request.request().previous_link_sha256
    {
        return Err(StaffError::Conflict);
    }
    // Refuse a new request until the latest operator mutation is reconciled.
    receipt_fence_in_transaction(connection, history, trust.environment()).await?;
    verify_head_in_transaction(connection, trust, trust.environment(), &head).await?;
    let physical_transaction = physical_transaction(connection).await?;
    // Sample after every awaited prerequisite. Terminal replay never calls it.
    let now = clock();
    signature::check_freshness(request.request(), now)?;
    let occurred_at =
        i64::try_from(now.map_err(StaffError::Rejected)?).map_err(|_| Error::Clock)?;
    Ok(Begin::Fresh(Box::new(Pending {
        physical_transaction,
        request,
        occurred_at,
    })))
}

#[cfg(test)]
#[allow(
    dead_code,
    reason = "Used by the source-included PostgreSQL fixture, not library unit tests"
)]
pub(crate) async fn fixture_begin_with_clock(
    connection: &mut PgConnection,
    trust: &VerifierTrust,
    history: &History,
    request: VerifiedRequest,
    clock: impl FnOnce() -> Result<u64, Error>,
) -> Result<Begin, StaffError> {
    begin_with_clock(connection, trust, history, request, clock).await
}

/// D1412: the caller holds the locked head in this same transaction. Every
/// mutation needs a receipt row, and the receipt of the latest mutation, the
/// one that releases its pending fence, must pass the existing verifier under
/// the reviewed activation in effect at that receipt's own sequence. Row
/// existence never opens the fence; an unknown activation or any mismatch
/// refuses and keeps it. Earlier receipts were checked when their own fence
/// was released, so no request re-verifies the whole retained history.
async fn receipt_fence_in_transaction(
    connection: &mut PgConnection,
    history: &History,
    environment: &str,
) -> Result<(), StaffError> {
    let missing:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_operations o WHERE o.environment=$1 AND o.requires_receipt AND NOT EXISTS(SELECT 1 FROM game_staff_action_receipts r WHERE r.environment=o.environment AND r.operation_id=o.operation_id))")
        .bind(environment).fetch_one(&mut *connection).await?;
    if missing {
        return Err(Error::Receipt.into());
    }
    let row=sqlx::query("SELECT o.operation_id::text,o.request_sha256,o.outcome_sha256,o.outcome_bytes,o.principal,o.key_fingerprint,o.control_seq::text,r.receipt_bytes,r.principal,r.key_fingerprint,r.signature,r.control_seq::text,l.signed_bytes,l.previous_link_sha256,l.link_sha256,l.principal,l.key_fingerprint,l.signature FROM game_staff_operations o JOIN game_staff_action_receipts r ON r.environment=o.environment AND r.operation_id=o.operation_id LEFT JOIN game_staff_control_links l ON l.environment=r.environment AND l.control_seq=r.control_seq AND l.link_kind='RECEIPT' WHERE o.environment=$1 AND o.requires_receipt ORDER BY o.control_seq DESC LIMIT 1")
        .bind(environment).fetch_optional(&mut *connection).await?;
    let Some(row) = row else {
        return Ok(());
    };
    let parse = |text: String| text.parse::<u64>().map_err(|_| Error::Invalid);
    let operation_sequence = parse(row.try_get(6)?)?;
    let receipt_sequence = parse(row.try_get(11)?)?;
    let signed = SignedBytes {
        bytes: row.try_get(7)?,
        principal: row.try_get(8)?,
        signature: row.try_get(10)?,
    };
    let fingerprint: String = row.try_get(9)?;
    let link_bytes: Option<Vec<u8>> = row.try_get(12)?;
    let link_previous: Option<String> = row.try_get(13)?;
    if link_bytes.as_ref() != Some(&signed.bytes)
        || row.try_get::<Option<String>, _>(14)? != Some(signature::sha256(&signed.bytes))
        || row.try_get::<Option<String>, _>(15)?.as_ref() != Some(&signed.principal)
        || row.try_get::<Option<String>, _>(16)?.as_ref() != Some(&fingerprint)
        || row.try_get::<Option<String>, _>(17)?.as_ref() != Some(&signed.signature)
        || receipt_sequence <= operation_sequence
    {
        return Err(Error::Receipt.into());
    }
    let receipt = history.receipt_at(signed, receipt_sequence)?;
    let value = receipt.receipt();
    let outcome: Outcome = oteryn_release_manifest::jcs::decode(
        &row.try_get::<Vec<u8>, _>(3)?,
        signature::MAX_OUTCOME_BYTES,
    )
    .map_err(|_| Error::Invalid)?;
    let operation_id: String = row.try_get(0)?;
    if value.environment != environment
        || crate::node::uuid_bytes(&value.operation_id).is_none()
        || crate::node::uuid_bytes(&value.operation_id) != crate::node::uuid_bytes(&operation_id)
        || row.try_get::<String, _>(1)? != value.request_sha256
        || row.try_get::<String, _>(2)? != value.outcome_sha256
        || row.try_get::<String, _>(4)? != receipt.signed().principal
        || row.try_get::<String, _>(5)? != receipt.fingerprint()
        || receipt.fingerprint() != fingerprint
        || outcome.disposition != value.disposition
        || link_previous.as_deref() != Some(value.previous_link_sha256.as_str())
    {
        return Err(Error::Receipt.into());
    }
    Ok(())
}

/// Caller performs its own fenced domain change on this SAME connection before
/// finish. The outcome FK/audit proof must also be carried by that domain row.
/// Never call a second root semantic pass from inside a domain transaction.
pub(crate) async fn finish_in_transaction(
    connection: &mut PgConnection,
    pending: Box<Pending>,
    contract: Contract,
    outcome: Outcome,
    server_build: &str,
) -> Result<StoredOutcome, StaffError> {
    if physical_transaction(connection).await? != pending.physical_transaction {
        return Err(Error::Binding.into());
    }
    if (outcome.disposition == Disposition::Committed) != (outcome.result == ResultCode::Ok) {
        return Err(Error::Invalid.into());
    }
    if u64::try_from(pending.occurred_at).ok() != Some(outcome.occurred_at.0) {
        return Err(Error::Binding.into());
    }
    let request = &pending.request;
    let environment = &request.request().environment;
    let bytes = outcome.canonical_bytes()?;
    let envelope = audit::encode(
        contract,
        request,
        &outcome,
        pending.occurred_at,
        server_build,
    )?;
    insert_link(
        connection,
        environment,
        request.request().control_seq.0,
        &request.request().previous_link_sha256,
        &request.request().roster_sha256,
        "REQUEST",
        request.signed(),
        request.fingerprint(),
    )
    .await?;
    sqlx::query("INSERT INTO game_staff_operations(environment,operation_id,request_sha256,request_bytes,outcome_bytes,outcome_sha256,principal,key_fingerprint,action_id,transaction_id,control_seq,requires_receipt,occurred_at,created_xact_id) VALUES($1,$2::text::uuid,$3,$4,$5,$6,$7,$8,$9::text::uuid,$10::text::uuid,$11::text::numeric,$12,$13,txid_current())")
        .bind(environment).bind(&request.request().operation_id).bind(request.digest()).bind(&request.signed().bytes).bind(&bytes).bind(signature::sha256(&bytes))
        .bind(&request.signed().principal).bind(request.fingerprint()).bind(&outcome.action_id).bind(&outcome.transaction_id)
        .bind(request.request().control_seq.0.to_string()).bind(!request.request().action.kind().is_read()).bind(pending.occurred_at).execute(&mut *connection).await?;
    sqlx::query("INSERT INTO game_staff_action_outbox(environment,operation_id,event_id,transaction_id,event_type_id,schema_revision,retention_profile_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state,created_xact_id) VALUES($1,$2::text::uuid,$3::text::uuid,$4::text::uuid,$5,$6,$7,$8,$9,$10,$11,1,txid_current())")
        .bind(environment).bind(&request.request().operation_id).bind(&outcome.action_id).bind(&outcome.transaction_id)
        .bind(i64::from(contract.event_type())).bind(i64::from(contract.revision())).bind(audit::RETENTION_PROFILE)
        .bind(pending.occurred_at).bind(pending.occurred_at.checked_add(audit::RETENTION_MS).ok_or(Error::Clock)?)
        .bind(&envelope).bind(signature::sha256(&envelope)).execute(&mut *connection).await?;
    advance_head(
        connection,
        environment,
        request.request().control_seq.0,
        request.digest(),
        &request.request().previous_link_sha256,
    )
    .await?;
    Ok(StoredOutcome { bytes, outcome })
}

#[allow(clippy::too_many_arguments)]
async fn insert_link(
    connection: &mut PgConnection,
    environment: &str,
    sequence: u64,
    previous: &str,
    roster: &str,
    kind: &str,
    signed: &SignedBytes,
    fingerprint: &str,
) -> Result<(), StaffError> {
    sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,link_sha256,previous_link_sha256,roster_sha256,link_kind,signed_bytes,principal,key_fingerprint,signature,created_xact_id) VALUES($1,$2::text::numeric,$3,$4,$5,$6,$7,$8,$9,$10,txid_current())")
        .bind(environment).bind(sequence.to_string()).bind(signature::sha256(&signed.bytes)).bind(previous).bind(roster).bind(kind)
        .bind(&signed.bytes).bind(&signed.principal).bind(fingerprint).bind(&signed.signature).execute(connection).await?;
    Ok(())
}
async fn advance_head(
    connection: &mut PgConnection,
    environment: &str,
    sequence: u64,
    digest: &str,
    previous: &str,
) -> Result<(), StaffError> {
    let count=sqlx::query("UPDATE game_staff_control_heads SET control_seq=$2::text::numeric,link_sha256=$3 WHERE environment=$1 AND control_seq=$2::text::numeric-1 AND link_sha256=$4")
        .bind(environment).bind(sequence.to_string()).bind(digest).bind(previous).execute(connection).await?.rows_affected();
    if count != 1 {
        return Err(StaffError::Conflict);
    }
    Ok(())
}

/// A chain-first traversal alone misses rows forged outside that chain by a
/// hostile database owner. Check retained operation/outbox/receipt rows too.
pub(crate) async fn evidence_orphans_in_transaction(
    connection: &mut PgConnection,
    environment: &str,
    through: u64,
) -> Result<bool, StaffError> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_operations o WHERE o.environment=$1 AND NOT EXISTS(SELECT 1 FROM game_staff_control_links l WHERE l.environment=o.environment AND l.control_seq=o.control_seq AND l.link_kind='REQUEST' AND l.link_sha256=o.request_sha256 AND l.signed_bytes=o.request_bytes AND l.principal=o.principal AND l.key_fingerprint=o.key_fingerprint AND l.created_xact_id=o.created_xact_id) UNION ALL SELECT 1 FROM game_staff_action_outbox a LEFT JOIN game_staff_operations o ON o.environment=a.environment AND o.operation_id=a.operation_id WHERE a.environment=$1 AND (o.operation_id IS NULL OR o.action_id<>a.event_id OR o.transaction_id<>a.transaction_id OR o.occurred_at<>a.occurred_at OR o.created_xact_id<>a.created_xact_id) UNION ALL SELECT 1 FROM game_staff_action_receipts r WHERE r.environment=$1 AND (NOT EXISTS(SELECT 1 FROM game_staff_operations o WHERE o.environment=r.environment AND o.operation_id=r.operation_id) OR NOT EXISTS(SELECT 1 FROM game_staff_control_links l WHERE l.environment=r.environment AND l.control_seq=r.control_seq AND l.link_kind='RECEIPT' AND l.signed_bytes=r.receipt_bytes AND l.principal=r.principal AND l.key_fingerprint=r.key_fingerprint AND l.signature=r.signature AND l.created_xact_id=r.created_xact_id)) UNION ALL SELECT 1 FROM game_staff_control_links l LEFT JOIN game_staff_control_heads h ON h.environment=l.environment WHERE l.environment=$1 AND (h.environment IS NULL OR l.control_seq>h.control_seq) UNION ALL SELECT 1 FROM game_staff_control_links l WHERE l.environment=$1 AND ((l.link_kind='REQUEST' AND NOT EXISTS(SELECT 1 FROM game_staff_operations o WHERE o.environment=l.environment AND o.control_seq=l.control_seq)) OR (l.link_kind='RECEIPT' AND NOT EXISTS(SELECT 1 FROM game_staff_action_receipts r WHERE r.environment=l.environment AND r.control_seq=l.control_seq))) UNION ALL SELECT 1 FROM game_staff_control_heads h WHERE h.environment=$1 AND h.control_seq<$2::text::numeric UNION ALL SELECT 1 FROM game_staff_operations o WHERE o.environment=$1 AND (o.outcome_sha256<>encode(sha256(o.outcome_bytes),'hex') OR o.request_sha256<>encode(sha256(o.request_bytes),'hex')))")
        .bind(environment).bind(through.to_string()).fetch_one(connection).await?)
}

pub(crate) async fn receipt_in_transaction(
    connection: &mut PgConnection,
    trust: &VerifierTrust,
    receipt: &VerifiedReceipt,
) -> Result<(), StaffError> {
    let value = receipt.receipt();
    if value.environment != trust.environment() || value.roster_sha256 != trust.roster_digest() {
        return Err(Error::Binding.into());
    }
    let head = lock_head(connection, trust.environment()).await?;
    let row=sqlx::query("SELECT request_sha256,outcome_sha256,principal,key_fingerprint,outcome_bytes FROM game_staff_operations WHERE environment=$1 AND operation_id=$2::text::uuid")
        .bind(trust.environment()).bind(&value.operation_id).fetch_optional(&mut *connection).await?.ok_or(Error::Receipt)?;
    let outcome: Outcome = oteryn_release_manifest::jcs::decode(
        &row.try_get::<Vec<u8>, _>(4)?,
        signature::MAX_OUTCOME_BYTES,
    )
    .map_err(|_| Error::Invalid)?;
    if row.try_get::<String, _>(0)? != value.request_sha256
        || row.try_get::<String, _>(1)? != value.outcome_sha256
        || row.try_get::<String, _>(2)? != receipt.signed().principal
        || row.try_get::<String, _>(3)? != receipt.fingerprint()
        || outcome.disposition != value.disposition
    {
        return Err(Error::Receipt.into());
    }
    let existing:Option<Vec<u8>>=sqlx::query_scalar("SELECT receipt_bytes FROM game_staff_action_receipts WHERE environment=$1 AND operation_id=$2::text::uuid")
        .bind(trust.environment()).bind(&value.operation_id).fetch_optional(&mut *connection).await?;
    if let Some(bytes) = existing {
        return if bytes == receipt.signed().bytes {
            Ok(())
        } else {
            Err(Error::Receipt.into())
        };
    }
    if head.activation_digest != trust.activation_digest()
        || head.sequence.checked_add(1) != Some(value.control_seq.0)
        || head.digest != value.previous_link_sha256
    {
        return Err(StaffError::Conflict);
    }
    verify_head_in_transaction(connection, trust, trust.environment(), &head).await?;
    insert_link(
        connection,
        trust.environment(),
        value.control_seq.0,
        &value.previous_link_sha256,
        &value.roster_sha256,
        "RECEIPT",
        receipt.signed(),
        receipt.fingerprint(),
    )
    .await?;
    sqlx::query("INSERT INTO game_staff_action_receipts(environment,operation_id,receipt_bytes,principal,key_fingerprint,signature,control_seq,created_xact_id) VALUES($1,$2::text::uuid,$3,$4,$5,$6,$7::text::numeric,txid_current())")
        .bind(trust.environment()).bind(&value.operation_id).bind(&receipt.signed().bytes).bind(&receipt.signed().principal).bind(receipt.fingerprint())
        .bind(&receipt.signed().signature).bind(value.control_seq.0.to_string()).execute(&mut *connection).await?;
    advance_head(
        connection,
        trust.environment(),
        value.control_seq.0,
        &signature::sha256(&receipt.signed().bytes),
        &value.previous_link_sha256,
    )
    .await
}

impl DurabilityRoot {
    /// A bounded historical SELECT, never a fallback execution route. The
    /// cursor sealed this request only after checking the independently reviewed
    /// roster activations and the complete retained chain through its head.
    pub async fn reconcile_staff_historical(
        &self,
        evidence: signature::HistoricalRequest,
    ) -> Result<Option<StoredOutcome>, StaffError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    sqlx::query("SET TRANSACTION READ ONLY")
                        .execute(&mut *tx)
                        .await?;
                    Ok(historical_outcome_in_transaction(&mut tx, evidence.request()).await)
                })
            })
            .await?
    }
    pub async fn staff_evidence_has_orphans(
        &self,
        environment: String,
        through: u64,
    ) -> Result<bool, StaffError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    Ok(evidence_orphans_in_transaction(&mut tx, &environment, through).await)
                })
            })
            .await?
    }
    /// The signature is checked before the root acquires a connection. A read
    /// caller may disclose data only after this committed outcome is returned.
    /// Mutating actions are owned by later allocated children and refused here.
    pub async fn audit_staff_read(
        &self,
        trust: Arc<VerifierTrust>,
        history: Arc<History>,
        signed: SignedBytes,
    ) -> Result<StoredOutcome, StaffError> {
        let request = trust.verify_request(signed)?;
        if !request.request().action.kind().is_read() {
            return Err(StaffError::UnsupportedAction);
        }
        let contract = Contract::registered()?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                        match begin_in_transaction(&mut tx, &trust, &history, request).await? {
                            Begin::Replay(outcome) => Ok(outcome),
                            Begin::Fresh(pending) => {
                                let action_id = crate::node::operator_files::uuid_v7()
                                    .map_err(|_| Error::Invalid)?;
                                let transaction_id = crate::node::operator_files::uuid_v7()
                                    .map_err(|_| Error::Invalid)?;
                                let occurred_at = signature::Decimal(
                                    u64::try_from(pending.occurred_at).map_err(|_| Error::Clock)?,
                                );
                                finish_in_transaction(
                                    &mut tx,
                                    pending,
                                    contract,
                                    Outcome {
                                        disposition: Disposition::Committed,
                                        result: ResultCode::Ok,
                                        action_id: crate::node::uuid_text(&action_id),
                                        transaction_id: crate::node::uuid_text(&transaction_id),
                                        occurred_at,
                                    },
                                    super::character_authority::SERVER_BUILD_ID,
                                )
                                .await
                            }
                        }
                    }
                    .await;
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await?
    }
    pub async fn store_staff_receipt(
        &self,
        trust: Arc<VerifierTrust>,
        signed: SignedBytes,
    ) -> Result<(), StaffError> {
        let receipt = trust.verify_receipt(signed)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = receipt_in_transaction(&mut tx, &trust, &receipt).await;
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await?
    }
    /// D1412: preparation opens only through the same verified receipt fence.
    pub async fn staff_prepare_head(
        &self,
        environment: String,
        history: Arc<History>,
    ) -> Result<Head, StaffError> {
        self.staff_head(environment, Some(history)).await
    }
    /// Receipt preparation must remain possible while a mutation lacks its
    /// receipt. This read returns no mutation or activation capability.
    pub async fn staff_receipt_head(&self, environment: String) -> Result<Head, StaffError> {
        self.staff_head(environment, None).await
    }
    async fn staff_head(
        &self,
        environment: String,
        receipt_fence: Option<Arc<History>>,
    ) -> Result<Head, StaffError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                        let head = lock_head(&mut tx, &environment).await?;
                        if let Some(history) = &receipt_fence {
                            receipt_fence_in_transaction(&mut tx, history, &environment).await?;
                            verify_head_in_transaction(
                                &mut tx,
                                history.current(),
                                &environment,
                                &head,
                            )
                            .await?;
                        }
                        Ok(head)
                    }
                    .await;
                    Ok(result)
                })
            })
            .await?
    }
    /// Bounded streaming inspection. `through` is the initial locked head;
    /// later appends cannot change that immutable verification prefix.
    pub async fn staff_evidence_page(
        &self,
        environment: String,
        after: u64,
        through: u64,
    ) -> Result<Vec<EvidenceRow>, StaffError> {
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let rows=sqlx::query("SELECT l.control_seq::text,l.previous_link_sha256,l.link_sha256,l.roster_sha256,l.key_fingerprint,l.signed_bytes,l.principal,l.signature,l.link_kind,o.request_bytes,o.outcome_bytes,o.occurred_at,a.envelope,r.receipt_bytes,r.principal AS receipt_principal,r.signature AS receipt_signature,rl.signed_bytes AS receipt_link,r.control_seq::text,o.requires_receipt,a.event_id::text,a.transaction_id::text,a.event_type_id,a.schema_revision,a.retention_profile_id,a.occurred_at,a.expires_at,a.envelope_sha256,a.publication_state,o.environment,o.operation_id::text,o.request_sha256,o.outcome_sha256,o.principal,o.key_fingerprint,o.action_id::text,o.transaction_id::text,o.control_seq::text,a.environment,a.operation_id::text FROM game_staff_control_links l LEFT JOIN game_staff_operations o ON o.environment=l.environment AND o.control_seq=l.control_seq LEFT JOIN game_staff_action_outbox a ON a.environment=o.environment AND a.operation_id=o.operation_id LEFT JOIN game_staff_action_receipts r ON r.environment=o.environment AND r.operation_id=o.operation_id AND r.control_seq<=$3::text::numeric LEFT JOIN game_staff_control_links rl ON rl.environment=r.environment AND rl.control_seq=r.control_seq WHERE l.environment=$1 AND l.control_seq>$2::text::numeric AND l.control_seq<=$3::text::numeric ORDER BY l.control_seq LIMIT $4")
                .bind(&environment).bind(after.to_string()).bind(through.to_string()).bind(signature::MAX_EVIDENCE_BATCH).fetch_all(&mut *tx).await?;
            let result=rows.into_iter().map(|row|->Result<EvidenceRow,StaffError> {
                let sequence=row.try_get::<String,_>(0)?.parse().map_err(|_|Error::Invalid)?;
                let request_bytes:Option<Vec<u8>>=row.try_get(9)?;
                let operation=if let Some(request_bytes)=request_bytes {
                    let receipt_bytes:Option<Vec<u8>>=row.try_get(13)?;
                    let envelope:Option<Vec<u8>>=row.try_get(12)?;
                    let envelope=envelope.map(|envelope|Ok::<_,sqlx::Error>(audit::StoredEvent {environment:row.try_get(37)?,operation_id:row.try_get(38)?,envelope,event_id:row.try_get(19)?,transaction_id:row.try_get(20)?,event_type:row.try_get(21)?,schema_revision:row.try_get(22)?,retention_profile:row.try_get(23)?,occurred_at:row.try_get(24)?,expires_at:row.try_get(25)?,envelope_sha256:row.try_get(26)?,publication_state:row.try_get(27)?})).transpose()?;
                    Some(OperationEvidence {record:audit::StoredOperation {request_bytes,outcome_bytes:row.try_get(10)?,occurred_at:row.try_get(11)?,requires_receipt:row.try_get(18)?,environment:row.try_get(28)?,operation_id:row.try_get(29)?,request_sha256:row.try_get(30)?,outcome_sha256:row.try_get(31)?,principal:row.try_get(32)?,fingerprint:row.try_get(33)?,action_id:row.try_get(34)?,transaction_id:row.try_get(35)?,control_seq:row.try_get::<String,_>(36)?.parse().map_err(|_|Error::Invalid)?},envelope,
                        receipt:receipt_bytes.map(|bytes|Ok::<_,sqlx::Error>(SignedBytes {bytes,principal:row.try_get(14)?,signature:row.try_get(15)?})).transpose()?,receipt_link:row.try_get(16)?,
                        receipt_sequence:row.try_get::<Option<String>,_>(17)?.map(|s|s.parse().map_err(|_|Error::Invalid)).transpose()?})
                } else {None};
                Ok(EvidenceRow {link:signature::ChainLink {sequence,previous:row.try_get(1)?,digest:row.try_get(2)?,roster:row.try_get(3)?,fingerprint:row.try_get(4)?,
                    signed:SignedBytes {bytes:row.try_get(5)?,principal:row.try_get(6)?,signature:row.try_get(7)?},kind:match row.try_get::<String,_>(8)?.as_str() {"RECEIPT"=>signature::LinkKind::Receipt,"REQUEST"=>signature::LinkKind::Request,"ACTIVATION"=>signature::LinkKind::Activation,_=>return Err(Error::Invalid.into())}},operation})
            }).collect::<Result<Vec<_>,_>>();
            Ok(result)
        })).await?
    }
}

async fn historical_outcome_in_transaction(
    connection: &mut PgConnection,
    request: &VerifiedRequest,
) -> Result<Option<StoredOutcome>, StaffError> {
    let row=sqlx::query("SELECT environment,operation_id::text,request_sha256,request_bytes,outcome_bytes,outcome_sha256,principal,key_fingerprint,action_id::text,transaction_id::text,control_seq::text,requires_receipt,occurred_at FROM game_staff_operations WHERE environment=$1 AND operation_id=$2::text::uuid")
        .bind(&request.request().environment).bind(&request.request().operation_id).fetch_optional(connection).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.try_get::<String, _>(2)? != request.digest()
        || row.try_get::<Vec<u8>, _>(3)? != request.signed().bytes
    {
        return Err(StaffError::OperationConflict);
    }
    let record = stored_operation(&row)?;
    let outcome = audit::verify_stored_operation(request, &record)?;
    Ok(Some(StoredOutcome {
        bytes: record.outcome_bytes,
        outcome,
    }))
}

/// Columns in the order both operation queries select them.
fn stored_operation(row: &sqlx::postgres::PgRow) -> Result<audit::StoredOperation, StaffError> {
    Ok(audit::StoredOperation {
        environment: row.try_get(0)?,
        operation_id: row.try_get(1)?,
        request_sha256: row.try_get(2)?,
        request_bytes: row.try_get(3)?,
        outcome_bytes: row.try_get(4)?,
        outcome_sha256: row.try_get(5)?,
        principal: row.try_get(6)?,
        fingerprint: row.try_get(7)?,
        action_id: row.try_get(8)?,
        transaction_id: row.try_get(9)?,
        control_seq: row
            .try_get::<String, _>(10)?
            .parse()
            .map_err(|_| Error::Invalid)?,
        requires_receipt: row.try_get(11)?,
        occurred_at: row.try_get(12)?,
    })
}
