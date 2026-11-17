//! Literal Item-id MoveEvents over the existing scoped physical Item ledger.
//! The original committed translation and selected UUID are retained; a replay
//! is reconciled before any current tile lookup and never selects a replacement.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_item_transaction::StandingTileRead;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementPositionSnapshot, RuntimeScopeRefV1,
};
use oteryn_gameplay_entities::carrier::native_translation_events::CommittedMovementOccurrence;
mod creature;
pub use creature::{
    PreparedSourceCreatureItem, SourceCreatureGroundStep, SourceCreatureGroundTile,
};

#[derive(Debug, Clone)]
pub(super) enum SourceGroundSubject {
    Player(CurrentCharacterItemFence),
    Creature(std::sync::Arc<crate::content::creature_policy::CompiledCreaturePolicy>),
}
impl SourceGroundSubject {
    pub(super) fn player(&self) -> Option<&CurrentCharacterItemFence> {
        match self {
            Self::Player(fence) => Some(fence),
            Self::Creature(_) => None,
        }
    }
}

pub(super) const SOURCE_STEP_CAUSE: i16 = 6;
pub(super) const SOURCE_BODY: [u8; 32] = [
    0x9e, 0x11, 0x07, 0xdc, 0x41, 0xab, 0xc1, 0x6b, 0x4f, 0x56, 0xe7, 0x95, 0x18, 0x0c, 0x1c, 0x17,
    0xc9, 0x17, 0xd8, 0xe7, 0x37, 0x54, 0x54, 0x7c, 0x44, 0x3f, 0x07, 0x0e, 0xd3, 0x16, 0x79, 0x16,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceGroundStepKind {
    Weeping,
    DeadAspect,
    Remains,
}
impl SourceGroundStepKind {
    pub fn from_definition(definition: &TypedDefinitionRef) -> Option<Self> {
        if definition.family != "Item" || definition.revision_ref != "definition-r1" {
            return None;
        }
        match definition.production_key.as_str() {
            "oteryn:item.tibia.i33876" => Some(Self::Weeping),
            "oteryn:item.tibia.i33949" => Some(Self::DeadAspect),
            "oteryn:item.tibia.i33984" => Some(Self::Remains),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct SourceGroundCause {
    pub(super) scope: RuntimeScopeRefV1,
    pub(super) generation: u64,
    pub(super) node: [u8; 16],
    pub(super) node_revision: u64,
    pub(super) assignment_revision: u64,
    pub(super) source_items: Vec<TypedDefinitionRef>,
    pub(super) definition: TypedDefinitionRef,
    pub(super) occurrence: String,
    pub(super) content: [u8; 32],
    pub(super) client: [u8; 32],
    pub(super) activation: u64,
    pub(super) map: [u8; 32],
    pub(super) frame: [u8; 32],
    pub(super) body: [u8; 32],
    pub(super) cast: [u8; 32],
    pub(super) actor: ExactActorRef,
    pub(super) placement: [u8; 16],
    pub(super) subject: SourceGroundSubject,
    pub(super) movement: MovementPositionSnapshot,
    pub(super) committed: CommittedMovementOccurrence,
    pub(super) reset_epoch: u64,
    pub(super) item: [u8; 16],
    pub(super) target: SpellGroundTarget,
}

#[derive(Debug, Clone)]
pub struct SourceGroundStep {
    cause: SourceGroundCause,
    kind: SourceGroundStepKind,
}
pub(super) struct PreparedSourceGroundOperations {
    pub(super) binding: [u8; 32],
    pub(super) operations: Vec<SpellItemOperation>,
}
pub struct SourceGroundStepReceipt {
    pub receipt: CreatureGroundReceipt,
    pub removed: bool,
}
impl SourceGroundStep {
    #[allow(
        clippy::too_many_arguments,
        reason = "actual movement, current native geometry and SQL destination read independently qualify the selected Item"
    )]
    pub fn from_current_player_step(
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        actor: ExactActorRef,
        session: GameSessionId,
        movement: MovementPositionSnapshot,
        reset_epoch: u64,
        read: &StandingTileRead,
        item: [u8; 16],
    ) -> Option<Self> {
        runtime.owner_fence().ok()?;
        let target =
            SpellGroundTarget::for_native_tile_read(room, runtime, movement.position()).ok()?;
        let committed = runtime.committed_movement_occurrence(actor).ok()??;
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        let pin = runtime.content_pin();
        if native.source_digest() != pin.server_artifact_digest()
            || !native.qualifies_soul_war_callbacks(pin.server_artifact_digest())
            || runtime.read_actor_position(actor).ok()? != movement
            || committed.mover() != actor
            || committed.after() != movement
            || movement.last_translation_revision() == 0
            || movement.revision() != movement.last_translation_revision()
            || runtime
                .player_control_facts(actor, session)
                .ok()?
                .control_loss
                .is_some()
            || runtime.house_actor_state(actor, session).is_ok()
            || read.target() != &target
            || read.content_digest() != native.source_digest()
            || read.fence().game_session_id != session
            || read.fence().character_id.as_bytes() != lease.character_id().as_bytes()
            || read.fence().character_lease_generation != lease.generation()
            || read.fence().runtime_scope != runtime.binding().runtime_scope()
            || read.fence().scope_ownership_generation != runtime.binding().scope_generation()
            || !uuid_v7(&item)
        {
            return None;
        }
        let selected = read
            .items()
            .iter()
            .find(|entry| entry.item_instance_id == item)?;
        let kind = SourceGroundStepKind::from_definition(&selected.definition)?;
        native.item_index()?.definition_ref(
            "Item",
            &selected.definition.production_key,
            "definition-r1",
        )?;
        let binding = runtime.binding();
        let mut hash = Sha256::new();
        hash.update(b"oteryn:source-ground-step:v1");
        hash.update(actor.placement_identity());
        hash.update(actor.actor_local_generation().to_be_bytes());
        hash.update(session.as_bytes());
        hash.update(read.fence().character_id.as_bytes());
        hash.update(read.fence().connection_generation.get().to_be_bytes());
        hash.update(lease.generation().to_be_bytes());
        hash.update(movement.revision().to_be_bytes());
        hash.update(movement.last_translation_revision().to_be_bytes());
        hash.update(committed.binding());
        hash.update(reset_epoch.to_be_bytes());
        hash.update(binding.world_id().as_bytes());
        match binding.runtime_scope() {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                hash.update([1]);
                hash.update(channel_id.as_bytes());
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                hash.update([2]);
                hash.update(instance_id);
            }
        }
        hash.update(binding.scope_generation().get().to_be_bytes());
        hash.update(binding.node_id().as_bytes());
        hash.update(binding.node_registration_revision().to_be_bytes());
        hash.update(binding.source_revision().to_be_bytes());
        hash.update(item);
        hash.update(selected.definition.production_key.as_bytes());
        hash.update(&target.spatial_position);
        hash.update(pin.activation_sequence().to_be_bytes());
        for digest in [
            pin.server_artifact_digest(),
            pin.client_artifact_digest(),
            pin.frame_binding_digest(),
            runtime.map_revision_digest(),
        ] {
            hash.update(digest);
        }
        let cast: [u8; 32] = hash.finalize().into();
        Some(Self {
            kind,
            cause: SourceGroundCause {
                scope: binding.runtime_scope(),
                generation: binding.scope_generation().get(),
                node: *binding.node_id().as_bytes(),
                node_revision: binding.node_registration_revision(),
                assignment_revision: binding.source_revision(),
                source_items: vec![selected.definition.clone()],
                definition: TypedDefinitionRef {
                    family: "Encounter".into(),
                    production_key: "oteryn:encounter.soul_war_taint_zones".into(),
                    revision_ref: "definition-r1".into(),
                },
                occurrence: format!("source-ground-step:{}", super::rotten_portal::hex(&cast)),
                content: native.source_digest(),
                client: pin.client_artifact_digest(),
                activation: pin.activation_sequence(),
                map: runtime.map_revision_digest(),
                frame: pin.frame_binding_digest(),
                body: SOURCE_BODY,
                cast,
                actor,
                placement: actor.placement_identity(),
                subject: SourceGroundSubject::Player(*read.fence()),
                movement,
                committed,
                reset_epoch,
                item,
                target,
            },
        })
    }
    pub fn kind(&self) -> SourceGroundStepKind {
        self.kind
    }
    pub fn item_instance_id(&self) -> [u8; 16] {
        self.cause.item
    }
    pub fn binding(&self) -> [u8; 32] {
        self.cause.cast
    }
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let c = &self.cause;
        let binding = runtime.binding();
        let pin = runtime.content_pin();
        runtime.owner_fence().is_ok()
            && binding.runtime_scope() == c.scope
            && binding.scope_generation().get() == c.generation
            && binding.node_id().as_bytes() == &c.node
            && binding.node_registration_revision() == c.node_revision
            && binding.source_revision() == c.assignment_revision
            && pin.activation_sequence() == c.activation
            && pin.server_artifact_digest() == c.content
            && pin.client_artifact_digest() == c.client
            && pin.frame_binding_digest() == c.frame
            && runtime.map_revision_digest() == c.map
            && runtime.read_actor_position(c.actor).ok() == Some(c.movement)
            && runtime
                .committed_movement_occurrence(c.actor)
                .ok()
                .flatten()
                == Some(c.committed)
            && match &c.subject {
                SourceGroundSubject::Player(fence) => runtime
                    .bound_attacker_lease(c.actor, fence.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    }),
                SourceGroundSubject::Creature(policy) => {
                    self.kind == SourceGroundStepKind::DeadAspect
                        && runtime
                            .companion_snapshot(c.actor)
                            .is_ok_and(|subject| subject.state.policy == *policy)
                        && runtime
                            .companion_policy(&policy.definition_key)
                            .is_ok_and(|current| current == *policy)
                }
            }
    }
}

async fn result(
    tx: &mut Transaction<'_, Postgres>,
    receipt: CreatureGroundReceipt,
) -> Result<SourceGroundStepReceipt> {
    let count: i32 = sqlx::query_scalar("SELECT operation_count FROM game_spell_item_receipts WHERE transaction_id=encode($1,'hex')::uuid")
        .bind(receipt.transaction_id.as_slice()).fetch_one(&mut **tx).await?;
    if !(0..=1).contains(&count) {
        return Err(SpellItemError::Rejected(
            "source STEP immutable operation count",
        ));
    }
    Ok(SourceGroundStepReceipt {
        receipt,
        removed: count == 1,
    })
}

async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    node: &NodeIncarnationProof,
    step: &SourceGroundStep,
) -> Result<SourceGroundStepReceipt> {
    let cause = ScopedGroundCauseRef::SourceStep(&step.cause);
    if let Some(receipt) =
        reconcile_scoped_ground_in_transaction(tx, authority, cause, || Ok(())).await?
    {
        return result(tx, receipt).await;
    }
    if let Some(fence) = step.cause.subject.player()
        && crate::durability::character_progression::assert_item_only_fence(tx, fence, node)
            .await?
            .is_err()
    {
        return Err(SpellItemError::Rejected("source STEP current Player fence"));
    }
    let tile = read_creature_scope_tile_in_transaction(tx, authority, &step.cause.target).await?;
    let operations = if let Some(current) = tile
        .items
        .iter()
        .find(|entry| entry.item_instance_id == step.cause.item)
    {
        if !step.cause.source_items.contains(&current.definition) {
            return Err(SpellItemError::Rejected(
                "source STEP selected Item definition changed",
            ));
        }
        vec![SpellItemOperation::RemoveGround(
            prepare_creature_scope_removal_in_transaction(
                tx,
                authority,
                &step.cause.target,
                step.cause.item,
            )
            .await?,
        )]
    } else {
        Vec::new()
    };
    let prepared = PreparedSourceGroundOperations {
        binding: step.cause.cast,
        operations,
    };
    let identity = |domain: &[u8]| {
        let digest = Sha256::new()
            .chain_update(domain)
            .chain_update(step.cause.cast)
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        id[6] = (id[6] & 15) | 0x70;
        id[8] = (id[8] & 63) | 0x80;
        id
    };
    let receipt = apply_scoped_ground_in_transaction(
        tx,
        authority,
        cause,
        identity(b"oteryn:source-ground-step-transaction:v1"),
        identity(b"oteryn:source-ground-step-event:v1"),
        ScopedGroundPreparedRef::SourceStep(&prepared),
        || Ok(()),
    )
    .await?;
    result(tx, receipt).await
}
impl DurabilityRoot {
    pub async fn commit_source_ground_step(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        step: &SourceGroundStep,
    ) -> Result<SourceGroundStepReceipt> {
        if node.fact().node_id().as_bytes() != &step.cause.node
            || node.fact().registration_revision() != step.cause.node_revision
        {
            return Err(SpellItemError::Rejected(
                "source STEP retained Node identity",
            ));
        }
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let mut context = (permit, step);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, ctx| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<SourceGroundStepReceipt> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            ctx.0,
                            &record,
                            &node,
                            ctx.1.cause.scope,
                            ctx.1.cause.generation,
                        )
                        .await?;
                        execute(&mut tx, &authority, &node, ctx.1).await
                    }
                    .await;
                    match outcome {
                        Ok(receipt) => {
                            commit_creature_item_owner_transaction(tx, deadline).await?;
                            Ok(Ok(receipt))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}
