//! A field is inspected once for an actual STEP destination. Periodic standing
//! reads only pause existing DOT; they never install or refresh an entry condition.
use super::super::ComposedFreshAdmission;
use super::ChannelSpellStates;
use crate::content::{ReferenceItemField, ReferenceItemType};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_item_transaction::StandingTileRead;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, MovementLocalPosition,
    MovementPositionSnapshot, RuntimeScopeRefV1,
};
use crate::movement::{CardinalStep, MovementError};
use crate::spell::cast::PlayerSpellState;
use crate::spell::world_items_execution::SpellGroundTarget;

/// Detached read evidence grants no movement or damage authority. The private
/// constructor binds actual origin, source destination and predecessor owner.
struct StepIngressRead<'read, 'holder> {
    _transaction: &'read sqlx::Transaction<'holder, sqlx::Postgres>,
    actor: ExactActorRef,
    session: GameSessionId,
    direction: CardinalStep,
    origin: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    before: Option<PlayerSpellState>,
    read: StandingTileRead,
    world:
        Option<&'read crate::durability::spell_field_policy::WorldFieldPolicyRead<'read, 'holder>>,
    creator: Option<CurrentFieldCreator>,
    now_us: u64,
}
struct CurrentFieldCreator {
    actor: ExactActorRef,
    session: GameSessionId,
    lease: u64,
    level: u32,
}

impl StepIngressRead<'_, '_> {
    /// Called before any physical write, under the same actual owner locks as
    /// the STEP. All fallible field checks finish before movement admission.
    #[allow(
        clippy::too_many_arguments,
        reason = "the owner turn binds every independently resolved fact explicitly"
    )]
    fn validate_current(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        room: &crate::content::QualifiedNativeEntryRoom,
        source: &crate::content::native_gameplay::NativeGameplayState,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: CardinalStep,
        prepared: Option<&crate::movement::source_floor_change::PreparedCurrentSourceStep<'_>>,
    ) -> Result<(), MovementError> {
        let fence = self.read.fence();
        if actor != self.actor
            || session != self.session
            || direction != self.direction
            || runtime
                .read_actor_position(actor)
                .map_err(MovementError::Actor)?
                != self.origin
            || self.origin.context() != runtime.pinned_movement_context()
            || states.get(runtime, actor, session) != self.before.as_ref()
            || states.has_pending_spell_commit(actor, session)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|f| f.control_loss.is_none())
            || fence.game_session_id != session
            || fence.runtime_scope
                != RuntimeScopeRefV1::channel(
                    runtime.binding().world_id(),
                    runtime.binding().channel_id(),
                )
            || fence.scope_ownership_generation != runtime.binding().scope_generation()
            || self.read.content_digest() != runtime.content_pin().server_artifact_digest()
            || source.source_digest() != self.read.content_digest()
            || if let Some(prepared) = prepared {
                prepared.validate_current(runtime)?;
                prepared.destination() != self.destination
            } else {
                destination(
                    runtime,
                    room.movement_cells(),
                    actor,
                    session,
                    self.origin,
                    direction,
                )? != self.destination
            }
            || SpellGroundTarget::for_native_tile_read(room, runtime, self.destination)
                .map_err(|_| MovementError::NotQualified)?
                != *self.read.target()
        {
            return Err(MovementError::NotQualified);
        }
        let tile = if let Some(prepared) = prepared {
            prepared
                .tile(self.destination)
                .ok_or(MovementError::NotQualified)?
        } else {
            room.movement_cells()
                .spell_tiles()
                .lookup(
                    room.movement_cells().scope(),
                    crate::content::LogicalCell {
                        x: self.destination.x,
                        y: self.destination.y,
                        z: i32::from(self.destination.floor),
                    },
                )
                .map_err(|_| MovementError::NotQualified)?
        };
        // SQL rows are the actual ordered destination items. An expired row is
        // not silently erased here: only the durable expiry writer may remove it.
        let mut first_magic_field = true;
        for item in self.read.items() {
            if item.blocks_movement {
                return Err(MovementError::Blocked);
            }
            let policy = source
                .item_policy(
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                )
                .ok_or(MovementError::NotQualified)?;
            let record = policy.record();
            let ReferenceItemField::Known(classification) = &record.semantics.classification else {
                return Err(MovementError::NotQualified);
            };
            match classification.item_type {
                ReferenceItemField::Known(ReferenceItemType::MagicField) => {
                    if !first_magic_field {
                        continue;
                    }
                    first_magic_field = false;
                    let recipe = record
                        .attributes
                        .field_condition
                        .as_ref()
                        .ok_or(MovementError::NotQualified)?;
                    let values = recipe
                        .condition_values()
                        .map_err(|_| MovementError::NotQualified)?;
                    if values.is_some() && !tile.flags().protection_zone {
                        let creator = self.creator.as_ref().ok_or(MovementError::NotQualified)?;
                        // A current player owns this field, so source onStepInField
                        // attaches that owner in a NoPvP tile; canDoCombat rejects
                        // the initial hit. Other ordered rows still need inspection.
                        if tile.no_pvp_zone() == Some(true) {
                            continue;
                        }
                        let world = self.world.ok_or(MovementError::NotQualified)?;
                        if world.world() != runtime.binding().world_id() {
                            return Err(MovementError::NotQualified);
                        }
                        let target = self.before.as_ref().ok_or(MovementError::NotQualified)?;
                        let protected = creator.level < world.protection_level()
                            || target.character_facts().level < world.protection_level();
                        if protected {
                            // Source isProtected rejects before combat flags,
                            // equipment absorption or initial HP application.
                            continue;
                        }
                        if world.mode()
                            == crate::durability::spell_field_policy::FieldWorldType::NoPvp
                        {
                            let current = runtime
                                .read_actor_position(creator.actor)
                                .map_err(MovementError::Actor)?;
                            if current.context() != runtime.pinned_movement_context() {
                                return Err(MovementError::NotQualified);
                            }
                            let position = current.position();
                            let creator_tile = room
                                .movement_cells()
                                .spell_tiles()
                                .lookup(
                                    room.movement_cells().scope(),
                                    crate::content::LogicalCell {
                                        x: position.x,
                                        y: position.y,
                                        z: i32::from(position.floor),
                                    },
                                )
                                .map_err(|_| MovementError::NotQualified)?;
                            let source_pvp =
                                creator_tile.pvp_zone().ok_or(MovementError::NotQualified)?;
                            let target_pvp = tile.pvp_zone().ok_or(MovementError::NotQualified)?;
                            // Source NoPvP worlds still permit combat when both
                            // actual current tiles are PvP zones.
                            if !(source_pvp && target_pvp) {
                                continue;
                            }
                        }
                        let origin = item
                            .field_origin
                            .as_ref()
                            .ok_or(MovementError::NotQualified)?;
                        let age = self
                            .read
                            .observed_at_unix_ms()
                            .checked_sub(origin.created_at_unix_ms)
                            .filter(|v| *v >= 0)
                            .ok_or(MovementError::NotQualified)?;
                        let history = target
                            .source_field_attack_history(
                                creator.actor,
                                creator.session,
                                creator.lease,
                                self.now_us,
                                world.in_fight_ms(),
                            )
                            .ok_or(MovementError::NotQualified)?;
                        let _source_attaches_player_owner = world.mode()
                            == crate::durability::spell_field_policy::FieldWorldType::NoPvp
                            || age <= 5000
                            || history;
                        // World NoPvP alone is insufficient: source still allows
                        // player combat when both current tiles are PvP zones.
                        // Current combat flags/skull/faction and initial field
                        // equipment/Mantra absorption remain unavailable. Never
                        // replace a historical player source with environmental HP.
                        return Err(MovementError::NotQualified);
                    }
                    // The accepted current PZ condition owner refuses harmful
                    // initial application, so no DOT instance or HP mutation is
                    // installed. A recipe with zero ticks likewise has no effect.
                    continue;
                }
                ReferenceItemField::Known(_) | ReferenceItemField::NotApplicable => {}
                ReferenceItemField::Unknown | ReferenceItemField::Conflict => {
                    return Err(MovementError::NotQualified);
                }
            }
        }
        Ok(())
    }
}
fn destination(
    runtime: &ChannelRuntimeV1,
    cells: &crate::content::NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    origin: MovementPositionSnapshot,
    direction: CardinalStep,
) -> Result<MovementLocalPosition, MovementError> {
    if crate::movement::source_floor_change::is_source_profile(cells) {
        return Ok(crate::movement::source_floor_change::prepare_source_step(
            runtime, cells, actor, session, origin, direction,
        )?
        .destination());
    }
    let p = origin.position();
    let (dx, dy) = match direction {
        CardinalStep::North => (0, -1),
        CardinalStep::East => (1, 0),
        CardinalStep::South => (0, 1),
        CardinalStep::West => (-1, 0),
    };
    Ok(MovementLocalPosition {
        x: p.x
            .checked_add(dx)
            .ok_or(MovementError::CoordinateOverflow)?,
        y: p.y
            .checked_add(dy)
            .ok_or(MovementError::CoordinateOverflow)?,
        floor: p.floor,
    })
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Runtime and state locks are held by the real transport caller. Ground
    /// advisory and SQL row locks remain held through the synchronous physical
    /// movement commit. This read-only transaction creates no cast command.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport) async fn step_with_field_ingress(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        now_us: u64,
        direction: CardinalStep,
        blocking: &std::collections::BTreeSet<crate::content::LogicalCell>,
        equipment_delta: Option<i32>,
    ) -> super::actor_movement::StepInChannel {
        use super::actor_movement::step_in_channel;
        let fail = || Err(MovementError::NotQualified);
        let Some(source) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return step_in_channel(
                runtime,
                states,
                self.movement_cells,
                actor,
                session,
                now_us,
                direction,
                blocking,
                equipment_delta,
            );
        };
        let Some(_room) = self.qualified_room else {
            return fail();
        };
        let Ok(origin) = runtime.read_actor_position(actor) else {
            return fail();
        };
        // Preserve original accepted entry movement without inventing an empty
        // field owner. Field casts cannot qualify against its absent metadata.
        if !crate::movement::source_floor_change::is_source_profile(self.movement_cells)
            && matches!(
                self.movement_cells.spell_tiles().lookup(
                    self.movement_cells.scope(),
                    crate::content::LogicalCell {
                        x: origin.position().x,
                        y: origin.position().y,
                        z: i32::from(origin.position().floor)
                    }
                ),
                Err(crate::content::SpellTileLookupError::Unknown)
            )
        {
            return step_in_channel(
                runtime,
                states,
                self.movement_cells,
                actor,
                session,
                now_us,
                direction,
                blocking,
                equipment_delta,
            );
        }
        if source.source_digest() != runtime.content_pin().server_artifact_digest()
            || states.has_pending_spell_commit(actor, session)
        {
            return fail();
        }
        let before = states.get(runtime, actor, session).cloned();
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return fail();
        };
        // The physical result is retained before rollback/pool-return awaits.
        // Read-only transaction cleanup failure cannot turn an actual movement
        // commit into a reported rejection or trigger a second movement.
        let mut context = (self, runtime, states, blocking, None);
        let result=pass.run_with_context(&mut context,move|holder,deadline,ctx|Box::pin(async move{
            let(owner,runtime,states,blocking,outcome)=ctx;
            let mut tx=crate::durability::spell_item_transaction::begin_spell_owner_transaction(holder,deadline).await?;
            let current=FreshAdmissionStore::from_root(owner.root.clone()).current_session_in_transaction(&mut tx,session).await?;
            if current.session_state()!=GameSessionState::Active{return Err(crate::durability::DurabilityError::Unavailable)}
            let fence=CurrentCharacterItemFence{
                character_id:crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes()).map_err(|_|crate::durability::DurabilityError::Unavailable)?,
                game_session_id:session,connection_generation:current.current_connection_generation(),character_lease_generation:current.current_character_lease().generation(),runtime_scope:current.current_runtime_scope(),scope_ownership_generation:current.current_scope_generation(),
            };
            let source=owner.active_generation.and_then(|active|active.native_gameplay()).ok_or(crate::durability::DurabilityError::Unavailable)?;
            let room=owner.qualified_room.ok_or(crate::durability::DurabilityError::Unavailable)?;
            let objects=owner.door.lock().await;
            let prepared=if crate::movement::source_floor_change::is_source_profile(owner.movement_cells) {
                Some(crate::movement::source_floor_change::collect_current_source_step(&mut tx,owner.root,owner.character,owner.holder,&fence,room,runtime,&objects,source,actor,session,origin,direction).await.map_err(|_|crate::durability::DurabilityError::Unavailable)?)
            } else {None};
            let destination=if let Some(prepared)=&prepared {prepared.destination()} else {destination(runtime,owner.movement_cells,actor,session,origin,direction).map_err(|_|crate::durability::DurabilityError::Unavailable)?};
            let target=SpellGroundTarget::for_native_tile_read(room,runtime,destination).map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            let read=crate::durability::spell_item_transaction::read_standing_player_tile_in_transaction(&mut tx,owner.root,owner.character,owner.holder,&fence,source.source_digest(),&target).await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            let mut creator=None;
            for item in read.items(){
                let Some(policy)=source.item_policy(&item.definition.production_key,&item.definition.revision_ref)else{break};
                let ReferenceItemField::Known(class)=&policy.record().semantics.classification else{break};
                if matches!(class.item_type,ReferenceItemField::Known(ReferenceItemType::MagicField)){
                    if let Some(historical)=&item.field_origin {
                        let current_source=states.actors.iter().find(|(candidate,candidate_session,_)|*candidate_session.as_bytes()==historical.game_session_id&&candidate.placement_identity()==historical.actor_placement_digest).map(|(a,s,state)|(*a,*s,state.character_facts().level));
                        if let Some((source_actor,source_session,level))=current_source {
                            let fresh=FreshAdmissionStore::from_root(owner.root.clone()).current_session_in_transaction(&mut tx,source_session).await?;
                            if fresh.session_state()==GameSessionState::Active
                                &&fresh.commit().character_id().as_bytes()==&historical.character_id
                                &&fresh.current_character_lease().generation()==historical.character_lease_generation
                                &&fresh.current_runtime_scope()==fence.runtime_scope
                                &&fresh.current_scope_generation()==fence.scope_ownership_generation
                                &&historical.source_scope_generation==runtime.binding().scope_generation().get()
                                &&historical.content_digest==runtime.content_pin().server_artifact_digest()
                                &&runtime.player_control_facts(source_actor,source_session).is_ok_and(|f|f.control_loss.is_none()) {
                                creator=Some(CurrentFieldCreator{actor:source_actor,session:source_session,lease:historical.character_lease_generation,level});
                            }
                        }
                    }
                    break;
                }
            }
            let world_data=if creator.is_some(){crate::durability::spell_field_policy::read_world_field_policy_in_transaction(&mut tx,owner.root,fence.runtime_scope,fence.scope_ownership_generation.get()).await?}else{None};
            let world=world_data.map(|data|data.bind(&tx));
            let validation={
                let ingress=StepIngressRead{_transaction:&tx,actor,session,direction,origin,destination,before,read,world:world.as_ref(),creator,now_us};
                ingress.validate_current(runtime,states,room,source,actor,session,direction,prepared.as_ref())
            };
            drop(world);
            match validation {
                Ok(())=>{
                    let proof=if let Some(prepared)=prepared {Some(crate::movement::source_floor_change::bind_current_source_step(&mut tx,runtime,prepared).await.map_err(|_|crate::durability::DurabilityError::Unavailable)?)} else {None};
                    *outcome=Some(super::actor_movement::step_in_channel_with_source_step(runtime,states,owner.movement_cells,actor,session,now_us,direction,blocking,equipment_delta,proof));
                },
                Err(error)=>*outcome=Some(Err(error)),
            }
            tx.rollback().await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            Ok(())
        })).await;
        if let Some(outcome) = context.4 {
            outcome
        } else {
            let _ = result;
            fail()
        }
    }
}
