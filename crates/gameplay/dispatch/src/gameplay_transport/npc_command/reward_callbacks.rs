//! Content selects a bounded exchange; the shared quest transaction owns every effect.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::reward_callbacks;
use oteryn_gameplay_quests::quest::world_callbacks::Storage;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn needs_inventory(
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
    reward_callbacks::load().is_ok_and(|callbacks| {
        callbacks.iter().any(|callback| {
            callback.npc == context.2
                && !callback.absent.is_empty()
                && callback.keywords.contains(&text)
        })
    })
}

pub(super) async fn owned_inventory(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Option<BTreeMap<String, u64>> {
    let inventory = admission
        .root
        .read_item_use_inventory(admission.character, admission.holder, command.item_fence?)
        .await
        .ok()?;
    let mut counts = BTreeMap::<String, u64>::new();
    let mut seen = BTreeSet::new();
    for item in inventory
        .equipment
        .iter()
        .chain(inventory.backpack.iter().map(|bag| &bag.backpack))
        .chain(
            inventory
                .backpack
                .iter()
                .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item)),
        )
        .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
    {
        if seen.insert(item.item_instance_id) {
            let count = counts
                .entry(item.definition.production_key.clone())
                .or_default();
            *count = count.saturating_add(u64::from(item.quantity));
        }
    }
    Some(counts)
}

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
    #[serde(default)]
    minimum_level: Option<u32>,
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
    owned: Option<&BTreeMap<String, u64>>,
) -> Option<Step> {
    let callbacks = reward_callbacks::load().ok()?;
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
    let level = admission
        .states
        .spell_states
        .try_lock()
        .ok()?
        .get(runtime, actor, command.game_session_id)
        .map_or(0, |state| state.character_facts().level);
    let catalogue = admission.quest_catalogue.as_deref()?;
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, fresh_conversation, tracks| {
            let text = intent.text.trim().to_ascii_lowercase();
            if matches!(text.as_str(), "hi" | "hello" | "greetings") && !fresh_conversation {
                return None;
            }
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
                    && c.minimum_level
                        .is_none_or(|minimum| minimum > 0 && level >= minimum)
                    && eligible(&c.storage)
                    && owned.map_or(c.absent.is_empty(), |owned| {
                        let count = |key: &str| owned.get(key).copied().unwrap_or(0);
                        c.absent.iter().all(|key| count(key) == 0)
                            && c.present
                                .iter()
                                .all(|item| count(&item.item) >= u64::from(item.count))
                            && (c.present_any.is_empty()
                                || c.present_any
                                    .iter()
                                    .any(|item| count(&item.item) >= u64::from(item.count)))
                    })
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
                    && c.minimum_level
                        .is_none_or(|minimum| minimum > 0 && level >= minimum)
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
