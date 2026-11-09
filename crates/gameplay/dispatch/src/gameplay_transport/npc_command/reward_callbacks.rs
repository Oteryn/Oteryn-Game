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
    minimum_level: Option<u32>,
    #[serde(default)]
    failure_reply: Vec<String>,
}

struct OfferFacts<'a> {
    offers: &'a Offers,
    level: Option<u32>,
}

pub(super) fn needs_level(
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
    load_offers().is_some_and(|rows| {
        rows.offers.iter().any(|offer| {
            offer.npc == context.2
                && offer.keywords.contains(&text)
                && offer.minimum_level.is_some()
        })
    })
}

/// Read through Character's existing recovery-fenced progression owner, outside
/// the runtime lock. This is advisory dialogue eligibility, never reward authority.
pub(super) async fn read_level(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Option<u32> {
    Some(
        admission
            .root
            .read_character_progression(admission.character, command.item_fence?.character_id)
            .await
            .ok()??
            .level,
    )
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    level: Option<u32>,
) -> Option<Step> {
    let callbacks = reward_callbacks::load().ok()?;
    let offers = load_offers()?;
    prepare_with_sources(
        admission,
        actor,
        command,
        intent,
        runtime,
        &callbacks,
        OfferFacts {
            offers: &offers,
            level,
        },
    )
}

#[cfg(test)]
pub(super) fn prepare_with_callbacks(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    callbacks: &[reward_callbacks::RewardCallback],
) -> Option<Step> {
    let offers = load_offers()?;
    prepare_with_sources(
        admission,
        actor,
        command,
        intent,
        runtime,
        callbacks,
        OfferFacts {
            offers: &offers,
            level: None,
        },
    )
}

fn load_offers() -> Option<Offers> {
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
    Some(offers)
}

#[cfg(test)]
pub(super) fn prepare_offer_fixture(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    source: &str,
    level: Option<u32>,
) -> Option<Step> {
    let offers = serde_json::from_str(source).ok()?;
    prepare_with_sources(
        admission,
        actor,
        command,
        intent,
        runtime,
        &[],
        OfferFacts {
            offers: &offers,
            level,
        },
    )
}

fn prepare_with_sources(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    callbacks: &[reward_callbacks::RewardCallback],
    facts: OfferFacts<'_>,
) -> Option<Step> {
    let OfferFacts { offers, level } = facts;
    if offers.schema != 1
        || offers.offers.len() > 512
        || offers.offers.iter().any(|o| o.requires.len() > 8)
    {
        return None;
    }
    let catalogue = admission.quest_catalogue.as_deref()?;
    let mut fallback = false;
    let step = quest_dialogue::prepare(
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
                    && (c.any_topic || c.topic == topic.number)
                    && c.keywords.contains(&text)
                    && eligible(&c.storage)
                    && c.requires.iter().all(|r| r.holds(catalogue, tracks))
            }) {
                if !offer
                    .minimum_level
                    .is_none_or(|minimum| level.is_some_and(|level| level >= minimum))
                {
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
            if matching.next().is_some() {
                return None;
            }
            fallback = true;
            Some(Choice {
                transition: Some(callback.key.clone()),
                next_topic: Some(Topic::from(callback.next_topic)),
                reply: callback.reply.clone(),
            })
        },
    );
    // The common selector's quest-copy lock has been released before the
    // programme planner reads that copy. Eligible callbacks and offers retain
    // precedence; only the ineligible keyword fallback may defer.
    if fallback
        && super::catalogue::has_eligible_program(admission, actor, command, intent, runtime)
    {
        None
    } else {
        step
    }
}
