//! Registered source GlobalEvent; no Creature, Player cost or invented Ability.
use super::scoped_global::{GlobalFireAction, ScopedGlobalGroundCause, ScopedGlobalIdentity};
use super::*;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition, RuntimeScopeRefV1};

#[derive(Debug)]
pub struct SpiteGlobalFireCallback {
    registration: crate::content::native_soul_war_global_events::QualifiedSpiteGlobalEvent,
    execution_us: u64,
    cause: ScopedGlobalGroundCause,
    operation: Option<SpellItemOperation>,
    target: SpellGroundTarget,
    position: MovementLocalPosition,
    due_us: u64,
}
/// Captured under the actual runtime before SQL. It grants no actor or new
/// registration: the existing scope transaction independently re-proves Node
/// possession/assignment, and the caller rechecks this DATA after the await.
#[derive(Debug)]
pub struct PreparedSpiteGlobalWrite {
    registration: crate::content::native_soul_war_global_events::QualifiedSpiteGlobalEvent,
    execution_us: u64,
    identity: ScopedGlobalIdentity,
    pins: [[u8; 32]; 4],
    activation: u64,
    cast: [u8; 32],
}
/// The immutable physical receipt decides whether expiry found a current fire.
/// A retry reads that result before inspecting the current tile again.
pub struct SpiteGlobalFireReceipt {
    pub receipt: CreatureGroundReceipt,
    pub removed: bool,
}
impl PreparedSpiteGlobalWrite {
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let binding = runtime.binding();
        let pin = runtime.content_pin();
        binding.runtime_scope() == self.identity.scope
            && binding.scope_generation().get() == self.identity.ownership_generation
            && binding.node_id().as_bytes() == &self.identity.node
            && binding.node_registration_revision() == self.identity.node_registration_revision
            && binding.source_revision() == self.identity.assignment_revision
            && runtime
                .owner_fence()
                .is_ok_and(|fence| fence.accepts_stamp(self.identity.stamp))
            && pin.activation_sequence() == self.activation
            && self.pins
                == [
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    runtime.map_revision_digest(),
                ]
    }
    fn matches(&self, callback: &SpiteGlobalFireCallback) -> bool {
        self.identity == callback.cause.identity
            && self.registration == callback.registration
            && self.execution_us == callback.execution_us
            && self.cast == callback.cause.cast
            && self.activation == callback.cause.activation
            && self.pins
                == [
                    callback.cause.content,
                    callback.cause.client,
                    callback.cause.frame,
                    callback.cause.map,
                ]
    }
}
impl SpiteGlobalFireCallback {
    pub fn prepare_current_write(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<PreparedSpiteGlobalWrite> {
        if !self.current_for(runtime) {
            return Err(SpellItemError::Rejected(
                "Spite current registration capture",
            ));
        }
        Ok(PreparedSpiteGlobalWrite {
            registration: self.registration.clone(),
            execution_us: self.execution_us,
            identity: self.cause.identity.clone(),
            pins: [
                self.cause.content,
                self.cause.client,
                self.cause.frame,
                self.cause.map,
            ],
            activation: self.cause.activation,
            cast: self.cause.cast,
        })
    }
    /// `tick` is the retained actual registration-clock occurrence, not a client command.
    pub fn from_current_registration(
        runtime: &mut ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        proof: &crate::content::native_soul_war_global_events::QualifiedSpiteGlobalEvent,
        tick: u64,
        index: u8,
        now_us: u64,
    ) -> Result<Self> {
        let binding = runtime.binding();
        let pin = runtime.content_pin();
        if tick == 0
            || proof.content() != pin.server_artifact_digest()
            || native.source_digest() != proof.content()
            || room.compiled().server_digest() != proof.content()
            || !native.qualifies_soul_war_callbacks(proof.content())
        {
            return Err(SpellItemError::Rejected(
                "Spite registered GlobalEvent native proof",
            ));
        }
        let [x, y, z] = *proof
            .positions()
            .get(usize::from(index))
            .ok_or(SpellItemError::Rejected("Spite literal fire position"))?;
        let position = MovementLocalPosition {
            x,
            y,
            floor: i16::try_from(z).map_err(|_| SpellItemError::Rejected("Spite fire floor"))?,
        };
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, position)
            .map_err(SpellItemError::Rejected)?;
        let policy = native
            .item_policy("oteryn:item.tibia.i33877", "definition-r1")
            .ok_or(SpellItemError::Rejected("Spite real fire Item policy"))?;
        let definition = QualifiedItemDefinition::from_native_policy(policy)
            .map_err(SpellItemError::Rejected)?;
        if !definition.materializable
            || !definition.ground_destination
            || definition.movable
            || definition.stack_maximum != 1
            || definition.container_capacity.is_some()
            || definition.decay.is_some()
            || !definition.decay_chain.is_empty()
        {
            return Err(SpellItemError::Rejected(
                "Spite source nonpickupable nondecaying fire",
            ));
        }
        let due_us = now_us
            .checked_add(
                proof
                    .expiry_ms()
                    .checked_mul(1000)
                    .ok_or(SpellItemError::Rejected("Spite expiry clock overflow"))?,
            )
            .ok_or(SpellItemError::Rejected("Spite expiry clock overflow"))?;
        let occurrence = format!("registered-global:{}:{tick}:{index}:create", proof.name());
        let mut digest = Sha256::new();
        digest.update(b"oteryn:registered-spite-global-fire:v1");
        digest.update(occurrence.as_bytes());
        digest.update(binding.world_id().as_bytes());
        let scope_identity = match binding.runtime_scope() {
            RuntimeScopeRefV1::Channel { channel_id, .. } => *channel_id.as_bytes(),
            RuntimeScopeRefV1::Instance { instance_id, .. } => instance_id,
        };
        digest.update(scope_identity);
        digest.update(binding.scope_generation().get().to_be_bytes());
        digest.update(proof.content());
        digest.update(proof.body_digest());
        digest.update(pin.client_artifact_digest());
        digest.update(pin.activation_sequence().to_be_bytes());
        digest.update(pin.frame_binding_digest());
        digest.update(runtime.map_revision_digest());
        let cast: [u8; 32] = digest.finalize().into();
        let content = pin.server_artifact_digest();
        let client = pin.client_artifact_digest();
        let activation = pin.activation_sequence();
        let map = runtime.map_revision_digest();
        let frame = pin.frame_binding_digest();
        let stamp = runtime
            .issue_owner_work()
            .map_err(|_| SpellItemError::Rejected("Spite real registered owner work"))?;
        let cause = ScopedGlobalGroundCause {
            action: GlobalFireAction::Create,
            identity: ScopedGlobalIdentity {
                scope_identity,
                scope: binding.runtime_scope(),
                ownership_generation: binding.scope_generation().get(),
                node: *binding.node_id().as_bytes(),
                node_registration_revision: binding.node_registration_revision(),
                assignment_revision: binding.source_revision(),
                stamp,
            },
            source_items: vec![TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i33877".into(),
                revision_ref: "definition-r1".into(),
            }],
            definition: TypedDefinitionRef {
                family: "Encounter".into(),
                production_key: proof.encounter_key().into(),
                revision_ref: "definition-r1".into(),
            },
            occurrence,
            content,
            client,
            activation,
            map,
            frame,
            body: proof.body_digest(),
            cast,
        };
        let mut item = [0; 16];
        item.copy_from_slice(&cast[..16]);
        item[6] = (item[6] & 15) | 0x70;
        item[8] = (item[8] & 63) | 0x80;
        let operation = SpellItemOperation::MintGround {
            item_instance_id: item,
            definition,
            quantity: 1,
            placement: target.placement(cause.occurrence.as_bytes().to_vec()),
            lifetime_millis: None,
            blocks_movement: false,
            blocks_projectile: false,
            description: None,
        };
        Ok(Self {
            registration: proof.clone(),
            execution_us: now_us,
            cause,
            operation: Some(operation),
            target,
            position,
            due_us,
        })
    }
    /// Capture the same registered timer at its original deadline. SQL retries
    /// retain this value, including execution time, instead of rebasing on UTC.
    pub fn prepare_current_expiry(
        &self,
        runtime: &mut ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        now_us: u64,
    ) -> Result<Self> {
        if self.cause.action != GlobalFireAction::Create
            || !self.current_for(runtime)
            || now_us < self.due_us
        {
            return Err(SpellItemError::Rejected(
                "Spite original registered expiry deadline",
            ));
        }
        // Reprove current geometry without fabricating a new field or actor.
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, self.position)
            .map_err(SpellItemError::Rejected)?;
        if target != self.target {
            return Err(SpellItemError::Rejected("Spite retained expiry geometry"));
        }
        let mut cause = self.cause.clone();
        cause.action = GlobalFireAction::Expire;
        cause.occurrence = format!(
            "{}:expire",
            self.cause
                .occurrence
                .strip_suffix(":create")
                .ok_or(SpellItemError::Rejected("Spite original create occurrence"))?
        );
        cause.cast = Sha256::new()
            .chain_update(b"oteryn:registered-spite-expiry:v1")
            .chain_update(self.cause.cast)
            .chain_update(cause.occurrence.as_bytes())
            .finalize()
            .into();
        cause.identity.stamp = runtime
            .issue_owner_work()
            .map_err(|_| SpellItemError::Rejected("Spite actual expiry owner work"))?;
        Ok(Self {
            registration: self.registration.clone(),
            execution_us: now_us,
            cause,
            operation: None,
            target,
            position: self.position,
            due_us: self.due_us,
        })
    }
    pub fn position(&self) -> MovementLocalPosition {
        self.position
    }
    pub fn due_us(&self) -> u64 {
        self.due_us
    }
    pub fn is_expiry(&self) -> bool {
        self.cause.action == GlobalFireAction::Expire
    }
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let binding = runtime.binding();
        let pin = runtime.content_pin();
        binding.runtime_scope() == self.cause.identity.scope
            && binding.scope_generation().get() == self.cause.identity.ownership_generation
            && *binding.node_id().as_bytes() == self.cause.identity.node
            && binding.node_registration_revision()
                == self.cause.identity.node_registration_revision
            && binding.source_revision() == self.cause.identity.assignment_revision
            && runtime
                .owner_fence()
                .is_ok_and(|owner| owner.accepts_stamp(self.cause.identity.stamp))
            && pin.server_artifact_digest() == self.cause.content
            && pin.client_artifact_digest() == self.cause.client
            && pin.activation_sequence() == self.cause.activation
            && pin.frame_binding_digest() == self.cause.frame
            && runtime.map_revision_digest() == self.cause.map
    }
}

pub async fn execute_spite_global_fire_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    callback: &SpiteGlobalFireCallback,
    prepared: &PreparedSpiteGlobalWrite,
) -> Result<SpiteGlobalFireReceipt> {
    if !prepared.matches(callback)
        || authority.runtime_scope() != callback.cause.identity.scope
        || authority.generation() != callback.cause.identity.ownership_generation
    {
        return Err(SpellItemError::Rejected(
            "Spite GlobalEvent current physical owner",
        ));
    }
    check_scope_transaction(tx, authority).await?;
    let cause = ScopedGroundCauseRef::Global(&callback.cause);
    let current = || {
        prepared
            .matches(callback)
            .then_some(())
            .ok_or(SpellItemError::Rejected(
                "Spite registered GlobalEvent current source generation",
            ))
    };
    // Reconcile before touching a current tile: retry cannot mint a second field
    // or reinterpret an already removed field as a new physical occurrence.
    if let Some(receipt) =
        reconcile_scoped_ground_in_transaction(tx, authority, cause, current).await?
    {
        return receipt_result(tx, callback, receipt).await;
    }
    let operations = if let Some(operation) = &callback.operation {
        vec![operation.clone()]
    } else {
        if callback.cause.action != GlobalFireAction::Expire
            || callback.execution_us < callback.due_us
        {
            return Err(SpellItemError::Rejected(
                "Spite retained expiry action/deadline",
            ));
        }
        // Source Tile:getItemById selects the first CURRENT matching Thing,
        // not necessarily the item minted by the original position callback.
        let tile = read_creature_scope_tile_in_transaction(tx, authority, &callback.target).await?;
        if let Some(item) = tile
            .items
            .iter()
            .find(|item| callback.cause.source_items.contains(&item.definition))
        {
            vec![SpellItemOperation::RemoveGround(
                prepare_creature_scope_removal_in_transaction(
                    tx,
                    authority,
                    &callback.target,
                    item.item_instance_id,
                )
                .await?,
            )]
        } else {
            Vec::new()
        }
    };
    let prepared = super::scoped_global::PreparedGlobalGroundOperations {
        scope_identity: callback.cause.identity.scope_identity,
        body: callback.cause.body,
        cast: callback.cause.cast,
        operations,
    };
    let identity = |domain: &[u8]| {
        let digest = Sha256::new()
            .chain_update(domain)
            .chain_update(callback.cause.cast)
            .chain_update(callback.cause.body)
            .chain_update(callback.cause.identity.scope_identity)
            .chain_update(callback.cause.identity.ownership_generation.to_be_bytes())
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
        identity(b"oteryn:registered-spite-global-transaction:v1"),
        identity(b"oteryn:registered-spite-global-event:v1"),
        ScopedGroundPreparedRef::Global(&prepared),
        current,
    )
    .await?;
    receipt_result(tx, callback, receipt).await
}

async fn receipt_result(
    tx: &mut Transaction<'_, Postgres>,
    callback: &SpiteGlobalFireCallback,
    receipt: CreatureGroundReceipt,
) -> Result<SpiteGlobalFireReceipt> {
    let count: i32 = sqlx::query_scalar(
        "SELECT operation_count FROM game_spell_item_receipts WHERE transaction_id=encode($1,'hex')::uuid",
    ).bind(receipt.transaction_id.as_slice()).fetch_one(&mut **tx).await?;
    if !(0..=1).contains(&count)
        || (callback.cause.action == GlobalFireAction::Create && count != 1)
    {
        return Err(SpellItemError::Rejected(
            "Spite immutable physical receipt count",
        ));
    }
    Ok(SpiteGlobalFireReceipt {
        receipt,
        removed: callback.cause.action == GlobalFireAction::Expire && count == 1,
    })
}

struct GlobalScopeContext<'a> {
    permit: &'a crate::durability::spell_owner_commit::SpellLanePermit,
    prepared: &'a PreparedSpiteGlobalWrite,
    callback: &'a SpiteGlobalFireCallback,
}

impl DurabilityRoot {
    /// Reuses the current Node/recovery/scope transaction without retaining a
    /// runtime guard. The caller keeps its physical lane, then rechecks captured
    /// scope/pins under the actual runtime before publishing memory effects.
    pub async fn commit_spite_global_fire(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        callback: &SpiteGlobalFireCallback,
        prepared: &PreparedSpiteGlobalWrite,
    ) -> Result<SpiteGlobalFireReceipt> {
        if !prepared.matches(callback)
            || node.fact().node_id().as_bytes() != &callback.cause.identity.node
            || node.fact().registration_revision()
                != callback.cause.identity.node_registration_revision
        {
            return Err(SpellItemError::Rejected(
                "Spite prepared current Node/source binding",
            ));
        }
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let scope = callback.cause.identity.scope;
        let generation = callback.cause.identity.ownership_generation;
        let mut context = GlobalScopeContext {
            permit,
            callback,
            prepared,
        };
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<SpiteGlobalFireReceipt> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            context.permit,
                            &record,
                            &node,
                            scope,
                            generation,
                        )
                        .await?;
                        let receipt = execute_spite_global_fire_in_transaction(
                            &mut tx,
                            &authority,
                            context.callback,
                            context.prepared,
                        )
                        .await?;
                        Ok(receipt)
                    }
                    .await;
                    match outcome {
                        Ok(value) => {
                            commit_creature_item_owner_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

impl oteryn_gameplay_entities::source_reservation_seal::Sealed for PreparedSpiteGlobalWrite {}
impl oteryn_gameplay_entities::SourceGlobalReservationProof for PreparedSpiteGlobalWrite {
    fn registration(
        &self,
    ) -> &crate::content::native_soul_war_global_events::QualifiedSpiteGlobalEvent {
        &self.registration
    }
    fn work_stamp(&self) -> oteryn_gameplay_foundation::foundation::RuntimeWorkStamp {
        self.identity.stamp
    }
    fn event_binding(&self) -> [u8; 32] {
        self.cast
    }
    fn execution_micros(&self) -> u64 {
        self.execution_us
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        PreparedSpiteGlobalWrite::current_for(self, runtime)
    }
}
