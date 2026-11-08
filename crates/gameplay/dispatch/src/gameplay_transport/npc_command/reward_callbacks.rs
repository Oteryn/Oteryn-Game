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
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let callbacks = reward_callbacks::load().ok()?;
    prepare_with_callbacks(admission, actor, command, intent, runtime, &callbacks)
}

pub(super) fn prepare_with_callbacks(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    callbacks: &[reward_callbacks::RewardCallback],
) -> Option<Step> {
    let mut offers: Offers = serde_json::from_str(include_str!(
        "../../../../../../content/quests/shared-reward-offers.json"
    ))
    .ok()?;
    let authored: Offers = serde_json::from_str(include_str!(
        "../../../../../../content/quests/npc-offers/l6-stories.json"
    ))
    .ok()?;
    if authored.schema != 1 {
        return None;
    }
    offers.offers.extend(authored.offers);
    if offers.schema != 1
        || offers.offers.len() > 512
        || offers.offers.iter().any(|o| o.requires.len() > 8)
    {
        return None;
    }
    let catalogue = admission.quest_catalogue.as_deref()?;
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _, tracks| {
            if admission.has_pending_paid_confirmation(command, intent, context) {
                return None;
            }
            let text = intent.text.trim().to_ascii_lowercase();
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
                    && c.requires.iter().all(|r| r.holds(catalogue, tracks))
            });
            if let Some(callback) = selected.next() {
                if selected.next().is_some() {
                    return None;
                }
                return Some(Choice {
                    transition: Some(callback.key.clone()),
                    next_topic: Some(Topic::from(callback.next_topic)),
                    reply: callback.reply.clone(),
                });
            }
            if let Some(offer) = offers.offers.iter().find(|c| {
                c.npc == context.2
                    && c.topic == topic.number
                    && c.keywords.contains(&text)
                    && eligible(&c.storage)
                    && c.requires.iter().all(|r| r.holds(catalogue, tracks))
            }) {
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
            if matching.next().is_some() {
                return None;
            }
            Some(Choice {
                transition: Some(callback.key.clone()),
                next_topic: Some(Topic::from(callback.next_topic)),
                reply: callback.reply.clone(),
            })
        },
    )
}
