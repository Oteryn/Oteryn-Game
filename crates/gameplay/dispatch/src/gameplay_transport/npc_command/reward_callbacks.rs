//! Content selects a bounded exchange; the shared quest transaction owns every effect.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::reward_callbacks;
use oteryn_gameplay_quests::quest::world_callbacks::Storage;
use serde::Deserialize;

#[cfg(test)]
mod any_topic_tests;
#[cfg(test)]
mod fixture_offers;
mod offer_topic;
mod offers;
mod selection_facts;
pub(super) use selection_facts::needs_facts;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Offers {
    schema: u32,
    offers: Vec<Offer>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Offer {
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
    minimum_level: Option<u32>,
    #[serde(default)]
    failure_reply: Vec<String>,
}

#[cfg(test)]
impl NpcConversations {
    pub(in crate::gameplay_transport) fn fixture_reward_callbacks(
        &self,
        rows: Vec<reward_callbacks::RewardCallback>,
    ) -> Result<(), String> {
        self.fixture_callback_rows
            .set(rows)
            .map_err(|_| "reward callback fixture already installed".to_owned())
    }
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    inventory: Option<&super::reward_facts::RewardFacts>,
) -> Option<Step> {
    let catalogue = admission.quest_catalogue.as_deref()?;
    let callbacks = reward_callbacks::load_for(catalogue).ok()?;
    #[cfg(test)]
    let callbacks = admission
        .states
        .npc_conversations
        .fixture_callback_rows
        .get()
        .unwrap_or(&callbacks);
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
    let m09: Offers = serde_json::from_str(include_str!(
        "../../../../../../content/quests/npc-offers/m09-rookie.json"
    ))
    .ok()?;
    if m09.schema != 1 {
        return None;
    }
    offers.offers.extend(m09.offers);
    let markers: Offers = serde_json::from_str(include_str!(
        "../../../../../../content/quests/npc-offers/m16-targuna-marker.json"
    ))
    .ok()?;
    if markers.schema != 1 {
        return None;
    }
    offers.offers.extend(markers.offers);
    #[cfg(test)]
    if let Some(rows) = admission.states.npc_conversations.fixture_offer_rows.get() {
        offers.offers.extend(rows.iter().cloned());
    }
    let koshei: Offers = serde_json::from_str(include_str!(
        "../../../../../../content/quests/npc-offers/qnpcfx-koshei.json"
    ))
    .ok()?;
    if koshei.schema != 1 {
        return None;
    }
    offers.offers.extend(koshei.offers);
    let m10: Offers = serde_json::from_str(include_str!(
        "../../../../../../content/quests/npc-offers/qnpcfx-m10.json"
    ))
    .ok()?;
    if m10.schema != 1 {
        return None;
    }
    offers.offers.extend(m10.offers);
    if offers.schema != 1
        || offers.offers.len() > 512
        || offers.offers.iter().any(|o| o.requires.len() > 8)
    {
        return None;
    }
    quest_dialogue::prepare(
    let offers = offers::load(admission)?;
    let catalogue = admission.quest_catalogue.as_deref()?;
    let mut failure_reply = Vec::new();
    let mut refused = false;
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
            let mut refusal = |lines: &[String]| {
                refused = !lines.is_empty();
                (!lines.is_empty()).then(|| Choice {
                    transition: None,
                    next_topic: None,
                    reply: lines.to_vec(),
                })
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
            let matching_stage = callbacks.iter().filter(|c| {
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
            let mut selected = matching_stage.clone().filter(|c| {
                c.inventory
                    .iter()
                    .all(|rule| inventory.is_some_and(|facts| rule.holds(&facts.quantities)))
            });
            if let Some(callback) = selected.next() {
                if selected.next().is_some() {
                    return None;
                }
                if !meets_level(callback.minimum_level) {
                    return refusal(&callback.failure_reply);
                }
                failure_reply = callback.failure_reply.clone();
                return Some(Choice {
                    transition: Some(callback.key.clone()),
                    next_topic: Some(Topic::from(callback.next_topic)),
                    reply: callback.reply.clone(),
                });
            }
            // A later stage with the same keyword cannot hide this stage's
            // explicit inventory refusal. Successful branches retain priority.
            let mut blocked = matching_stage;
            if let Some(callback) = blocked.next()
                && blocked.next().is_none()
                && let Some(choice) = refusal(&callback.failure_reply)
            {
                return Some(choice);
            }
            if let Some(offer) = offers.offers.iter().find(|c| {
                c.npc == context.2
                    && c.matches_topic(topic.number)
                    && c.keywords.contains(&text)
                    && eligible(&c.storage)
                    && c.requires.iter().all(|r| r.holds(catalogue, tracks))
            }) {
                if !meets_level(offer.minimum_level) {
                    return refusal(&offer.failure_reply);
                }
                return Some(Choice {
                    transition: None,
                    next_topic: Some(Topic::from(offer.next_topic)),
                    reply: offer.reply.clone(),
                });
            }
            // A source-owned offer may explain why a mission prerequisite is
            // missing. Select an unambiguous stage before falling back to lore.
            let mut blocked = offers.offers.iter().filter(|offer| {
                offer.npc == context.2
                    && offer.matches_topic(topic.number)
                    && offer.keywords.contains(&text)
                    && eligible(&offer.storage)
                    && !offer.requires.iter().all(|r| r.holds(catalogue, tracks))
            });
            if let Some(offer) = blocked.next()
                && blocked.next().is_none()
                && let Some(choice) = refusal(&offer.failure_reply)
            {
                return Some(choice);
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
            if (!meets_level(callback.minimum_level)
                || !callback
                    .inventory
                    .iter()
                    .all(|rule| inventory.is_some_and(|facts| rule.holds(&facts.quantities))))
                && let Some(choice) = refusal(&callback.failure_reply)
            {
                return Some(choice);
            }
            failure_reply = callback.failure_reply.clone();
            Some(Choice {
                transition: Some(callback.key.clone()),
                next_topic: Some(Topic::from(callback.next_topic)),
                reply: callback.reply.clone(),
            })
        },
    );
    step.map(|step| step.with_failure_reply(failure_reply).with_refusal(refused))
}

#[cfg(test)]
#[test]
fn sterling_marker_offer_and_refusal_follow_the_source_topics() -> Result<(), serde_json::Error> {
    let offers: Offers = serde_json::from_str(include_str!(
        "../../../../../../content/quests/npc-offers/m16-targuna-marker.json"
    ))?;
    assert_eq!(offers.offers.len(), 2);
    let offer = &offers.offers[0];
    assert_eq!(offer.npc, "oteryn:npc.sterling");
    assert_eq!((offer.topic, offer.next_topic), (0, 2));
    assert!(
        offer.requires.is_empty(),
        "the mission question precedes the Morla check"
    );
    let refusal = &offers.offers[1];
    assert_eq!(refusal.npc, offer.npc);
    assert_eq!((refusal.topic, refusal.next_topic), (2, 0));
    assert!(
        refusal
            .storage
            .iter()
            .any(|r| r.track.ends_with("/morla_done")
                && r.minimum == 0
                && r.maximum == 0
                && r.preserve)
    );
    Ok(())
}
