//! Iskan runs through the shared conversation and quest revision owners.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::iskan as source;

pub(super) async fn warlord_greeting(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
) -> bool {
    let greeting = intent.text.trim().eq_ignore_ascii_case("hi")
        || intent.text.trim().eq_ignore_ascii_case("hello");
    if !greeting {
        return false;
    }
    let is_iskan = {
        let runtime = admission.runtime.lock().await;
        service_context::live_npc(&runtime, actor, command.game_session_id, intent.npc_actor)
            .is_some_and(|npc| npc.key == source::NPC)
    };
    let Some(account) = admission
        .session_account(command.game_session_id)
        .filter(|_| is_iskan)
    else {
        return false;
    };
    admission
        .root
        .read_account_achievements(admission.character, account, admission.achievements.len())
        .await
        .is_ok_and(|facts| {
            facts.iter().any(|fact| {
                fact.achievement_key == crate::durability::arena_match::donor::ACHIEVEMENTS[2]
            })
        })
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    warlord: bool,
) -> Option<Step> {
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, fresh, tracks| {
            if context.2 != source::NPC {
                return None;
            }
            let stage = |track: &str| tracks.get(track).copied().unwrap_or(-1);
            let known = stage(source::BARBARIAN) >= 8;
            let killed_husky = stage(source::HUSKY_KILL) >= 1;
            let text = intent.text.trim().to_ascii_lowercase();
            let (transition, next, reply) = match text.as_str() {
                "hi" | "hello" if fresh => (
                    None,
                    0,
                    if !known {
                        "You're not one of us. I have nothing to discuss with you."
                    } else if killed_husky {
                        "You have been killing our huskies. Give me 500 gold and I'll forget it."
                    } else if warlord {
                        "Greetings, |PLAYERNAME|, Warlord of Svargrond. What can I do for you?"
                    } else {
                        "Greetings. What can I do for you?"
                    },
                ),
                "mission" | "passage" if !known => (None, 0, "Sorry, not possible."),
                "mission" if stage(source::QUESTLINE) == 1 => (
                    None,
                    1,
                    "Sniffler has run off. Will you look for him northwest of town?",
                ),
                "mission" if stage(source::QUESTLINE) == 2 => (
                    Some(source::REPORT),
                    0,
                    "You're a friend to me and the dogs. Ask me for a passage to Nibelor any time.",
                ),
                "mission" => (None, 0, "I have no mission for you."),
                "yes" if topic.number == 1 && known && !killed_husky => (
                    Some(source::ACCEPT),
                    0,
                    "Call him by name and tell him you have meat. Return after he has eaten.",
                ),
                _ => return None,
            };
            Some(Choice {
                transition: transition.map(str::to_owned),
                next_topic: Some(Topic::from(next)),
                reply: vec![reply.to_owned()],
            })
        },
    )
}
