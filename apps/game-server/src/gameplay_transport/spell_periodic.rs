//! Actual Channel periodic turns join fenced tile reads to the existing player
//! condition store. A historical field creator is never a current PvP grant.
use super::super::ComposedFreshAdmission;
use super::{ChannelSpellStates, qualified_protection_zone};
use crate::ability::condition::{DotElement, TickFacts};
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceItemField, ReferenceItemType};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_item_transaction::StandingTileRead;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, RuntimeScopeRefV1,
};
use crate::spell::world_items_execution::SpellGroundTarget;
use oteryn_protocol_oteryn::actor_spell::ActorVitals;

fn standing_element(
    read: &StandingTileRead,
    source: &NativeGameplayState,
) -> Option<Option<DotElement>> {
    for item in read.items() {
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
        use crate::ability::condition::{ConditionSourceKind, TickKind};
        let before = self.get(runtime, actor, session)?;
        let preview = crate::spell::actor_conditions::stage_source_owner_cycle(
            before,
            now,
            Some(facts),
            None,
        )
        .ok()??;
        if !preview.combat_ticks.iter().any(|t| {
            matches!(t.kind, TickKind::Damage { .. })
                && t.provenance.source_kind == ConditionSourceKind::Creature
        }) {
            return None;
        }
        if preview.combat_ticks.iter().any(|t| {
            matches!(t.kind, TickKind::Damage { .. })
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
    pub(crate) fn tick_with_standing_read(
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
            let Some(element) = standing_element(read, source) else {
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
    pub(in crate::gameplay_transport) async fn tick_spell_periodic(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        let now = self.owner_now();
        let mut runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
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
        let room = self.qualified_room?;
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
            .read_standing_player_tile(self.character, self.holder, fence, content_digest, target)
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
        states.tick_with_standing_read(
            &mut runtime,
            self.movement_cells,
            room,
            source,
            actor,
            session,
            now,
            &read,
        )
    }
}
