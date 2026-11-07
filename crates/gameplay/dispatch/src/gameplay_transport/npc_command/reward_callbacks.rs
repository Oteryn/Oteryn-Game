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
    next_topic: u16,
    storage: Vec<Storage>,
    #[serde(default)]
    requires: Vec<reward_callbacks::TrackRequirement>,
    reply: Vec<String>,
    #[serde(default)]
    close_after: bool,
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
    let mut conclusion = (false, Vec::new());
    let step = quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _, tracks| {
            let text = intent.text.trim().to_ascii_lowercase();
            let eligible = |storage: &[Storage]| {
                storage.iter().all(|s| {
                    catalogue.track(&s.track).is_some_and(|definition| {
                        (s.minimum..=s.maximum)
                            .contains(&tracks.get(&s.track).copied().unwrap_or(definition.initial))
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
                conclusion = (callback.close_after, callback.failure_reply.clone());
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
            conclusion = (offer.close_after, Vec::new());
            Some(Choice {
                transition: None,
                next_topic: Some(Topic::from(offer.next_topic)),
                reply: offer.reply.clone(),
            })
        },
    );
    step.map(|step| step.conclusion(conclusion.0, conclusion.1))
}
