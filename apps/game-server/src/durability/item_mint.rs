//! DUR-03 native one-item Ground MINT (VSL-COMBAT-01 stage C).
//!
//! This component owns durable application, idempotency and reconciliation
//! only. It does not prove that Combat legitimately produced the loot output
//! cause: [`ItemMintCause`] has no runtime constructor, because the stage D
//! Combat owner builds it from its committed death.
//!
//! Lifecycle of one logical MINT transaction:
//! 1. [`DurabilityRoot::freeze_item_mint`] validates every registered bound and
//!    fixes the TransactionId, EventId, ItemInstanceId, trusted timestamp and
//!    exact event bytes before any commit attempt can become ambiguous.
//! 2. [`DurabilityRoot::commit_item_mint`] commits that frozen candidate under
//!    the current recovery, admission-relation, runtime-scope assignment and
//!    node-incarnation fence. The death's ownership generation must be the
//!    current assignment (D52); otherwise the MINT is refused, never deferred.
//! 3. After an unknown outcome, [`DurabilityRoot::reconcile_item_mint`] reads
//!    the receipt of the same cause. Committed returns the original result; not
//!    committed permits a retry of the same frozen candidate.
//!
//! Each commit attempt and reconciliation pass charges one DUR03-RL-08 work
//! unit to the frozen candidate; a fourth is rejected before database work.

use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_mint_audit::{
    self as audit, AuditError, CreatureDeathOccurrenceRefV1, ITEM_LIFECYCLE_LIVE,
    LOOT_MINT_TYPED_CAUSE, MintEventIdentity, OneItemGroundV1, OneItemMintV1, OneItemProvenanceV1,
    OneItemStateV1, OneItemTypedDefinitionRevisionV1, RL08_RETRY_WORK_UNITS_MAX,
};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation, scope_key};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, ItemMintError>;
const INTENT_BINDING_VERSION: u8 = 1;
const EVENT_TYPE_ID: i64 = audit::EVENT_TYPE_ID as i64;
const EVENT_SCHEMA_REVISION: i64 = audit::EVENT_SCHEMA_REVISION as i64;

/// `TypedDefinitionRef = (DefinitionFamily, ProductionKey, DefinitionRevisionRef)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedDefinitionRef {
    pub family: String,
    pub production_key: String,
    pub revision_ref: String,
}

/// `CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` durable, restart-stable death key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureDeathKey {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_ownership_generation: ScopeOwnershipGeneration,
    actor_local_id: u32,
    actor_local_generation: u64,
}

/// The typed loot output cause `(death key, LootTableDefinitionRef,
/// LootEntryOrPurposeKey, DeterministicDrawOrdinal)`, stored in full as the
/// MINT's unique source cause. Stage D owns its runtime constructor; only a
/// test constructor exists here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMintCause {
    death: CreatureDeathKey,
    loot_table: TypedDefinitionRef,
    purpose_key: String,
    draw_ordinal: u32,
}

impl ItemMintCause {
    /// Test-only: production causes descend from a committed Combat death.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn for_test(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_ownership_generation: ScopeOwnershipGeneration,
        actor_local_id: u32,
        actor_local_generation: u64,
        loot_table: TypedDefinitionRef,
        purpose_key: String,
        draw_ordinal: u32,
    ) -> Self {
        Self {
            death: CreatureDeathKey {
                world_id,
                channel_id,
                scope_ownership_generation,
                actor_local_id,
                actor_local_generation,
            },
            loot_table,
            purpose_key,
            draw_ordinal,
        }
    }
}

/// Actual typed Ground in the death's own World/Channel and generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundPlacement {
    pub spatial_position: Vec<u8>,
    pub corpse_ref: Vec<u8>,
    pub map_revision: String,
    pub content_revision: String,
    pub native_room_placement_context: Vec<u8>,
}

/// Complete semantic input of one Ground MINT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMintRequest {
    pub cause: ItemMintCause,
    pub item: TypedDefinitionRef,
    pub quantity: u32,
    pub ground: GroundPlacement,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
}

/// Frozen candidate of one logical MINT transaction. It is deliberately not
/// `Clone`: every attempt and reconciliation reuses this exact value and its
/// DUR03-RL-08 budget.
#[derive(Debug)]
pub struct ItemMintCandidate {
    request: ItemMintRequest,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    envelope: Vec<u8>,
    envelope_sha256: [u8; 32],
    intent_binding: [u8; 33],
    work_units_used: u8,
}

impl ItemMintCandidate {
    #[must_use]
    pub const fn transaction_id(&self) -> &[u8; 16] {
        &self.transaction_id
    }
    #[must_use]
    pub const fn event_id(&self) -> &[u8; 16] {
        &self.event_id
    }
    #[must_use]
    pub const fn item_instance_id(&self) -> &[u8; 16] {
        &self.item_instance_id
    }
    #[must_use]
    pub const fn occurred_at_unix_ms(&self) -> i64 {
        self.occurred_at_unix_ms
    }
    /// Exact immutable EventEnvelope bytes (event type 2).
    #[must_use]
    pub fn envelope(&self) -> &[u8] {
        &self.envelope
    }
    #[must_use]
    pub const fn work_units_used(&self) -> u8 {
        self.work_units_used
    }

    fn charge_work_unit(&mut self) -> Result<()> {
        if self.work_units_used >= RL08_RETRY_WORK_UNITS_MAX {
            return Err(ItemMintError::CapacityExceeded);
        }
        self.work_units_used += 1;
        Ok(())
    }
}

/// Terminal committed result of one cause. It never changes after commit,
/// including after the audit event's own retention expiry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedItemMint {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub item_instance_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub envelope_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemMintOutcome {
    Committed(CommittedItemMint),
    /// The cause already minted; this is its original terminal result.
    AlreadyCommitted(CommittedItemMint),
}

/// Durable Ground ItemInstance read back from PostgreSQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundItemInstance {
    pub item_instance_id: [u8; 16],
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
    pub runtime_scope_ownership_generation: u64,
    pub ground: GroundPlacement,
    pub minted_transaction_id: [u8; 16],
}

#[derive(Debug)]
pub enum ItemMintError {
    /// Malformed or oversize input; registered `INVALID_INPUT`, never truncated.
    InvalidInput,
    /// A registered DUR-03 ceiling (including RL-08) was exceeded.
    CapacityExceeded,
    /// The current recovery, runtime-scope or node-incarnation fence rejected
    /// the MINT. Under D52 a death whose generation ended is never minted.
    AuthorityRejected,
    /// The cause already minted with a different intent (integrity CONFLICT).
    ConflictingCause,
    /// A frozen identity is already bound to a different cause.
    ConflictingCandidate,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for ItemMintError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<AuditError> for ItemMintError {
    fn from(error: AuditError) -> Self {
        match error {
            AuditError::InvalidInput => Self::InvalidInput,
            AuditError::CapacityExceeded => Self::CapacityExceeded,
        }
    }
}

impl std::fmt::Display for ItemMintError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid item MINT input"),
            Self::CapacityExceeded => formatter.write_str("item MINT capacity exceeded"),
            Self::AuthorityRejected => formatter.write_str("item MINT authority rejected"),
            Self::ConflictingCause => {
                formatter.write_str("MINT cause was reused with different semantics")
            }
            Self::ConflictingCandidate => {
                formatter.write_str("MINT identity is bound to a different cause")
            }
            Self::Unavailable(error) => {
                write!(formatter, "item MINT storage is unavailable: {error:?}")
            }
        }
    }
}

impl std::error::Error for ItemMintError {}

/// Owned copy of the frozen candidate moved into one database pass.
struct FrozenMint {
    request: ItemMintRequest,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    envelope: Vec<u8>,
    envelope_sha256: [u8; 32],
    intent_binding: [u8; 33],
}

impl FrozenMint {
    fn of(candidate: &ItemMintCandidate) -> Self {
        Self {
            request: candidate.request.clone(),
            transaction_id: candidate.transaction_id,
            event_id: candidate.event_id,
            item_instance_id: candidate.item_instance_id,
            occurred_at_unix_ms: candidate.occurred_at_unix_ms,
            envelope: candidate.envelope.clone(),
            envelope_sha256: candidate.envelope_sha256,
            intent_binding: candidate.intent_binding,
        }
    }
}

impl DurabilityRoot {
    /// Validate every registered bound and fix the candidate identities and
    /// exact event bytes. This pass reads only producer-owned UUIDv7 values and
    /// the trusted database timestamp; it mutates nothing and grants nothing.
    pub async fn freeze_item_mint(&self, request: ItemMintRequest) -> Result<ItemMintCandidate> {
        validate_request(&request)?;
        let intent_binding = intent_binding(&request)?;
        let (transaction_id, event_id, item_instance_id, occurred_at_unix_ms) = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let row = sqlx::query(
                        "SELECT game_character_uuid_v7()::text AS transaction_id, \
                                game_character_uuid_v7()::text AS event_id, \
                                game_character_uuid_v7()::text AS item_instance_id, \
                                floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
                                  AS occurred_at",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let ids = (
                        uuid_text(row.try_get("transaction_id")?)?,
                        uuid_text(row.try_get("event_id")?)?,
                        uuid_text(row.try_get("item_instance_id")?)?,
                        row.try_get::<i64, _>("occurred_at")?,
                    );
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(ids)
                })
            })
            .await?;
        let envelope = audit::encode_mint_event(
            MintEventIdentity {
                event_id,
                transaction_id,
                occurred_at_unix_ms,
                server_build_id: SERVER_BUILD_ID,
            },
            mint_message(&request, item_instance_id),
        )?;
        let envelope_sha256 = Sha256::digest(&envelope).into();
        Ok(ItemMintCandidate {
            request,
            transaction_id,
            event_id,
            item_instance_id,
            occurred_at_unix_ms,
            envelope,
            envelope_sha256,
            intent_binding,
            work_units_used: 0,
        })
    }

    /// Commit one frozen MINT candidate. A cause that already minted returns
    /// its original terminal result without reacquiring runtime authority; a
    /// changed intent for the same cause conflicts. A new cause commits only
    /// while its death's ownership generation is the current assignment held
    /// by this node's current incarnation.
    pub async fn commit_item_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        candidate: &mut ItemMintCandidate,
    ) -> Result<ItemMintOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemMintError::AuthorityRejected)?;
        candidate.charge_work_unit()?;
        let frozen = FrozenMint::of(candidate);
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_cause(&mut tx, &frozen.request.cause).await?;

                    if let Some(row) = load_receipt(&mut tx, &frozen.request.cause).await? {
                        let stored: Vec<u8> = row.try_get("intent_binding")?;
                        if stored != frozen.intent_binding {
                            return Ok(Err(ItemMintError::ConflictingCause));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(ItemMintOutcome::AlreadyCommitted(committed)));
                    }

                    let identity_reused: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_item_mint_receipts \
                                         WHERE transaction_id = encode($1,'hex')::uuid \
                                            OR event_id = encode($2,'hex')::uuid \
                                            OR item_instance_id = encode($3,'hex')::uuid) \
                             OR EXISTS (SELECT 1 FROM game_item_audit_outbox \
                                         WHERE transaction_id = encode($1,'hex')::uuid \
                                            OR event_id = encode($2,'hex')::uuid) \
                             OR EXISTS (SELECT 1 FROM game_item_instances \
                                         WHERE item_instance_id = encode($3,'hex')::uuid)",
                    )
                    .bind(frozen.transaction_id.as_slice())
                    .bind(frozen.event_id.as_slice())
                    .bind(frozen.item_instance_id.as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    if identity_reused {
                        return Ok(Err(ItemMintError::ConflictingCandidate));
                    }

                    // DUR-03 §32 / D52: the death's generation must be the
                    // current assignment held by this node's live incarnation.
                    let death = frozen.request.cause.death;
                    let fact = node.fact();
                    let assignment = sqlx::query(
                        "SELECT 1 FROM game_runtime_scope_assignments \
                         WHERE scope_key = $1 AND world_id = encode($2,'hex')::uuid \
                           AND channel_id = encode($3,'hex')::uuid AND state = 1 \
                           AND ownership_generation = $4::text::numeric(20,0) \
                           AND holder_node_id = encode($5,'hex')::uuid \
                           AND holder_registration_revision = $6::text::numeric(20,0) \
                         FOR SHARE",
                    )
                    .bind(scope_key(death.world_id, death.channel_id).as_slice())
                    .bind(death.world_id.as_bytes().as_slice())
                    .bind(death.channel_id.as_bytes().as_slice())
                    .bind(death.scope_ownership_generation.get().to_string())
                    .bind(fact.node_id().as_bytes().as_slice())
                    .bind(fact.registration_revision().to_string())
                    .fetch_optional(&mut *tx)
                    .await?;
                    if assignment.is_none() || !prove_current_incarnation(&mut tx, &node).await? {
                        return Ok(Err(ItemMintError::AuthorityRejected));
                    }

                    insert_mint(&mut tx, &frozen).await?;
                    let committed = CommittedItemMint {
                        transaction_id: frozen.transaction_id,
                        event_id: frozen.event_id,
                        item_instance_id: frozen.item_instance_id,
                        occurred_at_unix_ms: frozen.occurred_at_unix_ms,
                        envelope_sha256: frozen.envelope_sha256,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(ItemMintOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Resolve an unknown outcome of a frozen candidate. The cause lock waits
    /// for any in-flight attempt, so `None` proves nothing committed for this
    /// cause and the same candidate may be retried. Never reacquires runtime
    /// authority.
    pub async fn reconcile_item_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        candidate: &mut ItemMintCandidate,
    ) -> Result<Option<CommittedItemMint>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemMintError::AuthorityRejected)?;
        candidate.charge_work_unit()?;
        let cause = candidate.request.cause.clone();
        let binding = candidate.intent_binding;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_cause(&mut tx, &cause).await?;
                    let Some(row) = load_receipt(&mut tx, &cause).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(None));
                    };
                    let stored: Vec<u8> = row.try_get("intent_binding")?;
                    if stored != binding {
                        return Ok(Err(ItemMintError::ConflictingCause));
                    }
                    let committed = decode_receipt(&row)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(committed)))
                })
            })
            .await?
    }

    /// Read one durable Ground ItemInstance.
    pub async fn read_item_instance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        item_instance_id: [u8; 16],
    ) -> Result<Option<GroundItemInstance>> {
        audit::check_uuid_v7(&item_instance_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemMintError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT i.item_instance_id::text, i.world_id::text, g.channel_id::text, \
                                i.definition_family, i.definition_production_key, \
                                i.definition_revision_ref, i.quantity, \
                                i.minted_transaction_id::text, \
                                g.runtime_scope_ownership_generation::text, g.spatial_position, \
                                g.corpse_ref, g.map_revision, g.content_revision, \
                                g.native_room_placement_context \
                           FROM game_item_instances i \
                           JOIN game_item_ground_locations g USING (item_instance_id, world_id) \
                          WHERE i.item_instance_id = encode($1,'hex')::uuid",
                    )
                    .bind(item_instance_id.as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let item = row.as_ref().map(decode_instance).transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(item))
                })
            })
            .await?
    }
}

fn validate_request(request: &ItemMintRequest) -> Result<()> {
    let cause = &request.cause;
    validate_definition(&cause.loot_table)?;
    audit::check_content_key(&cause.purpose_key)?;
    validate_definition(&request.item)?;
    let ground = &request.ground;
    audit::check_technical_bytes(&ground.spatial_position)?;
    audit::check_technical_bytes(&ground.corpse_ref)?;
    audit::check_content_key(&ground.map_revision)?;
    audit::check_content_key(&ground.content_revision)?;
    audit::check_technical_bytes(&ground.native_room_placement_context)?;
    audit::check_content_key(&request.content_revision)?;
    audit::check_content_key(&request.ruleset_revision)?;
    audit::check_content_key(&request.sim_revision)?;
    if request.quantity == 0 || cause.death.actor_local_generation == 0 {
        return Err(ItemMintError::InvalidInput);
    }
    audit::TransactionResourceUsage::MINT.check()?;
    Ok(())
}

fn validate_definition(value: &TypedDefinitionRef) -> Result<()> {
    audit::check_technical_text(&value.family)?;
    audit::check_content_key(&value.production_key)?;
    audit::check_content_key(&value.revision_ref)?;
    Ok(())
}

fn push_text(out: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let length = u16::try_from(value.len()).map_err(|_| ItemMintError::InvalidInput)?;
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(value);
    Ok(())
}

fn push_definition(out: &mut Vec<u8>, value: &TypedDefinitionRef) -> Result<()> {
    push_text(out, value.family.as_bytes())?;
    push_text(out, value.production_key.as_bytes())?;
    push_text(out, value.revision_ref.as_bytes())
}

/// Digest of the complete business intent. Frozen identities and the trusted
/// timestamp are excluded, so a re-frozen candidate of the same intent
/// resolves to the original result instead of conflicting.
fn intent_binding(request: &ItemMintRequest) -> Result<[u8; 33]> {
    let cause = &request.cause;
    let death = &cause.death;
    let mut semantic = vec![INTENT_BINDING_VERSION];
    semantic.extend_from_slice(death.world_id.as_bytes());
    semantic.extend_from_slice(death.channel_id.as_bytes());
    semantic.extend_from_slice(&death.scope_ownership_generation.get().to_be_bytes());
    semantic.extend_from_slice(&death.actor_local_id.to_be_bytes());
    semantic.extend_from_slice(&death.actor_local_generation.to_be_bytes());
    push_definition(&mut semantic, &cause.loot_table)?;
    push_text(&mut semantic, cause.purpose_key.as_bytes())?;
    semantic.extend_from_slice(&cause.draw_ordinal.to_be_bytes());
    push_definition(&mut semantic, &request.item)?;
    semantic.extend_from_slice(&request.quantity.to_be_bytes());
    let ground = &request.ground;
    for value in [
        ground.spatial_position.as_slice(),
        ground.corpse_ref.as_slice(),
        ground.map_revision.as_bytes(),
        ground.content_revision.as_bytes(),
        ground.native_room_placement_context.as_slice(),
        request.content_revision.as_bytes(),
        request.ruleset_revision.as_bytes(),
        request.sim_revision.as_bytes(),
        LOOT_MINT_TYPED_CAUSE.as_bytes(),
    ] {
        push_text(&mut semantic, value)?;
    }
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = [0_u8; 33];
    binding[0] = INTENT_BINDING_VERSION;
    binding[1..].copy_from_slice(&digest);
    Ok(binding)
}

fn definition_message(value: &TypedDefinitionRef) -> OneItemTypedDefinitionRevisionV1 {
    OneItemTypedDefinitionRevisionV1 {
        family: value.family.clone(),
        production_key: value.production_key.clone(),
        revision_ref: value.revision_ref.clone(),
    }
}

fn mint_message(request: &ItemMintRequest, item_instance_id: [u8; 16]) -> OneItemMintV1 {
    let death = &request.cause.death;
    let world = death.world_id.as_bytes().to_vec();
    let channel = death.channel_id.as_bytes().to_vec();
    let generation = death.scope_ownership_generation.get();
    OneItemMintV1 {
        after: Some(OneItemStateV1 {
            item_instance_id: item_instance_id.to_vec(),
            world_id: world.clone(),
            definition: Some(definition_message(&request.item)),
            quantity: request.quantity,
            lifecycle: ITEM_LIFECYCLE_LIVE,
        }),
        destination: Some(OneItemGroundV1 {
            world_id: world.clone(),
            channel_id: channel.clone(),
            spatial_position: request.ground.spatial_position.clone(),
            corpse_ref: request.ground.corpse_ref.clone(),
            map_revision: request.ground.map_revision.clone(),
            content_revision: request.ground.content_revision.clone(),
            native_room_placement_context: request.ground.native_room_placement_context.clone(),
            runtime_scope_ownership_generation: generation,
        }),
        source: Some(OneItemProvenanceV1 {
            typed_cause: LOOT_MINT_TYPED_CAUSE.into(),
            death_occurrence: Some(CreatureDeathOccurrenceRefV1 {
                world_id: world,
                channel_id: channel,
                scope_ownership_generation: generation,
                actor_local_id: death.actor_local_id,
                actor_local_generation: death.actor_local_generation,
            }),
            loot_table: Some(definition_message(&request.cause.loot_table)),
            loot_purpose_key: request.cause.purpose_key.clone(),
            draw_ordinal: request.cause.draw_ordinal,
            content_revision: request.content_revision.clone(),
            ruleset_revision: request.ruleset_revision.clone(),
            sim_revision: request.sim_revision.clone(),
        }),
        before_semantically_absent: true,
    }
}

/// Serializes attempts and reconciliation of one cause. The primary key on the
/// full cause, not this lock, is the uniqueness authority.
async fn lock_cause(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cause: &ItemMintCause,
) -> std::result::Result<(), DurabilityError> {
    let mut key = Vec::new();
    key.extend_from_slice(cause.death.world_id.as_bytes());
    key.extend_from_slice(cause.death.channel_id.as_bytes());
    key.extend_from_slice(&cause.death.scope_ownership_generation.get().to_be_bytes());
    key.extend_from_slice(&cause.death.actor_local_id.to_be_bytes());
    key.extend_from_slice(&cause.death.actor_local_generation.to_be_bytes());
    for value in [
        cause.loot_table.family.as_bytes(),
        cause.loot_table.production_key.as_bytes(),
        cause.loot_table.revision_ref.as_bytes(),
        cause.purpose_key.as_bytes(),
    ] {
        key.extend_from_slice(&u16::try_from(value.len()).unwrap_or(u16::MAX).to_be_bytes());
        key.extend_from_slice(value);
    }
    key.extend_from_slice(&cause.draw_ordinal.to_be_bytes());
    let digest: [u8; 32] = Sha256::digest(&key).into();
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:item-mint:' || encode($1, 'hex'), 0))",
    )
    .bind(digest.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cause: &ItemMintCause,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT intent_binding, transaction_id::text, event_id::text, \
                item_instance_id::text, occurred_at, envelope_sha256 \
           FROM game_item_mint_receipts \
          WHERE death_world_id = encode($1,'hex')::uuid \
            AND death_channel_id = encode($2,'hex')::uuid \
            AND death_scope_ownership_generation = $3::text::numeric(20,0) \
            AND death_actor_local_id = $4 \
            AND death_actor_local_generation = $5::text::numeric(20,0) \
            AND loot_table_family = $6 AND loot_table_production_key = $7 \
            AND loot_table_revision_ref = $8 AND loot_purpose_key = $9 \
            AND draw_ordinal = $10",
    )
    .bind(cause.death.world_id.as_bytes().as_slice())
    .bind(cause.death.channel_id.as_bytes().as_slice())
    .bind(cause.death.scope_ownership_generation.get().to_string())
    .bind(i64::from(cause.death.actor_local_id))
    .bind(cause.death.actor_local_generation.to_string())
    .bind(&cause.loot_table.family)
    .bind(&cause.loot_table.production_key)
    .bind(&cause.loot_table.revision_ref)
    .bind(&cause.purpose_key)
    .bind(i64::from(cause.draw_ordinal))
    .fetch_optional(&mut **tx)
    .await?)
}

async fn insert_mint(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenMint,
) -> std::result::Result<(), DurabilityError> {
    let request = &frozen.request;
    let death = &request.cause.death;
    let world = death.world_id.as_bytes().as_slice();
    let item = frozen.item_instance_id.as_slice();
    sqlx::query(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, 1, \
           encode($7,'hex')::uuid)",
    )
    .bind(item)
    .bind(world)
    .bind(&request.item.family)
    .bind(&request.item.production_key)
    .bind(&request.item.revision_ref)
    .bind(i64::from(request.quantity))
    .bind(frozen.transaction_id.as_slice())
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_ground_locations(item_instance_id, world_id, channel_id, \
           runtime_scope_ownership_generation, spatial_position, corpse_ref, map_revision, \
           content_revision, native_room_placement_context) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           $4::text::numeric(20,0), $5, $6, $7, $8, $9)",
    )
    .bind(item)
    .bind(world)
    .bind(death.channel_id.as_bytes().as_slice())
    .bind(death.scope_ownership_generation.get().to_string())
    .bind(request.ground.spatial_position.as_slice())
    .bind(request.ground.corpse_ref.as_slice())
    .bind(&request.ground.map_revision)
    .bind(&request.ground.content_revision)
    .bind(request.ground.native_room_placement_context.as_slice())
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_mint_receipts(death_world_id, death_channel_id, \
           death_scope_ownership_generation, death_actor_local_id, \
           death_actor_local_generation, loot_table_family, loot_table_production_key, \
           loot_table_revision_ref, loot_purpose_key, draw_ordinal, intent_binding, \
           transaction_id, event_id, item_instance_id, occurred_at, envelope_sha256, \
           committed_at) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3::text::numeric(20,0), $4, \
           $5::text::numeric(20,0), $6, $7, $8, $9, $10, $11, encode($12,'hex')::uuid, \
           encode($13,'hex')::uuid, encode($14,'hex')::uuid, $15, $16, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(world)
    .bind(death.channel_id.as_bytes().as_slice())
    .bind(death.scope_ownership_generation.get().to_string())
    .bind(i64::from(death.actor_local_id))
    .bind(death.actor_local_generation.to_string())
    .bind(&request.cause.loot_table.family)
    .bind(&request.cause.loot_table.production_key)
    .bind(&request.cause.loot_table.revision_ref)
    .bind(&request.cause.purpose_key)
    .bind(i64::from(request.cause.draw_ordinal))
    .bind(frozen.intent_binding.as_slice())
    .bind(frozen.transaction_id.as_slice())
    .bind(frozen.event_id.as_slice())
    .bind(item)
    .bind(frozen.occurred_at_unix_ms)
    .bind(frozen.envelope_sha256.as_slice())
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
           transaction_count, event_type_id, schema_revision, retention_profile_id, \
           item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, \
           encode($6,'hex')::uuid, $7, $7 + $8, $9, sha256($9), 1)",
    )
    .bind(frozen.event_id.as_slice())
    .bind(frozen.transaction_id.as_slice())
    .bind(EVENT_TYPE_ID)
    .bind(EVENT_SCHEMA_REVISION)
    .bind(audit::RETENTION_PROFILE_ID)
    .bind(item)
    .bind(frozen.occurred_at_unix_ms)
    .bind(audit::AUDIT_RETENTION_P90D_MS)
    .bind(frozen.envelope.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedItemMint, DurabilityError> {
    let digest: Vec<u8> = row.try_get("envelope_sha256")?;
    Ok(CommittedItemMint {
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        envelope_sha256: digest
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    })
}

fn decode_instance(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<GroundItemInstance, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    Ok(GroundItemInstance {
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        world_id: WorldId::decode(&uuid_text(row.try_get("world_id")?)?).map_err(invalid)?,
        channel_id: ChannelId::decode(&uuid_text(row.try_get("channel_id")?)?).map_err(invalid)?,
        definition: TypedDefinitionRef {
            family: row.try_get("definition_family")?,
            production_key: row.try_get("definition_production_key")?,
            revision_ref: row.try_get("definition_revision_ref")?,
        },
        quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        runtime_scope_ownership_generation: row
            .try_get::<String, _>("runtime_scope_ownership_generation")?
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        ground: GroundPlacement {
            spatial_position: row.try_get("spatial_position")?,
            corpse_ref: row.try_get("corpse_ref")?,
            map_revision: row.try_get("map_revision")?,
            content_revision: row.try_get("content_revision")?,
            native_room_placement_context: row.try_get("native_room_placement_context")?,
        },
        minted_transaction_id: uuid_text(row.try_get("minted_transaction_id")?)?,
    })
}

fn uuid_text(value: &str) -> std::result::Result<[u8; 16], DurabilityError> {
    let hex: String = value
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if hex.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut out = [0_u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn definition(family: &str, key: &str) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: family.into(),
            production_key: key.into(),
            revision_ref: "rev-1".into(),
        }
    }

    fn request() -> ItemMintRequest {
        ItemMintRequest {
            cause: ItemMintCause::for_test(
                WorldId::decode(&id(1)).expect("world"),
                ChannelId::decode(&id(2)).expect("channel"),
                ScopeOwnershipGeneration::new(1).expect("generation"),
                7,
                1,
                definition("LootTable", "fixture:loot"),
                "fixture:purpose.drop".into(),
                1,
            ),
            item: definition("ItemType", "fixture:alpha"),
            quantity: 1,
            ground: GroundPlacement {
                spatial_position: vec![1, 2, 3],
                corpse_ref: id(3).to_vec(),
                map_revision: "map-1".into(),
                content_revision: "content-1".into(),
                native_room_placement_context: id(6).to_vec(),
            },
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        }
    }

    #[test]
    fn intent_binding_covers_the_whole_intent_and_every_cause_component() {
        let base = intent_binding(&request()).expect("binding");
        assert_eq!(base[0], INTENT_BINDING_VERSION);
        assert_eq!(intent_binding(&request()).expect("binding"), base);
        let mutations: [fn(&mut ItemMintRequest); 8] = [
            |r| r.cause.death.actor_local_id = 8,
            |r| r.cause.death.actor_local_generation = 2,
            |r| r.cause.loot_table.revision_ref = "rev-2".into(),
            |r| r.cause.purpose_key = "fixture:purpose.other".into(),
            |r| r.cause.draw_ordinal = 2,
            |r| r.item.production_key = "fixture:beta".into(),
            |r| r.quantity = 2,
            |r| r.ground.spatial_position = vec![1, 2, 4],
        ];
        for mutate in mutations {
            let mut changed = request();
            mutate(&mut changed);
            assert_ne!(intent_binding(&changed).expect("binding"), base);
        }
    }

    #[test]
    fn invalid_input_fails_before_database_work() {
        let mut zero = request();
        zero.quantity = 0;
        assert!(matches!(
            validate_request(&zero),
            Err(ItemMintError::InvalidInput)
        ));
        let mut generation = request();
        generation.cause.death.actor_local_generation = 0;
        assert!(matches!(
            validate_request(&generation),
            Err(ItemMintError::InvalidInput)
        ));
        let mut long = request();
        long.cause.purpose_key = "p".repeat(513);
        assert!(matches!(
            validate_request(&long),
            Err(ItemMintError::InvalidInput)
        ));
        let mut max = request();
        max.cause.purpose_key = "p".repeat(512);
        max.ground.corpse_ref = vec![3; 128];
        assert!(validate_request(&max).is_ok());
        let mut technical = request();
        technical.ground.corpse_ref = vec![3; 129];
        assert!(matches!(
            validate_request(&technical),
            Err(ItemMintError::InvalidInput)
        ));
    }

    #[test]
    fn frozen_mint_message_is_admissible_and_in_the_death_scope() {
        let message = mint_message(&request(), id(9));
        audit::check_mint(&message).expect("admissible MINT");
        let ground = message.destination.expect("ground");
        assert_eq!(ground.world_id, id(1).to_vec());
        assert_eq!(ground.channel_id, id(2).to_vec());
        assert_eq!(ground.runtime_scope_ownership_generation, 1);
    }
}
