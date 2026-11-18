//! Armenius's source conversation from Crystal 201189f8.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;

use oteryn_gameplay_quests::quest::armenius::{
    CONFRONTATION_ROUTE, FINAL_TOPIC, MISSION, NPC, Outcome, RATS_ROUTE, RESOLVE, RESOLVED_MISSION,
    outcome,
};

pub(super) enum Step {
    Dialogue(Box<quest_dialogue::Step>),
    Teleport(catalogue::Step),
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    match step {
        Step::Dialogue(step) => {
            quest_dialogue::apply(admission, actor, command, intent, *step).await
        }
        Step::Teleport(step) => catalogue::apply(admission, actor, command, intent, step).await,
    }
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let mut teleport = None;
    let dialogue = quest_dialogue::prepare(
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
                match final_outcome(runtime, command.game_session_id, command.command_id)? {
                    Outcome::Rats { destination } => {
                        teleport = Some((context.clone(), RATS_ROUTE, destination));
                        return None;
                    }
                    Outcome::Confrontation { destination, .. } => {
                        teleport = Some((context.clone(), CONFRONTATION_ROUTE, destination));
                        return None;
                    }
                    Outcome::Dismissed => {}
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
    );
    if let Some((context, route, destination)) = teleport {
        return catalogue::armenius_teleport(
            admission,
            actor,
            command,
            intent,
            context,
            route,
            destination,
        )
        .map(Step::Teleport);
    }
    dialogue.map(|step| Step::Dialogue(Box::new(step)))
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
