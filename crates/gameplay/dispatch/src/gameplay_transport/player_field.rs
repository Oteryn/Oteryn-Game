//! Player field HP/condition successors join the existing PvP receipt and death
//! owners. Every tick resolves the current source; lineage is evidence only.
use super::{ChannelSpellStates, PlayerDeath};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_items_abi::{DurableFieldOrigin, SpellGroundTarget};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::{DurabilityError, spell_item_transaction as storage};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::spell::cast::PlayerSpellState;
use oteryn_gameplay_foundation::foundation::condition::{
    ConditionDefinition, ConditionLineage, ConditionTick, TickKind,
};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(super) struct PlayerFieldContact {
    pub(super) origin: DurableFieldOrigin,
    pub(super) item: [u8; 16],
    pub(super) definition: ConditionDefinition,
    pub(super) element: oteryn_gameplay_domain::domain::condition::DotElement,
    pub(super) attributed: bool,
}

#[derive(Debug)]
pub(in crate::gameplay_transport) struct PendingPlayerField {
    actor: ExactActorRef,
    session: GameSessionId,
    before: PlayerSpellState,
    next: PlayerSpellState,
    condition_next: PlayerSpellState,
    ticks: Vec<ConditionTick<String>>,
    position: MovementLocalPosition,
    death: Option<PlayerDeath>,
    fence: CurrentCharacterItemFence,
    content: [u8; 32],
    target: SpellGroundTarget,
    receipts: Vec<([u8; 32], [u8; 32])>,
    cycle: [u8; 32],
    binding: [u8; 32],
    locks: Vec<(ExactActorRef, GameSessionId, bool)>,
    now_us: u64,
    initial: bool,
    qualified: bool,
}

pub(in crate::gameplay_transport) fn park(
    pending: PendingPlayerField,
) -> Box<dyn std::any::Any + Send> {
    Box::new(super::native_combat_cast::UnresolvedSpellCommit::PlayerField(Box::new(pending)))
}

fn lineage(origin: &DurableFieldOrigin) -> ConditionLineage {
    ConditionLineage {
        source_character: origin.character_id,
        source_session: origin.game_session_id,
        source_lease_generation: origin.character_lease_generation,
        source_actor_placement: origin.actor_placement_digest,
        source_scope_generation: origin.source_scope_generation,
        source_content_digest: origin.content_digest,
        source_catalog_digest: origin.catalog_digest,
        source_creation_command: origin.creation_command_id,
    }
}

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn stage_player_field_entry(
        &self,
        tx: &mut Type2Transaction<'_>,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        read: &storage::StandingTileRead,
        actor: ExactActorRef,
        session: GameSessionId,
        destination: MovementLocalPosition,
        now_us: u64,
        contact: PlayerFieldContact,
    ) -> Result<Option<PendingPlayerField>, DurabilityError> {
        let before = states
            .get(runtime, actor, session)
            .ok_or(DurabilityError::Unavailable)?
            .clone();
        let mut source = format!(
            "player:field:{}",
            contact
                .item
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        if !contact.attributed {
            source.push_str(":unattributed");
        }
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
            contact.origin.content_digest,
        );
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&Sha256::digest(source.as_bytes())[..16]);
        let facts = oteryn_gameplay_foundation::foundation::condition::ApplicationFacts {
            now: now_us,
            base_speed: u16::try_from(before.owned_base_speed())
                .map_err(|_| DurabilityError::Unavailable)?,
            mana_shield_capacity: 0,
            target_reentry_protected: runtime
                .current_player_reentry_protection(actor, session, now_us)
                .map_err(|_| DurabilityError::Unavailable)?,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(occurrence),
        };
        let (next, ticks) = before
            .stage_player_field_entry(
                source,
                &contact.definition,
                &lineage(&contact.origin),
                &facts,
                contact.element,
            )
            .ok_or(DurabilityError::Unavailable)?;
        let pending = self
            .stage_player_field_ticks(
                tx,
                permit,
                runtime,
                states,
                read,
                actor,
                session,
                destination,
                now_us,
                true,
                before,
                next,
                ticks,
            )
            .await?;
        // A rejected initial combat application must not leave a condition
        // that begins hurting once immunity or a PvP restriction expires.
        if !pending.qualified {
            Ok(None)
        } else {
            Ok(Some(pending))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn stage_player_field_ticks(
        &self,
        tx: &mut Type2Transaction<'_>,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        read: &storage::StandingTileRead,
        actor: ExactActorRef,
        session: GameSessionId,
        position: MovementLocalPosition,
        now_us: u64,
        initial: bool,
        before: PlayerSpellState,
        mut next: PlayerSpellState,
        ticks: Vec<ConditionTick<String>>,
    ) -> Result<PendingPlayerField, DurabilityError> {
        read.check_transaction(tx)
            .await
            .map_err(|_| DurabilityError::Unavailable)?;
        if !crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
            .check_current_runtime_capabilities(
                tx,
                read.fence().runtime_scope,
                read.fence().scope_ownership_generation.get(),
                &crate::gameplay_transport::world_required_field_guard::current_check(
                    &self.required_capabilities,
                ),
            )
            .await?
        {
            return Err(DurabilityError::Unavailable);
        }
        let room = self
            .rooms
            .gate_periodic_damage()
            .ok_or(DurabilityError::Unavailable)?;
        let objects = self.door.lock().await;
        let mut qualified = false;
        let mut receipts = Vec::new();
        let mut locks = Vec::new();
        let cycle = Sha256::new()
            .chain_update(b"oteryn:player-field-cycle:v1")
            .chain_update(actor.placement_identity())
            .chain_update(session.as_bytes())
            .chain_update(before.revision().to_be_bytes())
            .chain_update(now_us.to_be_bytes())
            .finalize()
            .into();
        let condition_next = next.clone();
        for tick in &ticks {
            let (amount, refused, agony) = match tick.kind {
                TickKind::Damage {
                    amount, refused, ..
                } => (amount, refused, false),
                TickKind::Agony { amount, refused } => (amount, refused, true),
                _ => return Err(DurabilityError::Unavailable),
            };
            if refused {
                continue;
            }
            // PAR-M10: environment drown has no source actor; it is dealt like a creature hit.
            if !agony
                && crate::gameplay_transport::paced_movement::underwater_drown::is_environment_drown(
                    &tick.provenance,
                )
            {
                next = crate::spell::actor_conditions::stage_creature_hit(&next, amount, now_us)
                    .map_err(|_| DurabilityError::Unavailable)?
                    .0;
                if next.vitals().health == 0 {
                    break;
                }
                continue;
            }
            if tick.provenance.source_kind
                == oteryn_gameplay_foundation::foundation::condition::ConditionSourceKind::Creature
            {
                let source = tick
                    .provenance
                    .source
                    .as_deref()
                    .filter(|key| key.starts_with("creature:"))
                    .ok_or(DurabilityError::Unavailable)?;
                if source.starts_with("creature:field:") {
                    let origin=crate::durability::spell_items_abi::DurableCreatureFieldOrigin::from_condition_source(source).ok_or(DurabilityError::Unavailable)?;
                    let content = self
                        .active_generation
                        .and_then(|generation| generation.native_gameplay())
                        .ok_or(DurabilityError::Unavailable)?;
                    if !origin.matches_current_content(runtime, content) {
                        return Err(DurabilityError::Unavailable);
                    }
                }
                let amount = if let TickKind::Damage { element, .. } = tick.kind
                    && source.starts_with("creature:field:")
                {
                    self.equipped_field_damage(tx, read, element, amount, initial, cycle)
                        .await?
                } else {
                    amount
                };
                next = if agony {
                    crate::spell::actor_conditions::stage_agony_hit(&next, amount, now_us)
                } else {
                    crate::spell::actor_conditions::stage_creature_hit(&next, amount, now_us)
                }
                .map_err(|_| DurabilityError::Unavailable)?
                .0;
                if next.vitals().health == 0 {
                    break;
                }
                continue;
            }
            if agony
                || tick.provenance.source_kind
                    != oteryn_gameplay_foundation::foundation::condition::ConditionSourceKind::Field
            {
                return Err(DurabilityError::Unavailable);
            }
            let historical = tick
                .provenance
                .lineage
                .as_ref()
                .ok_or(DurabilityError::Unavailable)?;
            if historical.source_content_digest != runtime.content_pin().server_artifact_digest()
                || historical.source_catalog_digest
                    != runtime.content_pin().server_artifact_digest()
                || historical.source_scope_generation != runtime.binding().scope_generation().get()
                || !tick
                    .provenance
                    .source
                    .as_deref()
                    .is_some_and(|source| source.starts_with("player:field:"))
            {
                return Err(DurabilityError::Unavailable);
            }
            let candidate = states
                .actors
                .iter()
                .find(|(source, session, _)| {
                    source.placement_identity() == historical.source_actor_placement
                        && *session.as_bytes() == historical.source_session
                })
                .map(|(source, session, _)| (*source, *session));
            let current_source = if let Some((source, source_session)) = candidate {
                if states.is_dead(source)
                    || !runtime
                        .player_control_facts(source, source_session)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                {
                    None
                } else {
                    let current =
                        crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                            self.root.clone(),
                        )
                        .current_session_in_transaction(tx, source_session)
                        .await?;
                    crate::gameplay_transport::item_fence_of(current)
                        .filter(|fence| {
                            *fence.character_id.as_bytes() == historical.source_character
                                && fence.character_lease_generation
                                    == historical.source_lease_generation
                                && fence.runtime_scope == read.fence().runtime_scope
                                && fence.scope_ownership_generation
                                    == read.fence().scope_ownership_generation
                        })
                        .map(|fence| (source, source_session, fence))
                }
            } else {
                None
            };
            let Some((source, source_session, fence)) = current_source else {
                // A departed caster cannot grant PvP authority or attribution. The
                // retained field is environmental damage under the current victim's
                // own fence, current World policy and qualified tile instead.
                let policy =
                    crate::durability::spell_field_policy::read_world_field_policy_in_transaction(
                        tx,
                        self.root,
                        read.fence().runtime_scope,
                        read.fence().scope_ownership_generation.get(),
                    )
                    .await?
                    .ok_or(DurabilityError::Unavailable)?;
                let policy = policy.bind(tx);
                let cells = room.movement_cells();
                let tile = cells
                    .spell_tiles()
                    .lookup(
                        cells.scope(),
                        crate::content::LogicalCell {
                            x: position.x,
                            y: position.y,
                            z: i32::from(position.floor),
                        },
                    )
                    .map_err(|_| DurabilityError::Unavailable)?;
                if policy.mode() != crate::durability::spell_field_policy::FieldWorldType::NoPvp
                    && before.character_facts().level >= policy.protection_level()
                    && !tile.flags().protection_zone
                    && tile.no_pvp_zone() == Some(false)
                {
                    let TickKind::Damage { element, .. } = tick.kind else {
                        return Err(DurabilityError::Unavailable);
                    };
                    qualified = true;
                    let magnitude = self
                        .equipped_field_damage(tx, read, element, amount, initial, cycle)
                        .await?;
                    if initial && magnitude == 0 {
                        next.conditions = before.conditions.clone();
                    }
                    next = crate::spell::actor_conditions::stage_creature_hit(
                        &next, magnitude, now_us,
                    )
                    .map_err(|_| DurabilityError::Unavailable)?
                    .0;
                }
                if next.vitals().health == 0 {
                    break;
                }
                continue;
            };
            let command = crate::foundation::CommandRef::new(
                source_session,
                crate::foundation::CommandId::new(historical.source_creation_command)
                    .map_err(|_| DurabilityError::Unavailable)?,
            );
            let authority = storage::assert_spell_item_authority_in_transaction(
                tx,
                permit,
                self.root,
                self.character,
                self.holder,
                &fence,
                command,
                historical.source_content_digest,
            )
            .await
            .map_err(|_| DurabilityError::Unavailable)?;
            let attributed = !tick
                .provenance
                .source
                .as_deref()
                .is_some_and(|source| source.ends_with(":unattributed"));
            if source == actor {
                let policy =
                    crate::durability::spell_field_policy::read_world_field_policy_in_transaction(
                        tx,
                        self.root,
                        authority.runtime_scope(),
                        authority.scope_generation(),
                    )
                    .await?
                    .ok_or(DurabilityError::Unavailable)?;
                let policy = policy.bind(tx);
                let permitted = policy.mode()
                    == crate::durability::spell_field_policy::FieldWorldType::PvpEnforced
                    && before.character_facts().level >= policy.protection_level();
                if permitted {
                    let tile = crate::spell::world_execution::qualified_combat_tile_in_transaction(
                        tx, &authority, room, runtime, &objects, position,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    if !tile.protection_zone() && tile.no_pvp_zone() == Some(false) {
                        let black:bool=sqlx::query_scalar("SELECT COALESCE((SELECT skull=2 AND skull_until>clock_timestamp() FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid),false)").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
                        let magnitude = if attributed {
                            oteryn_gameplay_combat_rules::pvp::pvp_damage(amount, black)
                        } else {
                            amount
                        };
                        let TickKind::Damage { element, .. } = tick.kind else {
                            return Err(DurabilityError::Unavailable);
                        };
                        qualified = true;
                        let magnitude = self
                            .equipped_field_damage(tx, read, element, magnitude, initial, cycle)
                            .await?;
                        if initial && magnitude == 0 {
                            next.conditions = before.conditions.clone();
                        }
                        next = crate::spell::actor_conditions::stage_creature_hit(
                            &next, magnitude, now_us,
                        )
                        .map_err(|_| DurabilityError::Unavailable)?
                        .0;
                    }
                }
                if next.vitals().health == 0 {
                    break;
                }
                continue;
            }
            let facts = crate::gameplay_transport::pvp_live::legal_player_field(
                tx, self.root, &authority, runtime, states, room, &objects, source, actor, position,
            )
            .await;
            let Ok(facts) = facts else { continue };
            qualified = true;
            let TickKind::Damage { element, .. } = tick.kind else {
                return Err(DurabilityError::Unavailable);
            };
            if !attributed {
                let amount = self
                    .equipped_field_damage(tx, read, element, amount, initial, cycle)
                    .await?;
                if initial && amount == 0 {
                    next.conditions = before.conditions.clone();
                }
                next = crate::spell::actor_conditions::stage_creature_hit(&next, amount, now_us)
                    .map_err(|_| DurabilityError::Unavailable)?
                    .0;
                if next.vitals().health == 0 {
                    break;
                }
                continue;
            }
            let magnitude =
                oteryn_gameplay_combat_rules::pvp::pvp_damage(amount, facts.target_black);
            let magnitude = self
                .equipped_field_damage(tx, read, element, magnitude, initial, cycle)
                .await?;
            if initial && magnitude == 0 {
                next.conditions = before.conditions.clone();
            }
            let mana_before = next.vitals().mana;
            let (successor, hit) =
                crate::spell::actor_conditions::stage_creature_hit(&next, magnitude, now_us)
                    .map_err(|_| DurabilityError::Unavailable)?;
            next = successor;
            let damage = hit
                .applied
                .checked_add(
                    mana_before
                        .checked_sub(next.vitals().mana)
                        .ok_or(DurabilityError::Unavailable)?,
                )
                .ok_or(DurabilityError::Unavailable)?;
            next.stage_committed_player_hit_history(
                source,
                source_session,
                historical.source_lease_generation,
                now_us,
            )
            .map_err(|_| DurabilityError::Unavailable)?;
            let occurrence: [u8; 32] = Sha256::new()
                .chain_update(b"oteryn:player-field-tick:v1")
                .chain_update(actor.placement_identity())
                .chain_update(session.as_bytes())
                .chain_update(before.revision().to_be_bytes())
                .chain_update(tick.sequence.to_be_bytes())
                .chain_update(tick.due.to_be_bytes())
                .chain_update(
                    tick.provenance
                        .source
                        .as_deref()
                        .unwrap_or_default()
                        .as_bytes(),
                )
                .finalize()
                .into();
            let binding: [u8; 32] = Sha256::digest(
                format!(
                    "{tick:?}:{}:{:?}:{:?}",
                    before.revision(),
                    before.vitals(),
                    next.vitals()
                )
                .as_bytes(),
            )
            .into();
            crate::durability::pvp_live::record_field_pvp_harm_in_transaction(
                tx, &authority, &facts, occurrence, binding, damage,
            )
            .await
            .map_err(|_| DurabilityError::Unavailable)?;
            receipts.push((occurrence, binding));
            locks.push((
                source,
                source_session,
                !facts.relation.target_aggressor_of_actor,
            ));
            locks.push((actor, session, false));
            if next.vitals().health == 0 {
                break;
            }
        }
        next.source_set_batch_revision(
            before
                .revision()
                .checked_add(1)
                .ok_or(DurabilityError::Unavailable)?,
        );
        states
            .deaths
            .try_reserve(1)
            .map_err(|_| DurabilityError::Unavailable)?;
        states
            .pvp_locks
            .try_reserve(locks.len())
            .map_err(|_| DurabilityError::Unavailable)?;
        let death = if next.vitals().health == 0 {
            Some(PlayerDeath {
                occurrence: (states.mint_death)().ok_or(DurabilityError::Unavailable)?,
                cell: position,
            })
        } else {
            None
        };
        let binding = Sha256::digest(
            format!(
                "{}:{:?}:{:?}:{:?}:{:?}:{position:?}:{ticks:?}",
                before.revision(),
                before.vitals(),
                before.owned_conditions(),
                next.vitals(),
                next.owned_conditions()
            )
            .as_bytes(),
        )
        .into();
        crate::durability::pvp_live::record_field_cycle_in_transaction(tx, read, cycle, binding)
            .await
            .map_err(|_| DurabilityError::Unavailable)?;
        Ok(PendingPlayerField {
            actor,
            session,
            before,
            next,
            condition_next,
            ticks,
            position,
            death,
            fence: *read.fence(),
            content: read.content_digest(),
            target: read.target().clone(),
            receipts,
            cycle,
            binding,
            locks,
            now_us,
            initial,
            qualified,
        })
    }

    pub(in crate::gameplay_transport) async fn resolve_player_field(
        &self,
        permit: &mut SpellLanePermit,
        pending: PendingPlayerField,
    ) {
        let mut window = permit.open_commit_window(pending, park);
        window.mark_already_committed();
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return;
        };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let mut context = (self, &mut *runtime, &mut *states, &mut window);
        let result = pass
            .run_with_context(&mut context, |holder, deadline, context| {
                Box::pin(async move {
                    let (owner, runtime, states, window) = context;
                    let mut tx = crate::durability::spell_item_transaction::begin_field_owner_transaction(holder,deadline).await?;
                    let pending = window.attempt();
                    let read = storage::read_standing_player_tile_in_transaction(
                        &mut tx,
                        window.permit(),
                        owner.root,
                        owner.character,
                        owner.holder,
                        &pending.fence,
                        pending.content,
                        &pending.target,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let committed = crate::durability::pvp_live::field_cycle_matches_in_transaction(
                        &mut tx, &read, pending.cycle, pending.binding,
                    ).await.map_err(|_|DurabilityError::Unavailable)?;
                    if committed { for (occurrence, binding) in &pending.receipts {
                        if !
                            crate::durability::pvp_live::field_pvp_receipt_matches_in_transaction(
                                &mut tx,
                                &read,
                                *occurrence,
                                *binding,
                            )
                            .await
                            .map_err(|_| DurabilityError::Unavailable)? { return Err(DurabilityError::Unavailable); }
                    } }
                    if !committed {
                        // The physical STEP already happened. Retry the same
                        // retained condition occurrence after a proven rollback,
                        // re-reading every current source/PvP fence, without
                        // issuing movement or consuming a second condition tick.
                        let replacement = owner
                            .stage_player_field_ticks(
                                &mut tx,
                                window.permit(),
                                runtime,
                                states,
                                &read,
                                pending.actor,
                                pending.session,
                                pending.position,
                                pending.now_us,
                                pending.initial,
                                pending.before.clone(),
                                pending.condition_next.clone(),
                                pending.ticks.clone(),
                            )
                            .await;
                        let replacement = match replacement {
                            Ok(replacement) => replacement,
                            Err(_) => {
                                // Rollback is now proven. A lost current source
                                // cannot turn this retained occurrence into a new
                                // grant, or hold every other channel writer forever.
                                tx.into_inner().rollback().await?;
                                return Ok(false);
                            }
                        };
                        if replacement.next != pending.next
                            || replacement.receipts != pending.receipts
                            || replacement.cycle != pending.cycle
                            || replacement.binding != pending.binding
                        {
                            // Absence of the cycle under the scope lock proved
                            // rollback. Current legality may have changed since
                            // the physical step; discard the obsolete successor
                            // instead of holding the entire channel lane forever.
                            tx.into_inner().rollback().await?;
                            return Ok(false);
                        }
                        *window.attempt_mut() = replacement;
                        crate::durability::pvp_live::commit_live_pvp_transaction(
                            tx.into_inner(), deadline, window,
                        )
                        .await?;
                    } else {
                        tx.into_inner().rollback().await?;
                    }
                    Ok(true)
                })
            })
            .await;
        match result {
            Ok(true) => {
                if window.attempt().matches_current(&runtime, &states) {
                    window.install().install(&mut runtime, &mut states);
                }
            }
            Ok(false) => {
                window.release();
            }
            Err(_) => {}
        }
    }
}

impl PendingPlayerField {
    pub(super) fn retain_step_cooldowns(
        &mut self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
    ) {
        if let Some(current) = states.get(runtime, self.actor, self.session) {
            self.before.cooldowns = current.cooldowns.clone();
            self.next.cooldowns = current.cooldowns.clone();
            self.condition_next.cooldowns = current.cooldowns.clone();
        }
    }
    pub(super) fn matches_current(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
    ) -> bool {
        states.get(runtime, self.actor, self.session) == Some(&self.before)
            && runtime
                .player_control_facts(self.actor, self.session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            && runtime
                .read_actor_position(self.actor)
                .is_ok_and(|current| {
                    current.position() == self.position
                        && current.context() == runtime.pinned_movement_context()
                })
            && self.fence.scope_ownership_generation == runtime.binding().scope_generation()
            && runtime.content_pin().server_artifact_digest() == self.content
    }
    pub(super) fn install(self, runtime: &mut ChannelRuntimeV1, states: &mut ChannelSpellStates) {
        if !self.matches_current(runtime, states) {
            return;
        }
        if !states.commit(runtime, self.actor, self.session, self.next) {
            return;
        }
        let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(self.now_us);
        for (actor, session, aggression) in self.locks {
            let lock = crate::gameplay_transport::pvp_live::lock_mut(states, actor, session);
            lock.on_pvp_hit(now);
            if aggression {
                lock.on_aggression(now)
            }
        }
        if let Some(death) = self.death {
            states.deaths.push((self.actor, self.session, death));
        }
    }
}
