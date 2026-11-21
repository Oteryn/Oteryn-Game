//! Crystal Vemiath/Chagorz/Bakragore onThink ->37000/Action50001.
//! Same canonical physical Item writer, truthful CreatureEvent source, no fake spell.
use super::*;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, RuntimeScopeRefV1, RuntimeWorkStamp,
    ThinkOccurrence,
};
const ITEM: &str = "oteryn:item.tibia.i37000";
fn layout(key: &str) -> Option<(Room, MovementLocalPosition)> {
    Some(match key {
        "oteryn:creature.vemiath" => (
            Room::Vemiath,
            MovementLocalPosition {
                x: 33043,
                y: 32335,
                floor: 15,
            },
        ),
        "oteryn:creature.chagorz" => (
            Room::Chagorz,
            MovementLocalPosition {
                x: 33043,
                y: 32366,
                floor: 15,
            },
        ),
        "oteryn:creature.bakragore" => (
            Room::Bakragore,
            MovementLocalPosition {
                x: 33043,
                y: 32397,
                floor: 15,
            },
        ),
        _ => return None,
    })
}
#[derive(Debug)]
pub struct RottenPortalSource {
    stamp: RuntimeWorkStamp,
    scope: RuntimeScopeRefV1,
    generation: u64,
    content: [u8; 32],
    client: [u8; 32],
    map: [u8; 32],
    frame: [u8; 32],
    key: String,
    seed: [u8; 32],
}
impl RottenPortalSource {
    /// Captured only by the real reserved first onThink. The later global
    /// addEvent checks a CURRENT named boss; its original issuer may be gone.
    pub fn from_current_native_think(
        runtime: &mut ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        occurrence: &ThinkOccurrence,
    ) -> Result<Self> {
        let snapshot = runtime
            .companion_snapshot(occurrence.actor)
            .map_err(|_| SpellItemError::Rejected("Rotten portal current parent"))?;
        let key = snapshot.state.policy.definition_key.clone();
        let (kind, _) =
            layout(&key).ok_or(SpellItemError::Rejected("Rotten portal parent role"))?;
        if !matches!(
            runtime.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || room.pinned_activity_source_room() != Some(kind)
            || snapshot.health <= 0
            || snapshot.state.master.is_some()
            || runtime.native_summon_master(occurrence.actor).is_some()
            || !native
                .qualifies_rotten_blood_callbacks(runtime.content_pin().server_artifact_digest())
        {
            return Err(SpellItemError::Rejected(
                "Rotten portal native source room/closure",
            ));
        }
        let stamp = runtime
            .issue_owner_work()
            .map_err(|_| SpellItemError::Rejected("Rotten portal current owner"))?;
        let pin = runtime.content_pin();
        let seed = Sha256::new()
            .chain_update(b"oteryn:rotten-portal-source:v1")
            .chain_update(occurrence.actor.placement_identity())
            .chain_update(occurrence.actor.actor_local_generation().to_be_bytes())
            .chain_update(occurrence.sequence.to_be_bytes())
            .chain_update(pin.server_artifact_digest())
            .finalize()
            .into();
        Ok(Self {
            stamp,
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation().get(),
            content: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            map: pin.map_revision_digest(),
            frame: pin.frame_binding_digest(),
            key,
            seed,
        })
    }
    fn current(
        &self,
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
    ) -> bool {
        runtime
            .owner_fence()
            .is_ok_and(|fence| fence.accepts_stamp(self.stamp))
            && runtime.binding().runtime_scope() == self.scope
            && runtime.binding().scope_generation().get() == self.generation
            && runtime.content_pin().server_artifact_digest() == self.content
            && runtime.content_pin().client_artifact_digest() == self.client
            && runtime.map_revision_digest() == self.map
            && runtime.content_pin().frame_binding_digest() == self.frame
            && layout(&self.key)
                .is_some_and(|(kind, _)| room.pinned_activity_source_room() == Some(kind))
    }
    pub fn from_current_named_boss(
        &self,
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        parent: ExactActorRef,
        ordinal: u64,
        now_us: u64,
    ) -> Result<RottenPortalCallback> {
        if !self.current(runtime, room)
            || ordinal == 0
            || !native
                .qualifies_rotten_blood_callbacks(runtime.content_pin().server_artifact_digest())
        {
            return Err(SpellItemError::Rejected(
                "Rotten portal retained source fence",
            ));
        }
        let snapshot = runtime
            .companion_snapshot_including_dead(parent)
            .map_err(|_| SpellItemError::Rejected("Rotten portal current named boss"))?;
        if !runtime
            .source_creature_is_registered(parent)
            .map_err(|_| SpellItemError::Rejected("Rotten portal registered named boss"))?
            || snapshot.state.policy.definition_key != self.key
            || snapshot.state.master.is_some()
            || runtime.native_summon_master(parent).is_some()
        {
            return Err(SpellItemError::Rejected(
                "Rotten portal current named source role",
            ));
        }
        let (_, position) =
            layout(&self.key).ok_or(SpellItemError::Rejected("Rotten portal source centre"))?;
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, position)
            .map_err(SpellItemError::Rejected)?;
        let policy = native
            .item_policy(ITEM, "definition-r1")
            .ok_or(SpellItemError::Rejected("Rotten portal actual Item"))?;
        if !matches!(&policy.record().semantics.classification,crate::content::ReferenceItemField::Known(classification)
            if classification.item_type==crate::content::ReferenceItemField::Known(crate::content::ReferenceItemType::Teleport))
        {
            return Err(SpellItemError::Rejected(
                "Rotten portal actual Teleport type",
            ));
        }
        let attributes = &policy.record().attributes;
        let definition = QualifiedItemDefinition::from_native_policy(policy)
            .map_err(SpellItemError::Rejected)?;
        if !definition.materializable
            || !definition.ground_destination
            || definition.movable
            || definition.stack_maximum != 1
            || definition.container_capacity.is_some()
            || definition.decay.is_some()
            || !definition.decay_chain.is_empty()
            || attributes.blocks_movement != Some(false)
            || attributes.blocks_projectile != Some(false)
            || attributes.immovable_block_solid != Some(false)
        {
            return Err(SpellItemError::Rejected(
                "Rotten portal exact nondecaying source Item",
            ));
        }
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| {
                record.profile.target.key == self.key
                    && record.profile.target.revision == snapshot.state.policy.definition_revision
            })
            .ok_or(SpellItemError::Rejected("Rotten portal source body"))?;
        let body = Sha256::digest(
            serde_json::to_vec(record)
                .map_err(|_| SpellItemError::Rejected("Rotten portal source encoding"))?,
        )
        .into();
        let cast = Sha256::new()
            .chain_update(b"oteryn:rotten-portal-create:v1")
            .chain_update(self.seed)
            .chain_update(parent.placement_identity())
            .chain_update(parent.actor_local_generation().to_be_bytes())
            .chain_update(ordinal.to_be_bytes())
            .chain_update(now_us.to_be_bytes())
            .finalize()
            .into();
        let creature = TypedDefinitionRef {
            family: "Creature".into(),
            production_key: self.key.clone(),
            revision_ref: "definition-r1".into(),
        };
        let cause = CreatureGroundCause {
            world: *parent.world_id().as_bytes(),
            scope: self.scope,
            scope_generation: self.generation,
            source_items: vec![TypedDefinitionRef {
                family: "Item".into(),
                production_key: ITEM.into(),
                revision_ref: "definition-r1".into(),
            }],
            placement: parent.placement_identity(),
            generation: parent.actor_local_generation(),
            ability: creature.clone(),
            creature,
            event_rule: Some(CreatureEventRule::RottenPortalCreate),
            occurrence: format!("rotten-portal:{}:{ordinal}:create", hex(&self.seed)),
            content: self.content,
            map: self.map,
            frame: self.frame,
            body,
            cast,
        };
        let item = super::primal_pod::identity(&cause, b"oteryn:rotten-portal-item:v1");
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
        Ok(RottenPortalCallback {
            cause,
            target,
            position,
            due_us: now_us,
            operation: Some(operation),
            item,
        })
    }
}
pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[derive(Debug)]
pub struct RottenPortalCallback {
    cause: CreatureGroundCause,
    target: SpellGroundTarget,
    position: MovementLocalPosition,
    due_us: u64,
    operation: Option<SpellItemOperation>,
    item: [u8; 16],
}
impl RottenPortalCallback {
    pub fn runtime_scope(&self) -> RuntimeScopeRefV1 {
        self.cause.scope
    }
    pub fn scope_generation(&self) -> u64 {
        self.cause.scope_generation
    }
    pub fn position(&self) -> MovementLocalPosition {
        self.position
    }
    pub fn due_us(&self) -> u64 {
        self.due_us
    }
    pub fn is_create(&self) -> bool {
        self.cause.event_rule == Some(CreatureEventRule::RottenPortalCreate)
    }
    pub fn item_instance_id(&self) -> [u8; 16] {
        self.item
    }
    /// Only the committed movement owner may supply this current step snapshot.
    pub fn from_committed_player_step(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
        committed: crate::foundation::MovementPositionSnapshot,
        now_us: u64,
    ) -> Result<Self> {
        if !self.is_create()
            || runtime.binding().runtime_scope() != self.cause.scope
            || runtime.binding().scope_generation().get() != self.cause.scope_generation
            || runtime.content_pin().server_artifact_digest() != self.cause.content
            || runtime.map_revision_digest() != self.cause.map
            || runtime.content_pin().frame_binding_digest() != self.cause.frame
            || runtime.read_actor_position(actor).ok() != Some(committed)
            || committed.position() != self.position
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Err(SpellItemError::Rejected(
                "Rotten portal exact committed Player step",
            ));
        }
        runtime
            .owner_fence()
            .map_err(|_| SpellItemError::Rejected("Rotten portal step current owner"))?;
        runtime
            .bound_attacker_lease(actor, session)
            .map_err(|_| SpellItemError::Rejected("Rotten portal step current lease"))?;
        let mut cause = self.cause.clone();
        cause.event_rule = Some(CreatureEventRule::RottenPortalStep);
        cause.occurrence.push_str(&format!(
            ":step:{}:{now_us}",
            hex(&actor.placement_identity())
        ));
        cause.cast = Sha256::new()
            .chain_update(b"oteryn:rotten-portal-player-step:v1")
            .chain_update(self.cause.cast)
            .chain_update(actor.placement_identity())
            .chain_update(actor.actor_local_generation().to_be_bytes())
            .chain_update(session.as_bytes())
            .chain_update(now_us.to_be_bytes())
            .finalize()
            .into();
        Ok(Self {
            cause,
            target: self.target.clone(),
            position: self.position,
            due_us: now_us,
            operation: None,
            item: self.item,
        })
    }
    pub fn expiry(&self) -> Result<Self> {
        if !self.is_create() {
            return Err(SpellItemError::Rejected("Rotten portal expiry source"));
        }
        let mut cause = self.cause.clone();
        cause.event_rule = Some(CreatureEventRule::RottenPortalExpire);
        cause.occurrence.push_str(":expire");
        cause.cast = Sha256::new()
            .chain_update(b"oteryn:rotten-portal-expire:v1")
            .chain_update(self.cause.cast)
            .finalize()
            .into();
        Ok(Self {
            cause,
            target: self.target.clone(),
            position: self.position,
            due_us: self
                .due_us
                .checked_add(15_000_000)
                .ok_or(SpellItemError::Rejected("Rotten portal expiry overflow"))?,
            operation: None,
            item: self.item,
        })
    }
}
pub(super) async fn accepted(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    cause: &CreatureGroundCause,
) -> Result<()> {
    check_scope_transaction(tx, authority).await?;
    let Some(instance) = cause.instance_bytes() else {
        return Err(SpellItemError::Rejected("Rotten portal Instance"));
    };
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id) JOIN game_boss_room_activities a ON a.admission=g.handoff_id WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND g.ownership_generation=$3::text::numeric(20,0) AND g.state=2 AND e.destination_server_artifact_digest=$4 AND e.destination_map_revision_digest=$5 AND e.destination_coordinate_frame_digest=$6 AND a.boss_creature_key=$7)")
        .bind(cause.world.as_slice()).bind(instance.as_slice()).bind(cause.scope_generation.to_string()).bind(cause.content.as_slice())
        .bind(cause.map.as_slice()).bind(cause.frame.as_slice()).bind(&cause.creature.production_key).fetch_one(&mut **tx).await?;
    if !active {
        return Err(SpellItemError::Rejected(
            "Rotten portal current accepted activity/source pin",
        ));
    }
    Ok(())
}
pub async fn execute_rotten_portal_callback_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    callback: &RottenPortalCallback,
    now_us: u64,
) -> Result<CreatureGroundReceipt> {
    if now_us < callback.due_us
        || authority.runtime_scope() != callback.cause.scope
        || authority.generation() != callback.cause.scope_generation
    {
        return Err(SpellItemError::Rejected(
            "Rotten portal current callback due/scope",
        ));
    }
    accepted(tx, authority, &callback.cause).await?;
    if let Some(receipt) =
        reconcile_creature_ground_in_transaction(tx, authority, &callback.cause, || Ok(())).await?
    {
        return Ok(receipt);
    }
    let operations = if let Some(operation) = &callback.operation {
        vec![operation.clone()]
    } else {
        // Lua reads CURRENT Tile(center):getItemById37000; never the captured old Item.
        let current =
            read_creature_scope_tile_in_transaction(tx, authority, &callback.target).await?;
        if let Some(item) = current.items.iter().find(|item| {
            item.definition.family == "Item"
                && item.definition.production_key == ITEM
                && item.definition.revision_ref == "definition-r1"
        }) {
            vec![SpellItemOperation::RemoveGround(
                prepare_creature_scope_removal_in_transaction(
                    tx,
                    authority,
                    &callback.target,
                    item.item_instance_id,
                )
                .await?,
            )]
        } else if callback.cause.event_rule == Some(CreatureEventRule::RottenPortalStep) {
            return Err(SpellItemError::Rejected(
                "Rotten portal stepped current Item absent",
            ));
        } else {
            Vec::new()
        }
    };
    let prepared = PreparedCreatureGroundOperations {
        placement: callback.cause.placement,
        generation: callback.cause.generation,
        body: callback.cause.body,
        cast: callback.cause.cast,
        operations,
    };
    apply_creature_ground_in_transaction(
        tx,
        authority,
        &callback.cause,
        super::primal_pod::identity(&callback.cause, b"oteryn:rotten-portal-transaction:v1"),
        super::primal_pod::identity(&callback.cause, b"oteryn:rotten-portal-event:v1"),
        &prepared,
        || Ok(()),
    )
    .await
}
struct PortalContext<'a> {
    permit: &'a crate::durability::spell_owner_commit::SpellLanePermit,
    callback: &'a RottenPortalCallback,
    now_us: u64,
}
impl DurabilityRoot {
    pub async fn commit_rotten_portal_callback(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        callback: &RottenPortalCallback,
        now_us: u64,
    ) -> Result<CreatureGroundReceipt> {
        let record = creature_item_recovery_record(recovery, self)?;
        let node = node.clone();
        let scope = callback.runtime_scope();
        let generation = callback.scope_generation();
        let mut context = PortalContext {
            permit,
            callback,
            now_us,
        };
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let outcome: Result<CreatureGroundReceipt> = async {
                        let authority = assert_spell_item_scope_with_recovery(
                            &mut tx,
                            context.permit,
                            &record,
                            &node,
                            scope,
                            generation,
                        )
                        .await?;
                        execute_rotten_portal_callback_in_transaction(
                            &mut tx,
                            &authority,
                            context.callback,
                            context.now_us,
                        )
                        .await
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
