//! Actual Channel periodic turns join fenced tile reads to the existing player
//! condition store. A historical field creator is never a current PvP grant.
use super::super::ComposedFreshAdmission;
use super::{ChannelSpellStates, qualified_protection_zone};
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceItemField, ReferenceItemType};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_item_transaction::StandingTileRead;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, RuntimeScopeRefV1,
};
use oteryn_gameplay_domain::domain::condition::DotElement;
use oteryn_gameplay_durability::durability::native_world_items::SpellGroundTarget;
use oteryn_gameplay_foundation::foundation::condition::TickFacts;
use oteryn_protocol_oteryn::actor_spell::ActorVitals;

fn standing_element(
    read: &StandingTileRead,
    source: &NativeGameplayState,
) -> Option<Option<DotElement>> {
    dynamic_element(read.items(), source)
}

fn dynamic_element(
    items: &[crate::durability::native_world_items::DurableTileItem],
    source: &NativeGameplayState,
) -> Option<Option<DotElement>> {
    for item in items {
        let policy = source.item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )?;
        let record = policy.record();
        let ReferenceItemField::Known(classification) = &record.semantics.classification else {
            return None;
        };
        match classification.item_type {
            ReferenceItemField::Known(ReferenceItemType::MagicField) => {
                return record.attributes.field_condition.as_ref()?.element().ok();
            }
            ReferenceItemField::Known(_) | ReferenceItemField::NotApplicable => {}
            ReferenceItemField::Unknown | ReferenceItemField::Conflict => return None,
        }
    }
    Some(None)
}
/// What one static top Item says about a standing field (COND-1c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticItemFact {
    /// The Item has no qualified policy: its field semantics are missing, so Unknown.
    NoPolicy,
    /// A known Item that is not a field.
    Plain,
    /// A known field Item; `None` when its element is not resolvable.
    Field(Option<Option<DotElement>>),
    /// A classification that is not known.
    Unknown,
}
fn fold_static(facts: impl IntoIterator<Item = StaticItemFact>) -> Option<Option<DotElement>> {
    // The first field wins, but every later Item is still checked: one without qualified
    // semantics could be the matching field, so it makes the whole tile Unknown.
    let mut field = Some(None);
    let mut seen_field = false;
    for fact in facts {
        match fact {
            StaticItemFact::NoPolicy | StaticItemFact::Unknown => return None,
            StaticItemFact::Plain => {}
            StaticItemFact::Field(element) => {
                if !seen_field {
                    seen_field = true;
                    field = element;
                }
            }
        }
    }
    field
}
/// The field a bundle tile carries as a static top Item: the bundle's fields are not
/// `game_item_ground_locations` rows. Missing metadata is Unknown, so no damage is consumed.
fn static_element(
    tile: &crate::content::QualifiedSpellTile,
    source: &NativeGameplayState,
) -> Option<Option<DotElement>> {
    fold_static(tile.top_source_ids().iter().map(|id| {
        let Some(policy) = source.item_policy_for_source_id(*id) else {
            return StaticItemFact::NoPolicy;
        };
        let record = policy.record();
        let ReferenceItemField::Known(classification) = &record.semantics.classification else {
            return StaticItemFact::Unknown;
        };
        match classification.item_type {
            ReferenceItemField::Known(ReferenceItemType::MagicField) => StaticItemFact::Field(
                record
                    .attributes
                    .field_condition
                    .as_ref()
                    .and_then(|f| f.element().ok()),
            ),
            ReferenceItemField::Known(_) | ReferenceItemField::NotApplicable => {
                StaticItemFact::Plain
            }
            ReferenceItemField::Unknown | ReferenceItemField::Conflict => StaticItemFact::Unknown,
        }
    }))
}
/// Ground Items and static bundle Items are two sources of the one standing fact: a known field
/// wins, Unknown on either side stays Unknown (no damage consumed).
fn combine_elements(
    dynamic: Option<Option<DotElement>>,
    fixed: Option<Option<DotElement>>,
) -> Option<Option<DotElement>> {
    match (dynamic?, fixed?) {
        (Some(element), _) | (None, Some(element)) => Some(Some(element)),
        (None, None) => Some(None),
    }
}
impl ChannelSpellStates {
    /// A preview is read-only. Canonical Creature-origin damage is consumed by
    /// the existing HP/conditions/death owner, including after its creator dies.
    /// Player/unknown provenance continues through the independent PvP path.
    pub(super) fn tick_current_creature_field_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: oteryn_simulation_determinism::SemanticTimeMicros,
        facts: TickFacts,
        native: Option<&NativeGameplayState>,
    ) -> Option<Option<(u64, ActorVitals)>> {
        use oteryn_gameplay_foundation::foundation::condition::{ConditionSourceKind, TickKind};
        let before = self.get(runtime, actor, session)?;
        let preview = crate::spell::actor_conditions::stage_source_owner_cycle(
            before,
            now,
            Some(facts),
            None,
        )
        .ok()??;
        // CONDITIONS-0 §3.5: an agony hit the cycle could not deal (lethal) is a creature hit
        // for this death-capable owner as well.
        let hit = |kind: TickKind| matches!(kind, TickKind::Damage { .. } | TickKind::Agony { .. });
        if !preview
            .combat_ticks
            .iter()
            .any(|t| hit(t.kind) && t.provenance.source_kind == ConditionSourceKind::Creature)
        {
            return None;
        }
        if preview.combat_ticks.iter().any(|t| {
            hit(t.kind)
                && (t.provenance.source_kind != ConditionSourceKind::Creature
                    || !t
                        .provenance
                        .source
                        .as_deref()
                        .is_some_and(|s| s.starts_with("creature:")))
        }) {
            return Some(None);
        }
        // Immutable Creature field provenance remains historical after caster
        // retirement, but never survives a foreign/current content generation.
        for tick in &preview.combat_ticks {
            if let Some(key) = tick
                .provenance
                .source
                .as_deref()
                .filter(|key| key.starts_with("creature:field:"))
            {
                let Some(origin)=crate::durability::spell_items_abi::DurableCreatureFieldOrigin::from_condition_source(key) else{return Some(None);};
                if !native.is_some_and(|content| origin.matches_current_content(runtime, content)) {
                    return Some(None);
                }
            }
        }
        // This ID scopes the owner pass only; actual DOT occurrence/provenance
        // is issued by the retained canonical ConditionStore, not this value.
        let mut bytes = actor.placement_identity();
        for (i, b) in now.get().to_be_bytes().iter().enumerate() {
            bytes[i] ^= *b;
        }
        let receipt = self.tick_source_player_conditions(
            runtime,
            actor,
            session,
            oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes),
            facts,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
        );
        Some(receipt.and_then(|r| {
            self.get(runtime, actor, session)
                .map(|s| (r.revision, s.vitals()))
        }))
    }

    /// The sealed read came from the real current DB owner. The caller keeps
    /// runtime/state locked; current physical position/pin are compared again.
    #[allow(
        clippy::too_many_arguments,
        reason = "the owner turn binds every independently resolved fact explicitly"
    )]
    pub fn tick_with_standing_read(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
        room: &crate::content::QualifiedNativeEntryRoom,
        source: &NativeGameplayState,
        actor: ExactActorRef,
        session: GameSessionId,
        now: oteryn_simulation_determinism::SemanticTimeMicros,
        read: &StandingTileRead,
    ) -> Option<(u64, ActorVitals)> {
        if read.fence().game_session_id != session
            || read.fence().runtime_scope
                != RuntimeScopeRefV1::channel(
                    runtime.binding().world_id(),
                    runtime.binding().channel_id(),
                )
            || read.fence().scope_ownership_generation != runtime.binding().scope_generation()
            || read.content_digest() != runtime.content_pin().server_artifact_digest()
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|f| f.control_loss.is_none())
        {
            return None;
        }
        let snapshot = runtime.borrow_movement_position().read(actor).ok()?;
        if snapshot.context() != runtime.pinned_movement_context()
            || SpellGroundTarget::from_native_owner(room, runtime, snapshot.position())
                .ok()?
                .ne(read.target())
        {
            return None;
        }
        let in_protection_zone = qualified_protection_zone(runtime, cells, actor)?;
        let standing_on_field = if in_protection_zone {
            None
        } else {
            let static_tile = cells
                .spell_tiles()
                .lookup(
                    cells.scope(),
                    crate::content::LogicalCell {
                        x: snapshot.position().x,
                        y: snapshot.position().y,
                        z: i32::from(snapshot.position().floor),
                    },
                )
                .ok();
            let dynamic = standing_element(read, source);
            let fixed = match static_tile {
                Some(tile) => static_element(tile, source),
                None => Some(None),
            };
            let element = combine_elements(dynamic, fixed);
            let Some(element) = element else {
                // Missing field semantics cannot consume damage. Non-damage
                // ticks depend on the independently qualified PZ, however.
                return self.tick_in_environment(runtime, cells, actor, session, now);
            };
            element
        };
        if let Some(source_result) = self.tick_current_creature_field_damage(
            runtime,
            actor,
            session,
            now,
            TickFacts {
                in_protection_zone,
                standing_on_field,
            },
            Some(source),
        ) {
            return source_result;
        }
        let stamp = runtime.issue_owner_work().ok()?;
        let before = self.get(runtime, actor, session)?;
        let prepared_result = crate::spell::periodic_execution::stage_player_periodic_turn(
            before,
            actor,
            session,
            stamp,
            now,
            TickFacts {
                in_protection_zone,
                standing_on_field,
            },
            // Accepted gameplay presently has no player PvP legality owner.
            // Field source receipt/session bytes are historical evidence only.
            |_| false,
        );
        let prepared = match prepared_result {
            Ok(Some(prepared)) => prepared,
            Ok(None) => return None,
            Err(_) => {
                // An unknown PvP grant leaves DOT pending; independently
                // qualified expiry, Serene and regeneration may still commit.
                let cycle =
                    crate::spell::actor_conditions::stage_source_owner_cycle_without_damage(
                        before,
                        now,
                        TickFacts {
                            in_protection_zone,
                            standing_on_field,
                        },
                        None,
                    )
                    .ok()??;
                if !cycle.combat_ticks.is_empty() {
                    return None;
                }
                let state = self.get_mut(runtime, actor, session)?;
                *state = cycle.next;
                return cycle
                    .publish_vitals
                    .then(|| (state.revision(), state.vitals()));
            }
        };
        if !runtime.owner_fence().ok()?.accepts_stamp(prepared.stamp())
            || prepared.actor() != actor
            || prepared.session() != session
            || self.get(runtime, actor, session)? != prepared.predecessor()
        {
            return None;
        }
        let (next, publish) = prepared.into_successor();
        let state = self.get_mut(runtime, actor, session)?;
        *state = next;
        publish.then(|| (state.revision(), state.vitals()))
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Consume the existing creature store while current World source levels and
    /// the actual standing tile remain locked. An unavailable source leaves it pending.
    pub(in crate::gameplay_transport) async fn tick_creature_periodic(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        target: ExactActorRef,
        now: oteryn_simulation_determinism::SemanticTimeMicros,
    ) -> Option<crate::foundation::CreaturePeriodicReceipt> {
        use crate::durability::{DurabilityError, spell_item_transaction as item_tx};
        use crate::gameplay_transport::hazard::combat;
        let snapshot = runtime.companion_snapshot(target).ok()?;
        if snapshot.state.conditions.instances().is_empty() {
            return None;
        }
        let room = self.rooms.gate_monster_ai()?;
        let native = self.active_generation?.native_gameplay()?;
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, runtime, states, permit);
        pass.run_with_context(&mut context, move |holder, deadline, ctx| {
            Box::pin(async move {
                let (owner, runtime, states, permit) = ctx;
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                let binding = runtime.binding();
                let scope = item_tx::assert_spell_item_scope_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
                    binding.scope_generation().get(),
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let position = runtime
                    .read_actor_position(target)
                    .map_err(|_| DurabilityError::Unavailable)?
                    .position();
                let tile = room
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
                    .map_err(|_| DurabilityError::Unavailable)?;
                let address = SpellGroundTarget::for_native_tile_read(room, runtime, position)
                    .map_err(|_| DurabilityError::Unavailable)?;
                let items =
                    item_tx::read_creature_scope_tile_in_transaction(&mut tx, &scope, &address)
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                let standing_on_field = combine_elements(
                    dynamic_element(&items.items, native),
                    static_element(tile, native),
                )
                .ok_or(DurabilityError::Unavailable)?;
                let facts = TickFacts {
                    in_protection_zone: tile.flags().protection_zone,
                    standing_on_field,
                };
                let mut conditions = snapshot.state.conditions.clone();
                let ticks = conditions.take_due(now.get(), facts);
                let mut levels = Vec::new();
                if combat::eligible(runtime, target) {
                    for tick in &ticks {
                        let Some(source) = tick.provenance.source else {
                            continue;
                        };
                        if levels.iter().any(|(actor, _, _)| *actor == source) {
                            continue;
                        }
                        let Some((_, session, _)) =
                            states.actors.iter().find(|(actor, _, _)| *actor == source)
                        else {
                            continue;
                        };
                        // Absent or retired sources retain the condition's ordinary damage;
                        // only a current player may supply a hazard dodge.
                        if !runtime
                            .player_control_facts(source, *session)
                            .is_ok_and(|facts| facts.control_loss.is_none())
                        {
                            continue;
                        }
                        let level = if combat::in_area(runtime, source) {
                            combat::effective_level(
                                &mut tx, owner.root, &scope, runtime, source, *session,
                            )
                            .await
                            .map_err(|_| DurabilityError::Unavailable)?
                        } else {
                            0
                        };
                        levels.push((source, *session, level));
                    }
                }
                let receipt = states
                    .tick_creature_conditions_with_hazard(
                        runtime,
                        target,
                        facts,
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
                        &levels,
                    )
                    .map_err(|_| DurabilityError::Unavailable)?;
                tx.rollback().await?;
                Ok(receipt)
            })
        })
        .await
        .ok()
    }

    pub(in crate::gameplay_transport) async fn tick_spell_periodic(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        let now = self.owner_now();
        // Lock order (§1.2): the lane before any Channel guard.
        let permit = self.spell_lane_permit().await?;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if states.has_pending_spell_commit(actor, session) {
            return None;
        }
        states
            .get_mut(&runtime, actor, session)?
            .expire_owner_premium(now.get())
            .ok()?;
        let state = states.get(&runtime, actor, session)?;
        if !crate::spell::actor_conditions::has_periodic_damage(state) {
            return states.tick_in_environment(
                &mut runtime,
                self.movement_cells,
                actor,
                session,
                now,
            );
        }
        // COND-1c: a bundle World has no fixture qualified room; its periodic player damage
        // reads the same bundle view as combat (BUNDLE-COMBAT-ENABLE-1).
        let room = self.rooms.gate_periodic_damage()?;
        let source = self.active_generation?.native_gameplay()?;
        let position = runtime.borrow_movement_position().read(actor).ok()?;
        let target =
            SpellGroundTarget::from_native_owner(room, &runtime, position.position()).ok()?;
        let content_digest = runtime.content_pin().server_artifact_digest();
        let current = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
            .ok()?
            .0;
        if current.session_state() != GameSessionState::Active {
            return None;
        }
        let fence = CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(
                *current.commit().character_id().as_bytes(),
            )
            .ok()?,
            game_session_id: session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
        };
        let read = match self
            .root
            .read_standing_player_tile(
                &permit,
                self.character,
                self.holder,
                fence,
                content_digest,
                target,
            )
            .await
        {
            Ok(read) => read,
            Err(_) => {
                return states.tick_in_environment(
                    &mut runtime,
                    self.movement_cells,
                    actor,
                    session,
                    now,
                );
            }
        };
        // The live owner locks have stayed held; nevertheless compare the full
        // position/context and every generation again before consuming ticks.
        if runtime.borrow_movement_position().read(actor).ok()? != position {
            return None;
        }
        self.with_hazard_level(
            &permit,
            &mut runtime,
            &mut states,
            actor,
            session,
            |runtime, states, _| {
                states.tick_with_standing_read(
                    runtime,
                    self.movement_cells,
                    room,
                    source,
                    actor,
                    session,
                    now,
                    &read,
                )
            },
        )
        .await
        .flatten()
    }
}

#[cfg(test)]
mod standing_fact_tests {
    use super::*;
    #[test]
    fn static_bundle_field_is_not_lost_and_unknown_stays_unknown() {
        let fire = Some(Some(DotElement::Fire));
        assert_eq!(combine_elements(Some(None), fire), fire);
        assert_eq!(combine_elements(fire, Some(None)), fire);
        assert_eq!(combine_elements(Some(None), Some(None)), Some(None));
        assert_eq!(combine_elements(None, fire), None);
        assert_eq!(combine_elements(fire, None), None);
    }
    #[test]
    fn an_unprofiled_static_item_is_unknown_not_a_known_non_field() {
        use StaticItemFact::*;
        assert_eq!(fold_static([Plain, NoPolicy]), None);
        assert_eq!(
            fold_static([NoPolicy, Field(Some(Some(DotElement::Fire)))]),
            None
        );
        assert_eq!(fold_static([Plain, Unknown]), None);
        assert_eq!(fold_static([Plain, Plain]), Some(None));
        assert_eq!(
            fold_static([Plain, Field(Some(Some(DotElement::Fire)))]),
            Some(Some(DotElement::Fire))
        );
        assert_eq!(fold_static([Field(None)]), None);
        let fire = Field(Some(Some(DotElement::Fire)));
        assert_eq!(fold_static([fire, NoPolicy]), None);
        assert_eq!(fold_static([fire, Unknown]), None);
        assert_eq!(fold_static([fire, Plain]), Some(Some(DotElement::Fire)));
        assert_eq!(
            fold_static([fire, Field(Some(Some(DotElement::Energy)))]),
            Some(Some(DotElement::Fire))
        );
    }
}
