//! Armenius's source conversation from Crystal 201189f8.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;

use oteryn_gameplay_quests::quest::armenius::{
    FINAL_TOPIC, MISSION, NPC, Outcome, RESOLVE, RESOLVED_MISSION, outcome,
};

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<quest_dialogue::Step> {
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _, tracks| {
            let mission = tracks.get(MISSION).copied();
            let speaks_spell = intent.text.eq_ignore_ascii_case("alori mort");
            if context.2 != NPC
                || intent.text == "cookie"
                || intent.text == "yes"
                || !(mission == Some(RESOLVED_MISSION) || (mission == Some(1) && speaks_spell))
            {
                return None;
            }
            if topic.number == FINAL_TOPIC {
                if final_outcome(runtime, command.game_session_id, command.command_id)?
                    != Outcome::Dismissed
                {
                    return None;
                }
                return Some(quest_dialogue::Choice {
                    transition: Some(RESOLVE.into()),
                    next_topic: None,
                    reply: vec!["Enough. Leave me alone.".into()],
                });
            }
            let (next, reply) = match topic.number {
                0 => (2, "Speak plainly or leave me alone."),
                2 => (3, "Those words unsettle me. You should leave."),
                3 => (4, "What do you think those words mean?"),
                4 => (
                    FINAL_TOPIC,
                    "Go on, say it again. I could use another laugh.",
                ),
                _ => return None,
            };
            Some(quest_dialogue::Choice {
                transition: None,
                next_topic: Some(next.into()),
                reply: vec![reply.into()],
            })
        },
    )
}

pub(in crate::gameplay_transport) fn final_outcome(
    runtime: &ChannelRuntimeV1,
    session: GameSessionId,
    command_id: u64,
) -> Option<Outcome> {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };

    let occurrence = super::super::npc_trade_command::derived(
        b"oteryn:armenius-final-dialogue:v1",
        session,
        command_id,
    );
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    deterministic_decision_u64(
        &root,
        DecisionOccurrenceId::from_bytes(occurrence),
        "npc.armenius.outcome",
        0,
    )
    .ok()
    .map(outcome)
}
