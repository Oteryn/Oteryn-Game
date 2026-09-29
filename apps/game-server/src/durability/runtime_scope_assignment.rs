//! GameNode process-incarnation registration (`GAME-NODE-REGISTRATION-BOOTSTRAP-AUTH-V1`)
//! and the Channel-only runtime-scope assignment writer (`OPS-SCOPE-ASSIGNMENT-FENCING-V1`)
//! bounded by the registered `NASG-*` rows of `NATIVE-SOURCE-RESOURCE-ENVELOPE-V1`.
//!
//! A `NodeId` is never a credential. Registration consumes one launch-scoped
//! bootstrap authorization; assignment accepts only a currently registered
//! incarnation and allocates its own source revision/decision identity.
//!
//! Lock order for every assignment mutation: admission relations (lexical,
//! EXCLUSIVE) -> writer slot -> writer namespace -> scope assignment -> target
//! registration (share) -> Runtime guard fence. Runtime guard publication takes
//! the admission relations first and then checks the assignment under the same
//! serialization, so no stale readiness can be restored.

use super::admission_authority_guards::{
    AdmissionGuardStore, GuardPublicationDisposition, Writer, decode_guard, encode_guard, write_key,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::{DurabilityError, DurabilityRoot, MAX_ADMISSION_GUARD_BYTES};
use base64::Engine;
use oteryn_game_server::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityPublicationErrorV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
    AdmissionPublicationSourceV1,
};
use oteryn_game_server::foundation::{ChannelId, NodeId, RuntimeScopeRefV1, WorldId};
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// `NASG-OPERATION-KEY`: exactly 32 opaque bytes.
pub const OPERATION_KEY_BYTES: usize = 32;
/// `NASG-OPERATION-KEY` canonical unpadded base64url text length.
pub const OPERATION_KEY_TEXT_BYTES: usize = 43;
/// `NASG-COMMAND-BYTES`: retained/encoded bytes per command.
pub const MAX_COMMAND_BYTES: usize = 1_024;
/// `NASG-COMMAND-BYTES`: actor/source identity bound.
pub const MAX_IDENTITY_BYTES: usize = 128;
/// `NASG-QUEUE`: pending commands per writer process.
pub const MAX_PENDING_COMMANDS: usize = 8;
/// `NASG-QUEUE`: aggregate retained pending bytes per writer process.
pub const MAX_PENDING_BYTES: usize = 8_192;
/// `NASG-INFLIGHT`: retained typed command/result/checkpoint bytes.
pub const MAX_INFLIGHT_BYTES: usize = 4_096;
/// `NASG-EXECUTION`: queue wait.
pub const QUEUE_WAIT: Duration = Duration::from_millis(1_000);
/// `NASG-EXECUTION`: dispatched operation budget including DB/lock acquisition.
pub const DISPATCH_BUDGET: Duration = Duration::from_millis(3_000);

const LAUNCH_BINDING_BYTES: usize = 128;
const DECISION_IDENTITY_BYTES: usize = 64;
const RECEIPT_RETAINED_BYTES: usize =
    OPERATION_KEY_BYTES + 33 + 8 + 1 + 16 + 8 + 8 + DECISION_IDENTITY_BYTES + 8 + 8;
const REGISTRATION_REJECTED: &str = "OTN01";
const COMMAND_VERSION: u8 = 1;
const STATE_ASSIGNED: i16 = 1;
const STATE_REVOKED: i16 = 2;
const CHANNEL_SCOPE_TAG: u8 = 1;
const _: () = assert!(MAX_COMMAND_BYTES + RECEIPT_RETAINED_BYTES <= MAX_INFLIGHT_BYTES);
const _: () = assert!(MAX_PENDING_COMMANDS * MAX_COMMAND_BYTES <= MAX_PENDING_BYTES);

// ---------------------------------------------------------------------------
// Process-incarnation registration
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum RegistrationError {
    /// Missing/malformed/replayed/changed authorization, colliding NodeId or unknown target.
    Rejected,
    /// The exact incarnation is not the current registration.
    NotCurrent,
    /// Authority state is unavailable or ambiguous; fail closed.
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for RegistrationError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

/// One-launch bootstrap secret. Only its SHA-256 digest is ever retained.
#[derive(Clone, PartialEq, Eq)]
pub struct BootstrapSecret([u8; 32]);

impl BootstrapSecret {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl fmt::Debug for BootstrapSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BootstrapSecret(<redacted>)")
    }
}

/// Operator-issued launch identity bound to exactly one authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchBinding(String);

impl LaunchBinding {
    pub fn new(value: &str) -> Result<Self, RegistrationError> {
        if valid_token(value, LAUNCH_BINDING_BYTES) {
            Ok(Self(value.to_owned()))
        } else {
            Err(RegistrationError::Rejected)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Sealed current-registration fact: exact NodeId plus its writer revision.
/// Holding a fact is not authority; every consumer revalidates it durably.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeRegistrationFact {
    node_id: NodeId,
    registration_revision: u64,
}

impl NodeRegistrationFact {
    #[must_use]
    pub const fn new(node_id: NodeId, registration_revision: u64) -> Self {
        Self {
            node_id,
            registration_revision,
        }
    }

    #[must_use]
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    #[must_use]
    pub const fn registration_revision(&self) -> u64 {
        self.registration_revision
    }
}

/// Process-held proof of one current incarnation: its sealed registration fact
/// plus the launch secret it registered with. Only this process holds the
/// secret; the database retains only its digest. A public NodeId/revision
/// alone never proves currentness to a fenced writer.
#[derive(Clone, PartialEq, Eq)]
pub struct NodeIncarnationProof {
    fact: NodeRegistrationFact,
    secret: BootstrapSecret,
}

impl NodeIncarnationProof {
    #[must_use]
    pub const fn new(fact: NodeRegistrationFact, secret: BootstrapSecret) -> Self {
        Self { fact, secret }
    }

    #[must_use]
    pub const fn fact(&self) -> NodeRegistrationFact {
        self.fact
    }
}

impl fmt::Debug for NodeIncarnationProof {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NodeIncarnationProof")
            .field("fact", &self.fact)
            .field("secret", &"<redacted>")
            .finish()
    }
}

impl DurabilityRoot {
    /// Control-plane: durably record one launch-scoped bootstrap authorization.
    pub async fn issue_node_bootstrap_authorization(
        &self,
        secret: &BootstrapSecret,
        launch: &LaunchBinding,
        supersedes: Option<NodeId>,
    ) -> Result<(), RegistrationError> {
        let secret = secret.0;
        let launch = launch.0.clone();
        let supersedes = supersedes.map(|node| node.as_bytes().to_vec());
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    if let Some(node) = &supersedes {
                        let known: bool = sqlx::query_scalar(
                            "SELECT EXISTS (SELECT 1 FROM game_node_registrations \
                             WHERE node_id = encode($1, 'hex')::uuid)",
                        )
                        .bind(node)
                        .fetch_one(&mut *tx)
                        .await?;
                        if !known {
                            return Ok(Err(RegistrationError::Rejected));
                        }
                    }
                    let inserted = sqlx::query(
                        "INSERT INTO game_node_bootstrap_authorizations \
                         (authorization_digest, launch_binding, supersedes_node_id, issued_at) \
                         VALUES (sha256($1), $2, encode($3::bytea, 'hex')::uuid, \
                                 floor(extract(epoch FROM statement_timestamp()))::bigint) \
                         ON CONFLICT DO NOTHING",
                    )
                    .bind(secret.as_slice())
                    .bind(&launch)
                    .bind(supersedes.as_deref())
                    .execute(&mut *tx)
                    .await?
                    .rows_affected();
                    if inserted != 1 {
                        // Exact replay after a lost response: the identical,
                        // unrevoked row already exists. Any difference rejects.
                        let exact: bool = sqlx::query_scalar(
                            "SELECT EXISTS (SELECT 1 FROM game_node_bootstrap_authorizations a \
                             WHERE a.authorization_digest = sha256($1) AND a.launch_binding = $2 \
                               AND a.supersedes_node_id IS NOT DISTINCT FROM encode($3::bytea, 'hex')::uuid \
                               AND NOT EXISTS (SELECT 1 FROM game_node_bootstrap_authorization_revocations r \
                                               WHERE r.authorization_digest = a.authorization_digest))",
                        )
                        .bind(secret.as_slice())
                        .bind(&launch)
                        .bind(supersedes.as_deref())
                        .fetch_one(&mut *tx)
                        .await?;
                        if !exact {
                            return Ok(Err(RegistrationError::Rejected));
                        }
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }

    /// Control-plane: revoke one abandoned, unconsumed launch authorization,
    /// identified by its retained secret. Idempotent; a consumed or unknown
    /// authorization rejects (end a registration through
    /// `revoke_node_registration` instead).
    pub async fn revoke_node_bootstrap_authorization(
        &self,
        secret: &BootstrapSecret,
    ) -> Result<(), RegistrationError> {
        let secret = secret.0;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    match sqlx::query("SELECT game_node_revoke_bootstrap_authorization($1)")
                        .bind(secret.as_slice())
                        .execute(&mut *tx)
                        .await
                    {
                        Ok(_) => {}
                        Err(error) if has_sql_state(&error, REGISTRATION_REJECTED) => {
                            return Ok(Err(RegistrationError::Rejected));
                        }
                        Err(error) => return Err(error.into()),
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }

    /// GameNode: consume the launch authorization and bind this exact incarnation.
    /// Replaying the identical request after a lost response returns the
    /// original result without consuming another authorization.
    pub async fn register_node_incarnation(
        &self,
        secret: &BootstrapSecret,
        launch: &LaunchBinding,
        node_id: NodeId,
    ) -> Result<NodeIncarnationProof, RegistrationError> {
        let proof_secret = secret.clone();
        let secret = secret.0;
        let launch = launch.0.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let registered = sqlx::query_scalar::<_, String>(
                        "SELECT game_node_register($1, $2, encode($3, 'hex')::uuid)::text",
                    )
                    .bind(secret.as_slice())
                    .bind(&launch)
                    .bind(node_id.as_bytes().as_slice())
                    .fetch_one(&mut *tx)
                    .await;
                    let revision = match registered {
                        Ok(revision) => parse_u64_text(&revision)?,
                        Err(error) if has_sql_state(&error, REGISTRATION_REJECTED) => {
                            return Ok(Err(RegistrationError::Rejected));
                        }
                        Err(error) => return Err(error.into()),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(NodeIncarnationProof::new(
                        NodeRegistrationFact::new(node_id, revision),
                        proof_secret,
                    )))
                })
            })
            .await?
    }

    /// Serialized proof that the caller is the exact current incarnation.
    pub async fn require_current_node_registration(
        &self,
        proof: &NodeIncarnationProof,
    ) -> Result<(), RegistrationError> {
        let proof = proof.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let current = prove_current_incarnation(&mut tx, &proof).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(if current {
                        Ok(())
                    } else {
                        Err(RegistrationError::NotCurrent)
                    })
                })
            })
            .await?
    }

    /// Control-plane: explicitly revoke one exact incarnation. Idempotent for an
    /// already revoked incarnation; never makes an ended registration current.
    pub async fn revoke_node_registration(
        &self,
        fact: NodeRegistrationFact,
    ) -> Result<(), RegistrationError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    // Writer-then-registration lock order, as in registration,
                    // currentness checks and game_node_end_registration.
                    sqlx::query(
                        "SELECT 1 FROM game_node_registration_writer WHERE writer_id = 1 FOR UPDATE",
                    )
                    .execute(&mut *tx)
                    .await?;
                    let state: Option<i16> = sqlx::query_scalar(
                        "SELECT state FROM game_node_registrations \
                         WHERE node_id = encode($1, 'hex')::uuid \
                           AND registration_revision = $2::text::numeric(20,0) FOR UPDATE",
                    )
                    .bind(fact.node_id.as_bytes().as_slice())
                    .bind(fact.registration_revision.to_string())
                    .fetch_optional(&mut *tx)
                    .await?;
                    match state {
                        Some(1) => {
                            // The single ending path allocates a writer revision
                            // and records the immutable ending with the row.
                            let ended: bool = sqlx::query_scalar(
                                "SELECT game_node_end_registration(encode($1, 'hex')::uuid, \
                                 $2::text::numeric(20,0), 2::smallint, NULL)",
                            )
                            .bind(fact.node_id.as_bytes().as_slice())
                            .bind(fact.registration_revision.to_string())
                            .fetch_one(&mut *tx)
                            .await?;
                            if !ended {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                        }
                        Some(2) => {}
                        Some(3) | None => return Ok(Err(RegistrationError::Rejected)),
                        Some(_) => return Err(DurabilityError::InvalidStoredState),
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }
}

/// DB-visible current-incarnation primitive for fenced writers (for example S2).
/// Proves possession of the incarnation secret against the retained digest and
/// holds a share lock on the exact current registration until `transaction`
/// ends, so a concurrent revoke/supersede serializes with the caller's mutation.
pub(crate) async fn prove_current_incarnation(
    transaction: &mut Transaction<'_, Postgres>,
    proof: &NodeIncarnationProof,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT game_node_prove_current_incarnation(encode($1, 'hex')::uuid, $2::text::numeric(20,0), $3)",
    )
    .bind(proof.fact.node_id.as_bytes().as_slice())
    .bind(proof.fact.registration_revision.to_string())
    .bind(proof.secret.0.as_slice())
    .fetch_one(&mut **transaction)
    .await?)
}

/// Control-plane currentness check of an assignment target (no possession).
async fn lock_current_registration(
    transaction: &mut Transaction<'_, Postgres>,
    fact: NodeRegistrationFact,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT game_node_lock_current_registration(encode($1, 'hex')::uuid, $2::text::numeric(20,0))",
    )
    .bind(fact.node_id.as_bytes().as_slice())
    .bind(fact.registration_revision.to_string())
    .fetch_one(&mut **transaction)
    .await?)
}

// ---------------------------------------------------------------------------
// Runtime-scope assignment
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum AssignmentError {
    /// Instance scopes are not allocated in the first slice.
    Unsupported,
    /// Input violates the closed typed/NASG bounds; nothing was retained.
    InvalidInput,
    /// `NASG-QUEUE` is full; rejected before retention.
    QueueFull,
    /// `NASG-EXECUTION` queue wait elapsed before dispatch.
    QueueTimeout,
    /// The single in-flight slot holds an unreconciled operation.
    ReconcileRequired,
    /// The dispatched operation's outcome is unknown; its slot is retained.
    Ambiguous,
    /// The caller is not the attested current holder of the assigned scope.
    NotCurrentHolder,
    /// Authority state is unavailable; nothing was submitted.
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for AssignmentError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

/// Authority-scoped 256-bit idempotency identity bound to one exact command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationKey([u8; OPERATION_KEY_BYTES]);

impl OperationKey {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; OPERATION_KEY_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn from_text(text: &str) -> Result<Self, AssignmentError> {
        if text.len() != OPERATION_KEY_TEXT_BYTES {
            return Err(AssignmentError::InvalidInput);
        }
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(text)
            .map_err(|_| AssignmentError::InvalidInput)?;
        let bytes: [u8; OPERATION_KEY_BYTES] = decoded
            .try_into()
            .map_err(|_| AssignmentError::InvalidInput)?;
        let key = Self(bytes);
        if key.to_text() != text {
            return Err(AssignmentError::InvalidInput);
        }
        Ok(key)
    }

    #[must_use]
    pub fn to_text(self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.0)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; OPERATION_KEY_BYTES] {
        &self.0
    }
}

/// Independently authenticated control actor recorded in the command binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlActor(String);

impl ControlActor {
    pub fn new(value: &str) -> Result<Self, AssignmentError> {
        if valid_token(value, MAX_IDENTITY_BYTES) {
            Ok(Self(value.to_owned()))
        } else {
            Err(AssignmentError::InvalidInput)
        }
    }
}

/// Exact predecessor CAS: the committed generation, writer source revision and
/// the Runtime guard publication binding (`None` when the scope has no guard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentPredecessor {
    pub ownership_generation: u64,
    pub source_revision: u64,
    pub runtime_guard_publication_revision: Option<u64>,
}

/// Closed first-slice command family. Callers cannot submit writer revisions,
/// decision identities or generations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentCommand {
    Assign {
        scope: RuntimeScopeRefV1,
        target: NodeRegistrationFact,
    },
    Replace {
        scope: RuntimeScopeRefV1,
        predecessor: AssignmentPredecessor,
        target: NodeRegistrationFact,
    },
    Revoke {
        scope: RuntimeScopeRefV1,
        predecessor: AssignmentPredecessor,
    },
}

impl AssignmentCommand {
    const fn scope(&self) -> RuntimeScopeRefV1 {
        match self {
            Self::Assign { scope, .. }
            | Self::Replace { scope, .. }
            | Self::Revoke { scope, .. } => *scope,
        }
    }

    const fn target(&self) -> Option<NodeRegistrationFact> {
        match self {
            Self::Assign { target, .. } | Self::Replace { target, .. } => Some(*target),
            Self::Revoke { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentRequest {
    pub operation_key: OperationKey,
    pub actor: ControlActor,
    pub command: AssignmentCommand,
}

impl AssignmentRequest {
    /// Canonical closed encoding retained in the slot and receipt (`NASG-COMMAND-BYTES`).
    pub fn encode(&self) -> Result<Vec<u8>, AssignmentError> {
        let (world_id, channel_id) = channel_scope(self.command.scope())?;
        if !valid_token(&self.actor.0, MAX_IDENTITY_BYTES) {
            return Err(AssignmentError::InvalidInput);
        }
        let mut encoded = Vec::with_capacity(256);
        encoded.push(COMMAND_VERSION);
        encoded.push(match self.command {
            AssignmentCommand::Assign { .. } => 1,
            AssignmentCommand::Replace { .. } => 2,
            AssignmentCommand::Revoke { .. } => 3,
        });
        encoded.extend_from_slice(&self.operation_key.0);
        let actor_len =
            u8::try_from(self.actor.0.len()).map_err(|_| AssignmentError::InvalidInput)?;
        encoded.push(actor_len);
        encoded.extend_from_slice(self.actor.0.as_bytes());
        encoded.extend_from_slice(&scope_key(world_id, channel_id));
        match self.command {
            AssignmentCommand::Replace { predecessor, .. }
            | AssignmentCommand::Revoke { predecessor, .. } => {
                if predecessor.ownership_generation == 0 || predecessor.source_revision == 0 {
                    return Err(AssignmentError::InvalidInput);
                }
                encoded.extend_from_slice(&predecessor.ownership_generation.to_be_bytes());
                encoded.extend_from_slice(&predecessor.source_revision.to_be_bytes());
                match predecessor.runtime_guard_publication_revision {
                    None => encoded.push(0),
                    Some(0) => return Err(AssignmentError::InvalidInput),
                    Some(revision) => {
                        encoded.push(1);
                        encoded.extend_from_slice(&revision.to_be_bytes());
                    }
                }
            }
            AssignmentCommand::Assign { .. } => {}
        }
        if let Some(target) = self.command.target() {
            if target.registration_revision == 0 {
                return Err(AssignmentError::InvalidInput);
            }
            encoded.extend_from_slice(target.node_id.as_bytes());
            encoded.extend_from_slice(&target.registration_revision.to_be_bytes());
        }
        if encoded.len() > MAX_COMMAND_BYTES {
            return Err(AssignmentError::InvalidInput);
        }
        Ok(encoded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentState {
    Assigned,
    Revoked,
}

/// Authoritative committed assignment for one Channel scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScopeAssignment {
    pub scope: RuntimeScopeRefV1,
    pub ownership_generation: u64,
    pub state: AssignmentState,
    pub holder: Option<NodeRegistrationFact>,
    pub source_revision: u64,
    pub decision_identity: String,
    pub decided_at: i64,
}

/// Immutable committed receipt for one operation identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentReceipt {
    pub operation_key: OperationKey,
    pub assignment: RuntimeScopeAssignment,
    /// Runtime guard publication revision written as the `ready = false` fence
    /// (present whenever the scope has a Runtime guard).
    pub fenced_publication_revision: Option<u64>,
}

impl AssignmentReceipt {
    /// Exact CAS predecessor as committed by this decision.
    #[must_use]
    pub const fn predecessor(&self) -> AssignmentPredecessor {
        AssignmentPredecessor {
            ownership_generation: self.assignment.ownership_generation,
            source_revision: self.assignment.source_revision,
            runtime_guard_publication_revision: self.fenced_publication_revision,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentRejection {
    PredecessorMismatch,
    NotAssigned,
    TargetNotCurrent,
    OperationConflict,
    GenerationExhausted,
    SourceRevisionExhausted,
    /// The authenticated session role is not the recorded actor, or holds no
    /// exact-scope control grant for this operation (OPS-NODE-BOOT-01 D2).
    NotGranted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentOutcome {
    Committed(AssignmentReceipt),
    Rejected(AssignmentRejection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileOutcome {
    Committed(AssignmentReceipt),
    /// The operation key committed a different exact command.
    Conflict,
    Absent,
}

impl DurabilityRoot {
    /// Consumer read of the current committed assignment. It never creates or
    /// advances authority and fails closed on regressed writer state.
    pub async fn read_runtime_scope_assignment(
        &self,
        scope: RuntimeScopeRefV1,
    ) -> Result<Option<RuntimeScopeAssignment>, AssignmentError> {
        let (world_id, channel_id) = channel_scope(scope)?;
        let key = scope_key(world_id, channel_id);
        Ok(self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let high_water = writer_high_water(&mut tx, false).await?;
                    require_history_matches_high_water(&mut tx, high_water).await?;
                    let current = load_assignment(&mut tx, &key, false).await?;
                    if current
                        .as_ref()
                        .is_some_and(|current| current.source_revision > high_water)
                    {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(current)
                })
            })
            .await?)
    }

    /// Current exact CAS predecessor for a replace/revoke: the authoritative
    /// assignment plus the Runtime guard publication binding, read together.
    pub async fn read_runtime_scope_predecessor(
        &self,
        scope: RuntimeScopeRefV1,
    ) -> Result<Option<AssignmentPredecessor>, AssignmentError> {
        let (world_id, channel_id) = channel_scope(scope)?;
        let key = scope_key(world_id, channel_id);
        let root = self.clone();
        Ok(self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    lock_admission_relations(&mut tx).await?;
                    let high_water = writer_high_water(&mut tx, false).await?;
                    require_history_matches_high_water(&mut tx, high_water).await?;
                    let Some(current) = load_assignment(&mut tx, &key, false).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(None);
                    };
                    require_guard_covers_latest_fence(&root, &mut tx, &key, scope).await?;
                    let guard = runtime_guard_publication_revision(&root, &mut tx, scope).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(AssignmentPredecessor {
                        ownership_generation: current.ownership_generation,
                        source_revision: current.source_revision,
                        runtime_guard_publication_revision: guard,
                    }))
                })
            })
            .await?)
    }
}

impl DurabilityRoot {
    /// Runtime readiness publication for an assigned Channel scope. The caller
    /// proves it is the exact current holder incarnation; the proof, the guard
    /// CAS and the readiness trigger's fence commit in one transaction. A
    /// replaced process holding only public NodeId/generation facts is refused.
    pub async fn publish_runtime_readiness(
        &self,
        proof: &NodeIncarnationProof,
        publication: &AdmissionAuthorityPublicationV1,
    ) -> Result<GuardPublicationDisposition, AssignmentError> {
        let [change] = publication.changes() else {
            return Err(AssignmentError::InvalidInput);
        };
        let AdmissionAuthorityGuardKeyV1::Runtime(scope) = change.key else {
            return Err(AssignmentError::InvalidInput);
        };
        let (world_id, channel_id) = channel_scope(scope)?;
        let key = scope_key(world_id, channel_id);
        let encoded = encode_guard(change, MAX_ADMISSION_GUARD_BYTES)?;
        let proof = proof.clone();
        let publication = publication.clone();
        let store = AdmissionGuardStore::from_root(self.clone());
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    lock_admission_relations(&mut tx).await?;
                    let attested: bool = sqlx::query_scalar(
                        "SELECT game_runtime_attest_readiness($1, encode($2, 'hex')::uuid, \
                                $3::text::numeric(20,0), $4)",
                    )
                    .bind(key.as_slice())
                    .bind(proof.fact.node_id.as_bytes().as_slice())
                    .bind(proof.fact.registration_revision.to_string())
                    .bind(proof.secret.0.as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    if !attested {
                        return Ok(Err(AssignmentError::NotCurrentHolder));
                    }
                    let changes = publication.changes();
                    let current = [store.load_locked(&mut tx, &changes[0].key).await?];
                    let disposition = match publication.validate_locked(&current) {
                        Err(AdmissionAuthorityPublicationErrorV1::Stale) => {
                            GuardPublicationDisposition::Stale
                        }
                        Err(_) => GuardPublicationDisposition::Conflict,
                        Ok(()) if current[0].as_ref() == Some(&changes[0]) => {
                            GuardPublicationDisposition::Existing
                        }
                        Ok(())
                            if !store
                                .successor_history_available(&mut tx, changes, &current)
                                .await? =>
                        {
                            GuardPublicationDisposition::Conflict
                        }
                        Ok(()) => {
                            store
                                .persist_locked(
                                    &mut tx,
                                    changes,
                                    &current,
                                    std::slice::from_ref(&encoded),
                                )
                                .await?;
                            GuardPublicationDisposition::Applied
                        }
                    };
                    sqlx::query(
                        "DELETE FROM game_runtime_readiness_attestations \
                         WHERE attested_xact = pg_current_xact_id()",
                    )
                    .execute(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(disposition))
                })
            })
            .await?
    }
}

struct QueueState {
    pending_commands: usize,
    pending_bytes: usize,
    unreconciled: Option<OperationKey>,
}

/// NASG accounting shared by every handle of one logical writer registration
/// on one process root, so duplicate opens cannot multiply the bounds.
struct WriterShared {
    queue: Arc<Mutex<QueueState>>,
    inflight: Arc<Semaphore>,
}

type WriterRegistry = Mutex<HashMap<(usize, Arc<str>), Weak<WriterShared>>>;

fn writer_registry() -> &'static WriterRegistry {
    static REGISTRY: OnceLock<WriterRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Process-lifetime root-to-registration binding. It is never released when
/// handles drop, so a root's durable slot custody cannot be bypassed by
/// opening another registration name after the last handle is gone.
type WriterBindings = Mutex<HashMap<usize, (RootLiveness, Arc<str>)>>;
type RootLiveness = Weak<dyn std::any::Any + Send + Sync>;

fn writer_bindings() -> &'static WriterBindings {
    static BINDINGS: OnceLock<WriterBindings> = OnceLock::new();
    BINDINGS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Single logical assignment-writer registration with the NASG queue and one
/// end-to-end in-flight slot whose exact binding is durably checkpointed.
#[derive(Clone)]
pub struct RuntimeScopeAssignmentWriter {
    root: DurabilityRoot,
    registration: Arc<str>,
    _shared: Arc<WriterShared>,
    queue: Arc<Mutex<QueueState>>,
    inflight: Arc<Semaphore>,
}

struct PendingReservation {
    queue: Arc<Mutex<QueueState>>,
    bytes: usize,
}

impl Drop for PendingReservation {
    fn drop(&mut self) {
        let mut queue = lock_queue(&self.queue);
        queue.pending_commands = queue.pending_commands.saturating_sub(1);
        queue.pending_bytes = queue.pending_bytes.saturating_sub(self.bytes);
    }
}

enum Dispatch {
    Outcome(AssignmentOutcome),
    Definitive(AssignmentError),
    Ambiguous,
}

impl RuntimeScopeAssignmentWriter {
    /// Restore durable slot custody before admitting any work. An occupied slot
    /// from a prior incarnation must be reconciled first.
    pub async fn open(
        root: DurabilityRoot,
        writer_registration: &str,
    ) -> Result<Self, AssignmentError> {
        if !valid_token(writer_registration, MAX_IDENTITY_BYTES) {
            return Err(AssignmentError::InvalidInput);
        }
        let registration: Arc<str> = Arc::from(writer_registration);
        {
            // NASG accounting is per assignment-writer process: one writer
            // registration per root for the root's lifetime.
            let mut bindings = writer_bindings()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            bindings.retain(|_, (liveness, _)| liveness.strong_count() > 0);
            let (_, bound) = bindings
                .entry(root.root_identity())
                .or_insert_with(|| (root.root_liveness(), registration.clone()));
            if **bound != *registration {
                return Err(AssignmentError::Unsupported);
            }
        }
        let restore = registration.clone();
        let occupied = root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    sqlx::query(
                        "INSERT INTO game_runtime_scope_assignment_slots (writer_registration) \
                         VALUES ($1) ON CONFLICT DO NOTHING",
                    )
                    .bind(&*restore)
                    .execute(&mut *tx)
                    .await?;
                    let occupied: Option<Vec<u8>> = sqlx::query_scalar(
                        "SELECT operation_key FROM game_runtime_scope_assignment_slots \
                         WHERE writer_registration = $1",
                    )
                    .bind(&*restore)
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    occupied
                        .map(|key| operation_key_from_stored(&key))
                        .transpose()
                })
            })
            .await?;
        let shared = {
            let mut registry = writer_registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            registry.retain(|_, shared| shared.strong_count() > 0);
            let key = (root.root_identity(), registration.clone());
            match registry.get(&key).and_then(Weak::upgrade) {
                Some(shared) => {
                    let mut queue = lock_queue(&shared.queue);
                    if queue.unreconciled.is_none() {
                        queue.unreconciled = occupied;
                    }
                    drop(queue);
                    shared
                }
                None => {
                    let shared = Arc::new(WriterShared {
                        queue: Arc::new(Mutex::new(QueueState {
                            pending_commands: 0,
                            pending_bytes: 0,
                            unreconciled: occupied,
                        })),
                        inflight: Arc::new(Semaphore::new(1)),
                    });
                    registry.insert(key, Arc::downgrade(&shared));
                    shared
                }
            }
        };
        Ok(Self {
            root,
            registration,
            queue: shared.queue.clone(),
            inflight: shared.inflight.clone(),
            _shared: shared,
        })
    }

    /// The operation identity whose slot must be reconciled before new work.
    #[must_use]
    pub fn unreconciled(&self) -> Option<OperationKey> {
        lock_queue(&self.queue).unreconciled
    }

    pub async fn submit(
        &self,
        request: &AssignmentRequest,
    ) -> Result<AssignmentOutcome, AssignmentError> {
        let command = request.encode()?;
        let _permit = self.admit(command.len(), None).await?;
        let key = request.operation_key;
        let scope = request.command.scope();
        let target = request.command.target();
        let dispatched = tokio::time::timeout(
            DISPATCH_BUDGET,
            self.dispatch(
                key,
                command,
                request.actor.clone(),
                request.command,
                scope,
                target,
            ),
        )
        .await;
        match dispatched {
            Ok(Dispatch::Outcome(outcome)) => Ok(outcome),
            Ok(Dispatch::Definitive(error)) => Err(error),
            Ok(Dispatch::Ambiguous) | Err(_) => {
                lock_queue(&self.queue).unreconciled = Some(key);
                Err(AssignmentError::Ambiguous)
            }
        }
    }

    /// Authoritative reconciliation of one exact operation (key and canonical
    /// command) under the slot lock. The outcome is a deterministic function of
    /// the immutable receipt and the caller's exact command, so a lost response
    /// can be reconciled again with the same answer after custody is cleared.
    pub async fn reconcile(
        &self,
        request: &AssignmentRequest,
    ) -> Result<ReconcileOutcome, AssignmentError> {
        let command = request.encode()?;
        let key = request.operation_key;
        let _permit = self.admit(command.len(), Some(key)).await?;
        let registration = self.registration.clone();
        let pass = self.root.try_issue_semantic_pass()?;
        let reconciled = tokio::time::timeout(
            DISPATCH_BUDGET,
            pass.run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    lock_admission_relations(&mut tx).await?;
                    let slot = lock_slot(&mut tx, &registration).await?;
                    match &slot {
                        // Custody answers only for its own exact binding.
                        Some((slot_key, slot_command))
                            if slot_key == key.0.as_slice() && *slot_command == command => {}
                        Some(_) => return Ok(Err(AssignmentError::ReconcileRequired)),
                        None => {}
                    }
                    // Only a fully retained authority history can prove the
                    // absence of a receipt; a regressed one fails closed.
                    let high_water = writer_high_water(&mut tx, true).await?;
                    require_history_matches_high_water(&mut tx, high_water).await?;
                    let outcome = match load_receipt(&mut tx, &key).await? {
                        Some((receipt, stored)) if stored == command => {
                            ReconcileOutcome::Committed(receipt)
                        }
                        Some(_) => ReconcileOutcome::Conflict,
                        None => ReconcileOutcome::Absent,
                    };
                    if slot.is_some() {
                        clear_slot(&mut tx, &registration).await?;
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(outcome))
                })
            }),
        )
        .await;
        let outcome = match reconciled {
            Ok(Ok(Ok(outcome))) => outcome,
            Ok(Ok(Err(error))) => return Err(error),
            Ok(Err(_)) | Err(_) => return Err(AssignmentError::Ambiguous),
        };
        let mut queue = lock_queue(&self.queue);
        if queue.unreconciled == Some(key) {
            queue.unreconciled = None;
        }
        Ok(outcome)
    }

    async fn admit(
        &self,
        bytes: usize,
        reconciling: Option<OperationKey>,
    ) -> Result<OwnedSemaphorePermit, AssignmentError> {
        let reservation = {
            let mut queue = lock_queue(&self.queue);
            admissible(&queue, reconciling)?;
            let pending_bytes = queue
                .pending_bytes
                .checked_add(bytes)
                .ok_or(AssignmentError::QueueFull)?;
            if queue.pending_commands >= MAX_PENDING_COMMANDS || pending_bytes > MAX_PENDING_BYTES {
                return Err(AssignmentError::QueueFull);
            }
            queue.pending_commands += 1;
            queue.pending_bytes = pending_bytes;
            PendingReservation {
                queue: self.queue.clone(),
                bytes,
            }
        };
        let acquired =
            tokio::time::timeout(QUEUE_WAIT, self.inflight.clone().acquire_owned()).await;
        drop(reservation);
        let permit = match acquired {
            Ok(Ok(permit)) => permit,
            Ok(Err(_)) => {
                return Err(AssignmentError::Unavailable(
                    DurabilityError::RootUnavailable,
                ));
            }
            Err(_) => return Err(AssignmentError::QueueTimeout),
        };
        admissible(&lock_queue(&self.queue), reconciling)?;
        Ok(permit)
    }

    async fn dispatch(
        &self,
        key: OperationKey,
        command: Vec<u8>,
        actor: ControlActor,
        typed: AssignmentCommand,
        scope: RuntimeScopeRefV1,
        target: Option<NodeRegistrationFact>,
    ) -> Dispatch {
        // Checkpoint the exact binding before authoritative submission.
        let pass = match self.root.try_issue_semantic_pass() {
            Ok(pass) => pass,
            Err(error) => return Dispatch::Definitive(error.into()),
        };
        let registration = self.registration.clone();
        let checkpoint_command = command.clone();
        let checkpointed = pass
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let changed = sqlx::query(
                        "UPDATE game_runtime_scope_assignment_slots SET operation_key = $2, command = $3, \
                         checkpointed_at = floor(extract(epoch FROM statement_timestamp()))::bigint \
                         WHERE writer_registration = $1 AND operation_key IS NULL",
                    )
                    .bind(&*registration)
                    .bind(key.0.as_slice())
                    .bind(&checkpoint_command)
                    .execute(&mut *tx)
                    .await?
                    .rows_affected();
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(changed == 1)
                })
            })
            .await;
        match checkpointed {
            Ok(true) => {}
            Ok(false) => return Dispatch::Definitive(AssignmentError::ReconcileRequired),
            Err(_) => return Dispatch::Ambiguous,
        }

        let pass = match self.root.try_issue_semantic_pass() {
            Ok(pass) => pass,
            Err(_) => return Dispatch::Ambiguous,
        };
        let root = self.root.clone();
        let registration = self.registration.clone();
        let outcome = pass
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let outcome = authoritative_transition(
                        &root,
                        &mut tx,
                        &registration,
                        key,
                        &command,
                        &actor,
                        typed,
                        scope,
                        target,
                    )
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(outcome)
                })
            })
            .await;
        match outcome {
            Ok(outcome) => Dispatch::Outcome(outcome),
            Err(_) => Dispatch::Ambiguous,
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn authoritative_transition(
    root: &DurabilityRoot,
    tx: &mut Transaction<'_, Postgres>,
    registration: &str,
    key: OperationKey,
    command: &[u8],
    actor: &ControlActor,
    typed: AssignmentCommand,
    scope: RuntimeScopeRefV1,
    target: Option<NodeRegistrationFact>,
) -> Result<AssignmentOutcome, DurabilityError> {
    let (world_id, channel_id) =
        channel_scope(scope).map_err(|_| DurabilityError::InvalidStoredState)?;
    let scope_key = scope_key(world_id, channel_id);
    lock_admission_relations(tx).await?;
    match lock_slot(tx, registration).await? {
        Some((slot_key, slot_command))
            if slot_key == key.0.as_slice() && slot_command == command => {}
        // Custody was reconciled or taken elsewhere; this submission must not proceed.
        _ => return Err(DurabilityError::Unavailable),
    }
    if let Some((receipt, stored_command)) = load_receipt(tx, &key).await? {
        clear_slot(tx, registration).await?;
        return Ok(if stored_command == command {
            AssignmentOutcome::Committed(receipt)
        } else {
            AssignmentOutcome::Rejected(AssignmentRejection::OperationConflict)
        });
    }
    // Exact-scope authorization of the authenticated session role, which must
    // also be the recorded actor. Group membership alone never authorizes.
    let operation: i16 = match typed {
        AssignmentCommand::Assign { .. } => 1,
        AssignmentCommand::Replace { .. } => 2,
        AssignmentCommand::Revoke { .. } => 3,
    };
    let granted: bool = sqlx::query_scalar(
        "SELECT $1 = session_user AND EXISTS (SELECT 1 FROM game_control_scope_grants \
         WHERE control_role = session_user AND world_id = encode($2, 'hex')::uuid \
           AND channel_id = encode($3, 'hex')::uuid AND operation = $4)",
    )
    .bind(actor.0.as_str())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(operation)
    .fetch_one(&mut **tx)
    .await?;
    if !granted {
        clear_slot(tx, registration).await?;
        return Ok(AssignmentOutcome::Rejected(AssignmentRejection::NotGranted));
    }
    let high_water = writer_high_water(tx, true).await?;
    require_history_matches_high_water(tx, high_water).await?;
    let current = load_assignment(tx, &scope_key, true).await?;
    // Admission relations are locked, so the guard binding cannot move here.
    require_guard_covers_latest_fence(root, tx, &scope_key, scope).await?;
    let guard_revision = runtime_guard_publication_revision(root, tx, scope).await?;
    let matches = |current: &RuntimeScopeAssignment, predecessor: AssignmentPredecessor| {
        current.ownership_generation == predecessor.ownership_generation
            && current.source_revision == predecessor.source_revision
            && guard_revision == predecessor.runtime_guard_publication_revision
    };
    let rejection = match (typed, &current) {
        (AssignmentCommand::Assign { .. }, None) => None,
        (AssignmentCommand::Assign { .. }, Some(_)) => {
            Some(AssignmentRejection::PredecessorMismatch)
        }
        (AssignmentCommand::Replace { predecessor, .. }, Some(current))
            if matches(current, predecessor) =>
        {
            None
        }
        (AssignmentCommand::Revoke { predecessor, .. }, Some(current))
            if matches(current, predecessor) =>
        {
            (current.state != AssignmentState::Assigned).then_some(AssignmentRejection::NotAssigned)
        }
        (AssignmentCommand::Replace { .. } | AssignmentCommand::Revoke { .. }, _) => {
            Some(AssignmentRejection::PredecessorMismatch)
        }
    };
    let mut rejection = rejection;
    if let (None, Some(target)) = (rejection, target)
        && !lock_current_registration(tx, target).await?
    {
        rejection = Some(AssignmentRejection::TargetNotCurrent);
    }
    let source_revision = successor_source_revision(high_water);
    let generation = match &current {
        // A pre-existing Runtime guard never sees its generation regress.
        None => Some(
            runtime_guard_generation(root, tx, scope)
                .await?
                .unwrap_or(1)
                .max(1),
        ),
        Some(current) => current.ownership_generation.checked_add(1),
    };
    let rejection = rejection.or_else(|| source_revision.err()).or_else(|| {
        generation
            .is_none()
            .then_some(AssignmentRejection::GenerationExhausted)
    });
    let (Ok(source_revision), Some(ownership_generation), None) =
        (source_revision, generation, rejection)
    else {
        clear_slot(tx, registration).await?;
        return Ok(AssignmentOutcome::Rejected(
            rejection.unwrap_or(AssignmentRejection::SourceRevisionExhausted),
        ));
    };
    let decision_identity = format!("runtime-scope-assignment:{source_revision}");
    let decided_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let (state, holder) = match typed {
        AssignmentCommand::Assign { target, .. } | AssignmentCommand::Replace { target, .. } => {
            (AssignmentState::Assigned, Some(target))
        }
        AssignmentCommand::Revoke { .. } => (AssignmentState::Revoked, None),
    };
    let assignment = RuntimeScopeAssignment {
        scope,
        ownership_generation,
        state,
        holder,
        source_revision,
        decision_identity,
        decided_at,
    };
    let stored_state = match state {
        AssignmentState::Assigned => STATE_ASSIGNED,
        AssignmentState::Revoked => STATE_REVOKED,
    };
    let holder_node = holder.map(|holder| holder.node_id.as_bytes().to_vec());
    let holder_revision = holder.map(|holder| holder.registration_revision.to_string());
    sqlx::query(
        "INSERT INTO game_runtime_scope_assignments \
         (scope_key, world_id, channel_id, ownership_generation, state, holder_node_id, \
          holder_registration_revision, source_revision, decision_identity, operation_key, decided_at) \
         VALUES ($1, encode($2, 'hex')::uuid, encode($3, 'hex')::uuid, $4::text::numeric(20,0), $5, \
                 encode($6::bytea, 'hex')::uuid, $7::text::numeric(20,0), $8::text::numeric(20,0), $9, $10, $11) \
         ON CONFLICT (scope_key) DO UPDATE SET ownership_generation = EXCLUDED.ownership_generation, \
             state = EXCLUDED.state, holder_node_id = EXCLUDED.holder_node_id, \
             holder_registration_revision = EXCLUDED.holder_registration_revision, \
             source_revision = EXCLUDED.source_revision, decision_identity = EXCLUDED.decision_identity, \
             operation_key = EXCLUDED.operation_key, decided_at = EXCLUDED.decided_at",
    )
    .bind(scope_key.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(ownership_generation.to_string())
    .bind(stored_state)
    .bind(holder_node.as_deref())
    .bind(holder_revision.as_deref())
    .bind(source_revision.to_string())
    .bind(&assignment.decision_identity)
    .bind(key.0.as_slice())
    .bind(decided_at)
    .execute(&mut **tx)
    .await?;
    // Every decision on a guarded scope fences the guard at the next
    // publication revision; the receipt retains that exact binding.
    let fenced_publication_revision = runtime_guard_publication_revision(root, tx, scope)
        .await?
        .map(|revision| {
            revision
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)
        })
        .transpose()?;
    sqlx::query(
        "UPDATE game_runtime_scope_assignment_writer \
         SET source_revision_high_water = $1::text::numeric(20,0) WHERE writer_id = 1",
    )
    .bind(source_revision.to_string())
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_runtime_scope_assignment_receipts \
         (operation_key, command, scope_key, ownership_generation, state, holder_node_id, \
          holder_registration_revision, source_revision, decision_identity, decided_at, \
          fenced_publication_revision) \
         VALUES ($1, $2, $3, $4::text::numeric(20,0), $5, encode($6::bytea, 'hex')::uuid, \
                 $7::text::numeric(20,0), $8::text::numeric(20,0), $9, $10, $11::text::numeric(20,0))",
    )
    .bind(key.0.as_slice())
    .bind(command)
    .bind(scope_key.as_slice())
    .bind(ownership_generation.to_string())
    .bind(stored_state)
    .bind(holder_node.as_deref())
    .bind(holder_revision.as_deref())
    .bind(source_revision.to_string())
    .bind(&assignment.decision_identity)
    .bind(decided_at)
    .bind(fenced_publication_revision.map(|revision| revision.to_string()))
    .execute(&mut **tx)
    .await?;
    // The fence runs once the assignment row, high-water and receipt are
    // consistent: the Runtime guard trigger validates authoritative history
    // and admits only publications carrying the current generation.
    let fenced = fence_runtime_guard(
        root,
        tx,
        scope,
        ownership_generation,
        &assignment.decision_identity,
        decided_at,
    )
    .await?;
    if fenced != fenced_publication_revision {
        return Err(DurabilityError::InvalidStoredState);
    }
    clear_slot(tx, registration).await?;
    Ok(AssignmentOutcome::Committed(AssignmentReceipt {
        operation_key: key,
        assignment,
        fenced_publication_revision,
    }))
}

/// Atomically move an existing Runtime guard to `ready = false` at the new
/// assignment generation, as the next monotonic Runtime source revision
/// attributed to the assignment decision. Every decision on a guarded scope
/// writes this successor, so the receipt carries the exact post-commit guard
/// publication binding; absent guards stay absent. The database trigger then
/// rejects any publication for an older generation.
async fn fence_runtime_guard(
    root: &DurabilityRoot,
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
    ownership_generation: u64,
    decision_identity: &str,
    now: i64,
) -> Result<Option<u64>, DurabilityError> {
    let store = AdmissionGuardStore::from_root(root.clone());
    let key = AdmissionAuthorityGuardKeyV1::Runtime(scope);
    let Some(current) = store.load_locked(tx, &key).await? else {
        return Ok(None);
    };
    let mut successor = current.clone();
    match &mut successor.state {
        AdmissionAuthorityGuardStateV1::Runtime {
            ready,
            ownership_generation: generation,
            ..
        } if *generation <= ownership_generation => {
            *ready = false;
            *generation = ownership_generation;
        }
        _ => return Err(DurabilityError::InvalidStoredState),
    }
    successor.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
        expected_publication_revision: current.publication_revision,
    };
    successor.publication_revision = current
        .publication_revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    // Keep the Runtime publication authority's typed CAS chain intact: the fence
    // is its next monotonic Game source revision, attributed to this decision.
    if current.source.purpose != AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness {
        return Err(DurabilityError::InvalidStoredState);
    }
    successor.source = AdmissionPublicationSourceV1 {
        authority: current.source.authority.clone(),
        purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
        source_revision: current
            .source
            .source_revision
            .checked_add(1)
            .ok_or(DurabilityError::InvalidStoredState)?,
        decision_identity: decision_identity.to_owned(),
        source_observed_at: now.max(current.source.source_observed_at),
        clock_uncertainty_seconds: 0,
    };
    let encoded = encode_guard(&successor, MAX_ADMISSION_GUARD_BYTES)?;
    let changes = [successor];
    let predecessors = [Some(current)];
    if !store
        .successor_history_available(tx, &changes, &predecessors)
        .await?
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    store
        .persist_locked(tx, &changes, &predecessors, &[encoded])
        .await?;
    Ok(Some(changes[0].publication_revision))
}

/// Every allocated writer revision leaves an immutable receipt. The database
/// validates contiguous coverage and exact equality between each current scope
/// row and that scope's latest retained receipt while holding the writer lock.
pub(super) async fn require_history_matches_high_water(
    tx: &mut Transaction<'_, Postgres>,
    high_water: u64,
) -> Result<(), DurabilityError> {
    let valid: bool = sqlx::query_scalar("SELECT game_runtime_scope_assignment_history_valid()")
        .fetch_one(&mut **tx)
        .await?;
    let observed: String = sqlx::query_scalar(
        "SELECT source_revision_high_water::text FROM game_runtime_scope_assignment_writer WHERE writer_id = 1",
    )
    .fetch_one(&mut **tx)
    .await?;
    if !valid || parse_u64_text(&observed)? != high_water {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

pub(super) async fn writer_high_water(
    tx: &mut Transaction<'_, Postgres>,
    lock: bool,
) -> Result<u64, DurabilityError> {
    let sql = if lock {
        "SELECT source_revision_high_water::text FROM game_runtime_scope_assignment_writer \
         WHERE writer_id = 1 FOR UPDATE"
    } else {
        "SELECT source_revision_high_water::text FROM game_runtime_scope_assignment_writer \
         WHERE writer_id = 1"
    };
    let high_water: Option<String> = sqlx::query_scalar(sql).fetch_optional(&mut **tx).await?;
    parse_u64_text(&high_water.ok_or(DurabilityError::InvalidStoredState)?)
}

pub(super) async fn load_assignment(
    tx: &mut Transaction<'_, Postgres>,
    scope_key: &[u8],
    lock: bool,
) -> Result<Option<RuntimeScopeAssignment>, DurabilityError> {
    let sql = if lock {
        "SELECT uuid_send(world_id), uuid_send(channel_id), ownership_generation::text, state, \
                uuid_send(holder_node_id), holder_registration_revision::text, source_revision::text, \
                decision_identity, decided_at \
         FROM game_runtime_scope_assignments WHERE scope_key = $1 FOR UPDATE"
    } else {
        "SELECT uuid_send(world_id), uuid_send(channel_id), ownership_generation::text, state, \
                uuid_send(holder_node_id), holder_registration_revision::text, source_revision::text, \
                decision_identity, decided_at \
         FROM game_runtime_scope_assignments WHERE scope_key = $1"
    };
    let row = sqlx::query(sql)
        .bind(scope_key)
        .fetch_optional(&mut **tx)
        .await?;
    row.map(|row| decode_assignment(&row, 0)).transpose()
}

async fn load_receipt(
    tx: &mut Transaction<'_, Postgres>,
    key: &OperationKey,
) -> Result<Option<(AssignmentReceipt, Vec<u8>)>, DurabilityError> {
    let row = sqlx::query(
        "SELECT uuid_send(a.world_id), uuid_send(a.channel_id), r.ownership_generation::text, r.state, \
                uuid_send(r.holder_node_id), r.holder_registration_revision::text, r.source_revision::text, \
                r.decision_identity, r.decided_at, r.command, r.fenced_publication_revision::text \
         FROM game_runtime_scope_assignment_receipts r \
         JOIN game_runtime_scope_assignments a USING (scope_key) \
         WHERE r.operation_key = $1",
    )
    .bind(key.0.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let assignment = decode_assignment(&row, 0)?;
    let command: Vec<u8> = row.try_get(9).map_err(stored)?;
    let fenced: Option<String> = row.try_get(10).map_err(stored)?;
    Ok(Some((
        AssignmentReceipt {
            operation_key: *key,
            assignment,
            fenced_publication_revision: fenced.as_deref().map(parse_u64_text).transpose()?,
        },
        command,
    )))
}

fn decode_assignment(
    row: &PgRow,
    offset: usize,
) -> Result<RuntimeScopeAssignment, DurabilityError> {
    let world: Vec<u8> = row.try_get(offset).map_err(stored)?;
    let channel: Vec<u8> = row.try_get(offset + 1).map_err(stored)?;
    let generation: String = row.try_get(offset + 2).map_err(stored)?;
    let state: i16 = row.try_get(offset + 3).map_err(stored)?;
    let holder_node: Option<Vec<u8>> = row.try_get(offset + 4).map_err(stored)?;
    let holder_revision: Option<String> = row.try_get(offset + 5).map_err(stored)?;
    let source_revision: String = row.try_get(offset + 6).map_err(stored)?;
    let decision_identity: String = row.try_get(offset + 7).map_err(stored)?;
    let decided_at: i64 = row.try_get(offset + 8).map_err(stored)?;
    let world_id = WorldId::decode(&world).map_err(|_| DurabilityError::InvalidStoredState)?;
    let channel_id =
        ChannelId::decode(&channel).map_err(|_| DurabilityError::InvalidStoredState)?;
    let holder = match (holder_node, holder_revision) {
        (Some(node), Some(revision)) => Some(NodeRegistrationFact::new(
            NodeId::decode(&node).map_err(|_| DurabilityError::InvalidStoredState)?,
            parse_u64_text(&revision)?,
        )),
        (None, None) => None,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let state = match (state, holder.is_some()) {
        (STATE_ASSIGNED, true) => AssignmentState::Assigned,
        (STATE_REVOKED, false) => AssignmentState::Revoked,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let ownership_generation = parse_u64_text(&generation)?;
    let source_revision = parse_u64_text(&source_revision)?;
    if ownership_generation == 0
        || source_revision == 0
        || decision_identity.is_empty()
        || decision_identity.len() > DECISION_IDENTITY_BYTES
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(RuntimeScopeAssignment {
        scope: RuntimeScopeRefV1::channel(world_id, channel_id),
        ownership_generation,
        state,
        holder,
        source_revision,
        decision_identity,
        decided_at,
    })
}

async fn lock_slot(
    tx: &mut Transaction<'_, Postgres>,
    registration: &str,
) -> Result<Option<(Vec<u8>, Vec<u8>)>, DurabilityError> {
    let row = sqlx::query(
        "SELECT operation_key, command FROM game_runtime_scope_assignment_slots \
         WHERE writer_registration = $1 FOR UPDATE",
    )
    .bind(registration)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(DurabilityError::InvalidStoredState)?;
    let key: Option<Vec<u8>> = row.try_get(0).map_err(stored)?;
    let command: Option<Vec<u8>> = row.try_get(1).map_err(stored)?;
    match (key, command) {
        (Some(key), Some(command)) => Ok(Some((key, command))),
        (None, None) => Ok(None),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

async fn clear_slot(
    tx: &mut Transaction<'_, Postgres>,
    registration: &str,
) -> Result<(), DurabilityError> {
    let cleared = sqlx::query(
        "UPDATE game_runtime_scope_assignment_slots \
         SET operation_key = NULL, command = NULL, checkpointed_at = NULL \
         WHERE writer_registration = $1 AND operation_key IS NOT NULL",
    )
    .bind(registration)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    if cleared == 1 {
        Ok(())
    } else {
        Err(DurabilityError::InvalidStoredState)
    }
}

fn admissible(
    queue: &QueueState,
    reconciling: Option<OperationKey>,
) -> Result<(), AssignmentError> {
    match (queue.unreconciled, reconciling) {
        (Some(pending), Some(key)) if pending == key => Ok(()),
        (Some(_), _) => Err(AssignmentError::ReconcileRequired),
        (None, _) => Ok(()),
    }
}

fn lock_queue(queue: &Mutex<QueueState>) -> MutexGuard<'_, QueueState> {
    queue
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn channel_scope(
    scope: RuntimeScopeRefV1,
) -> Result<(WorldId, ChannelId), AssignmentError> {
    match scope {
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => Ok((world_id, channel_id)),
        RuntimeScopeRefV1::Instance { .. } => Err(AssignmentError::Unsupported),
    }
}

pub(super) fn scope_key(world_id: WorldId, channel_id: ChannelId) -> [u8; 33] {
    let mut key = [0_u8; 33];
    key[0] = CHANNEL_SCOPE_TAG;
    key[1..17].copy_from_slice(world_id.as_bytes());
    key[17..].copy_from_slice(channel_id.as_bytes());
    key
}

fn operation_key_from_stored(bytes: &[u8]) -> Result<OperationKey, DurabilityError> {
    bytes
        .try_into()
        .map(OperationKey)
        .map_err(|_| DurabilityError::InvalidStoredState)
}

fn valid_token(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn parse_u64_text(value: &str) -> Result<u64, DurabilityError> {
    value
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)
}

fn stored(_: sqlx::Error) -> DurabilityError {
    DurabilityError::InvalidStoredState
}

fn has_sql_state(error: &sqlx::Error, expected: &str) -> bool {
    error
        .as_database_error()
        .and_then(|database| database.code())
        .is_some_and(|code| code == expected)
}

async fn runtime_guard(
    root: &DurabilityRoot,
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
) -> Result<Option<(u64, u64)>, DurabilityError> {
    let store = AdmissionGuardStore::from_root(root.clone());
    match store
        .load_locked(tx, &AdmissionAuthorityGuardKeyV1::Runtime(scope))
        .await?
    {
        None => Ok(None),
        Some(change) => match change.state {
            AdmissionAuthorityGuardStateV1::Runtime {
                ownership_generation,
                ..
            } => Ok(Some((change.publication_revision, ownership_generation))),
            _ => Err(DurabilityError::InvalidStoredState),
        },
    }
}

/// A partial restore must not roll the Runtime guard back behind the fence
/// recorded by the scope's latest retained receipt. Guard publications only
/// advance, and `load_locked` already rejects a current row that trails its
/// own history, so the current guard must be at or beyond that fence;
/// otherwise the predecessor binding is untrustworthy.
async fn require_guard_covers_latest_fence(
    root: &DurabilityRoot,
    tx: &mut Transaction<'_, Postgres>,
    scope_key: &[u8],
    scope: RuntimeScopeRefV1,
) -> Result<(), DurabilityError> {
    let latest = sqlx::query(
        "SELECT fenced_publication_revision::text AS fenced, \
                ownership_generation::text AS generation, decision_identity \
         FROM game_runtime_scope_assignment_receipts WHERE scope_key = $1 \
         ORDER BY source_revision DESC LIMIT 1",
    )
    .bind(scope_key)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(latest) = latest else {
        return Ok(());
    };
    let fenced: Option<String> = latest.try_get("fenced")?;
    let Some(fenced) = fenced else {
        return Ok(());
    };
    let fenced = parse_u64_text(&fenced)?;
    let generation = parse_u64_text(&latest.try_get::<String, _>("generation")?)?;
    let decision: String = latest.try_get("decision_identity")?;
    let store = AdmissionGuardStore::from_root(root.clone());
    let key = AdmissionAuthorityGuardKeyV1::Runtime(scope);
    let current = store
        .load_locked(tx, &key)
        .await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    if current.publication_revision < fenced {
        return Err(DurabilityError::InvalidStoredState);
    }
    // The exact fence publication must still be retained under this guard's
    // key: every SQL mirror must agree with its payload, which must carry the
    // same publication authority, this receipt's decision and generation, and
    // not ready. A dropped, moved or substituted fence revision fails closed.
    let mut encoded_key = Writer::new(MAX_ADMISSION_GUARD_BYTES);
    write_key(&mut encoded_key, &key)?;
    let row = sqlx::query(
        "SELECT source_authority, source_revision::text AS source_revision, \
                decision_identity, change_json \
         FROM game_durability_admission_guard_history \
         WHERE guard_key = $1 AND publication_revision = $2::text::numeric(20,0)",
    )
    .bind(&encoded_key.bytes)
    .bind(fenced.to_string())
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(DurabilityError::InvalidStoredState)?;
    let authority: String = row.try_get("source_authority")?;
    let source_revision = parse_u64_text(&row.try_get::<String, _>("source_revision")?)?;
    let mirrored_decision: String = row.try_get("decision_identity")?;
    let payload: String = row.try_get("change_json")?;
    let change = decode_guard(&payload, MAX_ADMISSION_GUARD_BYTES)?;
    let retained = change.key == key
        && change.publication_revision == fenced
        && change.source.authority == authority
        && change.source.source_revision == source_revision
        && change.source.decision_identity == mirrored_decision
        && change.source.authority == current.source.authority
        && change.source.purpose == AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness
        && change.source.decision_identity == decision
        && matches!(
            change.state,
            AdmissionAuthorityGuardStateV1::Runtime {
                ready: false,
                ownership_generation,
                ..
            } if ownership_generation == generation
        );
    if !retained {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

async fn runtime_guard_publication_revision(
    root: &DurabilityRoot,
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
) -> Result<Option<u64>, DurabilityError> {
    Ok(runtime_guard(root, tx, scope)
        .await?
        .map(|(revision, _)| revision))
}

async fn runtime_guard_generation(
    root: &DurabilityRoot,
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
) -> Result<Option<u64>, DurabilityError> {
    Ok(runtime_guard(root, tx, scope)
        .await?
        .map(|(_, generation)| generation))
}

/// Writer-owned successor revision. Only a valid, fully retained history
/// reaches this point; the exhausted namespace rejects rather than wrapping.
fn successor_source_revision(high_water: u64) -> Result<u64, AssignmentRejection> {
    high_water
        .checked_add(1)
        .ok_or(AssignmentRejection::SourceRevisionExhausted)
}

#[cfg(test)]
mod successor_revision_tests {
    use super::{AssignmentRejection, successor_source_revision};

    #[test]
    fn source_revision_successor_is_checked() {
        assert_eq!(successor_source_revision(0), Ok(1));
        assert_eq!(successor_source_revision(41), Ok(42));
        assert_eq!(successor_source_revision(u64::MAX - 1), Ok(u64::MAX));
        assert_eq!(
            successor_source_revision(u64::MAX),
            Err(AssignmentRejection::SourceRevisionExhausted)
        );
    }
}
