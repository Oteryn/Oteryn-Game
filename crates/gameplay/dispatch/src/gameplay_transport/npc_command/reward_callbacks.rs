//! Content selects a bounded exchange; the shared quest transaction owns every effect.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step as DialogueStep, Topic};
use super::*;
use oteryn_gameplay_quests::quest::reward_callbacks;
use oteryn_gameplay_quests::quest::world_callbacks::Storage;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Offers {
    schema: u32,
    offers: Vec<Offer>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Offer {
    npc: String,
    keywords: Vec<String>,
    topic: u16,
    next_topic: u16,
    storage: Vec<Storage>,
    #[serde(default)]
    requires: Vec<reward_callbacks::TrackRequirement>,
    reply: Vec<String>,
}

pub(super) struct Step {
    dialogue: DialogueStep,
    guard: Option<(quest_dialogue::Context, reward_callbacks::RewardCallback)>,
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let callbacks = reward_callbacks::load().ok()?;
    let offers: Offers = serde_json::from_str(include_str!(
        "../../../../../../content/quests/shared-reward-offers.json"
    ))
    .ok()?;
    if offers.schema != 1 || offers.offers.iter().any(|o| o.requires.len() > 8) {
        return None;
    }
    let catalogue = admission.quest_catalogue.as_deref()?;
    let mut guard = None;
    let dialogue = quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _, tracks| {
            let text = intent.text.trim().to_ascii_lowercase();
            let eligible = |storage: &[Storage]| {
                storage.iter().all(|s| {
                    catalogue.track(&s.track).is_some_and(|track| {
                        (s.minimum..=s.maximum)
                            .contains(&tracks.get(&s.track).copied().unwrap_or(track.initial))
                    })
                })
            };
            let mut selected = callbacks.iter().filter(|c| {
                c.npc == context.2
                    && c.topic == topic.number
                    && c.keywords.contains(&text)
                    && eligible(&c.storage)
                    && c.requires.iter().all(|r| r.holds(catalogue, tracks))
            });
            if let Some(callback) = selected.next() {
                if selected.next().is_some() {
                    return None;
                }
                if callback
                    .clock_effects()
                    .any(|clock| clock.elapsed_seconds.is_some())
                    && !callback.cooldown_reply.is_empty()
                {
                    guard = Some((context.clone(), callback.clone()));
                }
                return Some(Choice {
                    transition: Some(callback.key.clone()),
                    next_topic: Some(Topic::from(callback.next_topic)),
                    reply: callback.reply.clone(),
                });
            }
            let offer = offers.offers.iter().find(|c| {
                c.npc == context.2
                    && c.topic == topic.number
                    && c.keywords.contains(&text)
                    && eligible(&c.storage)
                    && c.requires.iter().all(|r| r.holds(catalogue, tracks))
            })?;
            Some(Choice {
                transition: None,
                next_topic: Some(Topic::from(offer.next_topic)),
                reply: offer.reply.clone(),
            })
        },
    )?;
    Some(Step { dialogue, guard })
}

/// Clock advice is a fresh read; the value writer independently rechecks every grant.
pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    use crate::durability::quest_state::RewardReadiness;
    let Some((context, callback)) = step.guard else {
        return quest_dialogue::apply(admission, actor, command, intent, step.dialogue).await;
    };
    let Some(catalogue) = admission.quest_catalogue.as_ref() else {
        return NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    };
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    };
    let observed = admission
        .root
        .read_quest_reward_clock_readiness(
            admission.character,
            fence,
            std::sync::Arc::clone(catalogue),
            callback.key.clone(),
        )
        .await;
    let Ok(observed) = observed else {
        return NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    };
    match observed.readiness {
        RewardReadiness::Ready => {
            quest_dialogue::apply(admission, actor, command, intent, step.dialogue).await
        }
        RewardReadiness::Cooldown => {
            let runtime = admission.runtime.lock().await;
            let reply = quest_dialogue::prepare(
                admission,
                actor,
                command,
                &intent,
                &runtime,
                |current, topic, _, _| {
                    if current != &context || topic.number != callback.topic {
                        return None;
                    }
                    Some(Choice {
                        transition: None,
                        next_topic: Some(Topic::from(callback.next_topic)),
                        reply: clock_reply(&callback.cooldown_reply, observed.remaining_seconds),
                    })
                },
            );
            drop(runtime);
            match reply {
                Some(reply) => {
                    quest_dialogue::apply(admission, actor, command, intent, reply).await
                }
                None => NpcTalkOutcome::only(NpcIntentDisposition::Rejected),
            }
        }
        _ => NpcTalkOutcome::only(NpcIntentDisposition::Rejected),
    }
}

fn clock_reply(lines: &[String], remaining: Option<i64>) -> Vec<String> {
    let remaining = remaining.unwrap_or(0).max(0);
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours} hour{}", if hours == 1 { "" } else { "s" }));
    }
    if minutes > 0 {
        parts.push(format!(
            "{minutes} minute{}",
            if minutes == 1 { "" } else { "s" }
        ));
    }
    let time = if parts.is_empty() {
        "less than a minute".into()
    } else {
        parts.join(" and ")
    };
    lines
        .iter()
        .map(|line| line.replace("|REMAINING|", &time))
        .collect()
}
