//! Crystal 201189f8 data-global/scripts/actions/other/mechanical_fishing.lua:106-108,218-237.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues,
};
use crate::map::overlay::TilePos;
use oteryn_gameplay_items::fishing::{
    MECHANICAL_FISHING_SPENT_WATER, MechanicalSewerAttempt, mechanical_sewer_attempt,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn project_sewer_fishing_failure(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        native: &NativeGameplayState,
        (key, revision, _, position): (MapEntryKey, u64, bool, TilePos),
        occurrence: Option<[u8; 16]>,
    ) {
        let MapEntryKey::Base(key) = key else {
            return;
        };
        let Some(map) = self.bundle_map() else {
            return;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let session = command.game_session_id;
        let Ok(at) = runtime.read_actor_position(actor) else {
            return;
        };
        let at = at.position();
        if i16::from(position.floor) != -at.floor
            || at.x.abs_diff(i32::from(position.x)) > 1
            || at.y.abs_diff(i32::from(position.y)) > 1
            || map.facts.object_revision(key) != revision
            || pinned_index(&runtime, self.active_generation).is_none()
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|f| f.control_loss.is_none())
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
            || !self.states.player_trades.physical(session, None)
        {
            return;
        }
        let Some(water) = view::compose_tile(&map.overlay, &map.facts.pinned(), position)
            .ok()
            .flatten()
            .and_then(|tile| {
                tile.entries
                    .into_iter()
                    .find(|entry| entry.source.key() == MapEntryKey::Base(key))
            })
        else {
            return;
        };
        let Some(occurrence) = occurrence else {
            if MECHANICAL_FISHING_SPENT_WATER.contains(&water.facts.appearance_id) {
                map.facts.emit_movement_effect(position, 2);
            }
            return;
        };
        let root = GameplayDecisionRoot::from_bytes(native.source_digest());
        let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
        let Ok(draw) = deterministic_decision_u64(&root, occurrence, "fishing.sewer.shock", 0)
        else {
            return;
        };
        let Ok(roll) = u32::try_from(crate::spell::uniform_draw(draw, 1, 100)) else {
            return;
        };
        let attempt = mechanical_sewer_attempt(water.facts.appearance_id, true, true, false, roll);
        if !matches!(
            attempt,
            MechanicalSewerAttempt::Miss | MechanicalSewerAttempt::Shocked
        ) {
            return;
        }
        map.facts.emit_movement_effect(position, 2);
        if attempt != MechanicalSewerAttempt::Shocked {
            return;
        }
        let now =
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(self.owner_now().get());
        if states
            .apply_source_health_loss(&runtime, actor, session, 35, now)
            .is_none()
        {
            return;
        }
        map.facts.emit_movement_effect(position, 12);
        self.states
            .chat
            .publish_quest_speech(&runtime, actor, session, "Yikes! That DID hurt!");
        let Some(appearance) = native
            .spell_appearances()
            .and_then(|index| index.for_look_type(33))
        else {
            return;
        };
        let Some(definition) = ConditionDefinition::new(
            "source:mechanical-fishing:shock",
            1,
            ConditionValues::Outfit {
                look_type: 33,
                duration_ms: 4_000,
            },
        )
        .and_then(|definition| definition.with_appearance(appearance.selection())) else {
            return;
        };
        let Some(current) = states.get(&runtime, actor, session) else {
            return;
        };
        let Ok(base_speed) = u16::try_from(current.owned_base_speed()) else {
            return;
        };
        let facts = ApplicationFacts {
            now: now.get(),
            base_speed,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence,
        };
        let expected = current.owned_conditions();
        let mut conditions = expected.clone();
        let applied = conditions.apply(
            &definition,
            Some(crate::spell::combat_execution::actor_atom(actor)),
            ConditionSourceKind::SelfUse,
            &[],
            &facts,
        );
        if applied.is_ok() && conditions != *expected {
            let mut next = current.clone();
            if next.apply_batch_conditions(expected, &conditions).is_ok()
                && next.advance_batch_revision().is_ok()
                && states.commit(&runtime, actor, session, next)
            {
                states.owner_wake.notify_one();
            }
        }
    }
}
