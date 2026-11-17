//! Djinn War greeting effects. The existing player condition owner consumes Soulfire ticks.
use super::*;
use crate::domain::condition::{ConditionValues, DotElement};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionProvenance, ConditionSourceKind,
};
use oteryn_gameplay_npc::greeting::{BAA_LEAL, soulfire_on_greeting};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use sha2::{Digest, Sha256};

const SOULFIRE_SOURCE: &str = "environment:baa_leal_soulfire";
const SOULFIRE_DEFINITION: &str = "npc.baa_leal.soulfire";

pub(in crate::gameplay_transport) fn is_soulfire(provenance: &ConditionProvenance<String>) -> bool {
    provenance.source_kind == ConditionSourceKind::Field
        && provenance.source.as_deref() == Some(SOULFIRE_SOURCE)
        && provenance.definition_key == SOULFIRE_DEFINITION
        && provenance.lineage.is_none()
}

pub(super) async fn soulfire(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
) -> Option<()> {
    let session = command.game_session_id;
    let npc = service_context::live_npc(runtime, actor, session, intent.npc_actor)?;
    if npc.key != BAA_LEAL {
        return None;
    }
    let (total, per_tick, interval_ms) = soulfire_on_greeting(
        &npc.key,
        &intent.text,
        oteryn_gameplay_npc::npc_talk::quest_greeting_aliases(&npc.key),
    )?;
    let speaker = service_context::speaker_position(runtime, actor, session)?;
    if speaker.floor != npc.position.floor
        || speaker.x.abs_diff(npc.position.x) > TALK_RANGE_TILES
        || speaker.y.abs_diff(npc.position.y) > TALK_RANGE_TILES
        || runtime
            .player_control_facts(actor, session)
            .ok()?
            .control_loss
            .is_some()
    {
        return None;
    }
    let mut states = admission.states.spell_states.lock().await;
    if !states.spell_idle(runtime, actor, session) {
        return None;
    }
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let dialogue = sessions.get_mut(&session)?;
    if dialogue
        .soulfire_command
        .is_some_and(|previous| previous >= command.command_id)
    {
        return None;
    }
    let before = states.get(runtime, actor, session)?;
    let definition = ConditionDefinition::new(
        SOULFIRE_DEFINITION,
        1,
        ConditionValues::DamageOverTime {
            element: DotElement::Fire,
            total_min: total,
            total_max: total,
            per_tick,
            interval_ms,
            delayed: true,
        },
    )?;
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let digest = Sha256::new()
        .chain_update(SOULFIRE_DEFINITION)
        .chain_update(session.as_bytes())
        .chain_update(command.command_id.to_be_bytes())
        .finalize();
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    let facts = ApplicationFacts {
        now: admission.owner_now().get(),
        base_speed: 1,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes(occurrence),
    };
    let (next, _) = before.stage_environment_field_contact(
        SOULFIRE_SOURCE.to_owned(),
        &definition,
        &facts,
        DotElement::Fire,
        |_, _, _, _| None,
    )?;
    if next != *before && !states.commit(runtime, actor, session, next) {
        return None;
    }
    dialogue.soulfire_command = Some(command.command_id);
    Some(())
}

/// The donor tries the King's tile and its eight immediate neighbors.
pub(super) fn spawn_guards(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    intent: &NpcTalkIntent,
    guards: &[&str],
) {
    let Some(npc) = service_context::live_npc(runtime, actor, session, intent.npc_actor) else {
        return;
    };
    for key in guards {
        for (dx, dy) in [
            (0, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let position = crate::foundation::MovementLocalPosition {
                x: npc.position.x + dx,
                y: npc.position.y + dy,
                floor: npc.position.floor,
            };
            if super::super::primal_menace::spawn_creature(admission, runtime, key, position)
                .is_some()
            {
                break;
            }
        }
    }
}
