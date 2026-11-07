//! Content selects a bounded exchange; the shared quest transaction owns every effect.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
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
    #[serde(default)]
    any_topic: bool,
    next_topic: u16,
    storage: Vec<Storage>,
    #[serde(default)]
    requires: Vec<reward_callbacks::TrackRequirement>,
    reply: Vec<String>,
    #[serde(default)]
    close_after: bool,
    #[serde(default)]
    minimum_level: Option<u32>,
    #[serde(default)]
    failure_reply: Vec<String>,
}

pub(super) fn needs_facts(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> bool {
    let Some(context) = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    ) else {
        return false;
    };
    let text = intent.text.trim().to_ascii_lowercase();
    reward_callbacks::load().is_ok_and(|rows| {
        rows.iter().any(|c| {
            c.npc == context.2
                && c.keywords.contains(&text)
                && (!c.inventory.is_empty() || c.minimum_level.is_some())
        })
    }) || serde_json::from_str::<Offers>(include_str!(
        "../../../../../../content/quests/shared-reward-offers.json"
    ))
    .is_ok_and(|rows| {
        rows.schema == 1
            && rows.offers.iter().any(|offer| {
                offer.npc == context.2
                    && offer.keywords.contains(&text)
                    && offer.minimum_level.is_some()
            })
    })
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    inventory: Option<&super::reward_facts::RewardFacts>,
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
    let mut conclusion = (false, Vec::new());
    let step = quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _, tracks| {
            let text = intent.text.trim().to_ascii_lowercase();
            let meets_level = |minimum: Option<u32>| {
                minimum.is_none_or(|minimum| inventory.is_some_and(|facts| facts.level >= minimum))
            };
            let eligible = |storage: &[Storage]| {
                storage.iter().all(|s| {
                    tracks
                        .get(&s.track)
                        .copied()
                        .or_else(|| catalogue.track(&s.track).map(|track| track.initial))
                        .is_some_and(|value| (s.minimum..=s.maximum).contains(&value))
                })
            };
            let mut selected = callbacks.iter().filter(|c| {
                c.carried_item.is_none()
                    && c.npc == context.2
                    && c.topic == topic.number
                    && c.keywords.contains(&text)
                    && eligible(&c.storage)
                    && c.state_ops
                        .iter()
                        .all(|op| op.advisory_holds(catalogue, tracks))
                    && c.participants.iter().all(|p| {
                        eligible(&p.storage)
                            && p.state_ops
                                .iter()
                                .all(|op| op.advisory_holds(catalogue, tracks))
                    })
                    && c.inventory
                        .iter()
                        .all(|r| inventory.is_some_and(|facts| r.holds(&facts.quantities)))
                    && c.requires.iter().all(|r| r.holds(catalogue, tracks))
            });
            if let Some(callback) = selected.next() {
                if selected.next().is_some() {
                    return None;
                }
                if !meets_level(callback.minimum_level) {
                    conclusion = (callback.close_after, Vec::new());
                    return (!callback.failure_reply.is_empty()).then(|| Choice {
                        transition: None,
                        next_topic: Some(Topic::from(callback.next_topic)),
                        reply: callback.failure_reply.clone(),
                    });
                }
                conclusion = (callback.close_after, callback.failure_reply.clone());
                return Some(Choice {
                    transition: Some(callback.key.clone()),
                    next_topic: Some(Topic::from(callback.next_topic)),
                    reply: callback.reply.clone(),
                });
            }
            if let Some(offer) = offers.offers.iter().find(|c| {
                c.npc == context.2
                    && (c.any_topic || c.topic == topic.number)
                    && c.keywords.contains(&text)
                    && eligible(&c.storage)
                    && c.requires.iter().all(|r| r.holds(catalogue, tracks))
            }) {
                conclusion = (offer.close_after, Vec::new());
                if !meets_level(offer.minimum_level) {
                    return (!offer.failure_reply.is_empty()).then(|| Choice {
                        transition: None,
                        next_topic: Some(Topic::from(0)),
                        reply: offer.failure_reply.clone(),
                    });
                }
                return Some(Choice {
                    transition: None,
                    next_topic: Some(Topic::from(offer.next_topic)),
                    reply: offer.reply.clone(),
                });
            }
            // A unique exchange owns its exact keyword even before eligibility.
            // Let the locked writer return the stage refusal, rather than falling
            // through to an unrelated legacy dialogue with the same text.
            let mut matching = callbacks.iter().filter(|c| {
                c.carried_item.is_none()
                    && c.npc == context.2
                    && c.topic == topic.number
                    && c.keywords.contains(&text)
            });
            let callback = matching.next()?;
            conclusion = (callback.close_after, callback.failure_reply.clone());
            if matching.next().is_some() {
                return None;
            }
            Some(Choice {
                transition: Some(callback.key.clone()),
                next_topic: Some(Topic::from(callback.next_topic)),
                reply: callback.reply.clone(),
            })
        },
    );
    step.map(|step| step.conclusion(conclusion.0, conclusion.1))
}

/// A recognized reward confirmation may not fall through to ordinary dialogue
/// when every native quest owner declined it. Greeting words retain their owner.
pub(super) fn unhandled_confirmation(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> bool {
    let text = intent.text.trim().to_ascii_lowercase();
    if matches!(text.as_str(), "hi" | "hello" | "bye" | "farewell") {
        return false;
    }
    let Some(context) = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    ) else {
        return false;
    };
    reward_callbacks::load().is_ok_and(|callbacks| {
        callbacks
            .iter()
            .any(|callback| callback.npc == context.2 && callback.keywords.contains(&text))
    })
}
