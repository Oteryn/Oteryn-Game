//! Crystal action_mead.lua:1-4,18-21: failed sips briefly display Item111.
use super::*;
use oteryn_gameplay_domain::domain::condition::ConditionValues;
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};
use oteryn_gameplay_quests::quest::{QuestTrackChange, barbarian_mead};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn project_mead_blackout(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        command: CommandRef,
        changes: &[QuestTrackChange],
        map: &BundleMap,
    ) -> bool {
        if !changes
            .iter()
            .any(|change| change.track == barbarian_mead::DRAW && change.after == 1)
            || !runtime
                .player_control_facts(actor, command.game_session_id())
                .is_ok_and(|facts| facts.control_loss.is_none())
            || states.is_dead(actor)
            || !states.spell_idle(runtime, actor, command.game_session_id())
        {
            return false;
        }
        let Some(index) =
            super::super::item_ref_admission::pinned_index(runtime, self.active_generation)
        else {
            return false;
        };
        if index
            .definition_ref("Item", "oteryn:item.tibia.i111", "definition-r1")
            .is_none()
        {
            return false;
        }
        let Some(position) = runtime
            .read_actor_position(actor)
            .ok()
            .and_then(|snapshot| native(snapshot.position()))
        else {
            return false;
        };
        let Some(definition) = ConditionDefinition::new(
            "quest.barbarian.mead.blackout",
            1,
            ConditionValues::ItemOutfit { duration_ms: 1000 },
        )
        .and_then(|definition| {
            definition.with_item_appearance(
                "oteryn:item.tibia.i111",
                "definition-r1",
                runtime.content_pin().server_artifact_digest(),
            )
        }) else {
            return false;
        };
        let now = self.owner_now().get();
        let Ok(protected) =
            runtime.current_player_reentry_protection(actor, command.game_session_id(), now)
        else {
            return false;
        };
        let Some(current) = states.get(runtime, actor, command.game_session_id()) else {
            return false;
        };
        let Ok(base_speed) = u16::try_from(current.owned_base_speed()) else {
            return false;
        };
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let hash = Sha256::new()
            .chain_update(b"oteryn:m47-blackout:v1")
            .chain_update(actor.placement_identity())
            .chain_update(command.game_session_id().as_bytes())
            .chain_update(command.command_id().get().to_le_bytes())
            .finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&hash[..16]);
        let facts = ApplicationFacts {
            now,
            base_speed,
            mana_shield_capacity: 0,
            target_reentry_protected: protected,
            source_reentry_protected: protected,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes(occurrence),
        };
        let expected = current.owned_conditions();
        let mut conditions = expected.clone();
        if conditions
            .apply(
                &definition,
                Some(crate::spell::combat_execution::actor_atom(actor)),
                ConditionSourceKind::SelfUse,
                &[],
                &facts,
            )
            .is_ok()
            && conditions != *expected
        {
            let mut next = current.clone();
            if next.apply_batch_conditions(expected, &conditions).is_ok()
                && next.advance_batch_revision().is_ok()
            {
                states.commit(runtime, actor, command.game_session_id(), next);
            }
        }
        // Source addCondition is best effort; the committed failed sip still emits HITBYPOISON.
        map.facts.emit_movement_effect(position, 17)
    }
}
