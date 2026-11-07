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
    let offers: Offers = serde_json::from_str(include_str!(
        "../../../../../../content/quests/shared-reward-offers.json"
    ))
    .ok()?;
    if offers.schema != 1 || offers.offers.iter().any(|o| o.requires.len() > 8) {
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
    )
}

/// A refused clock may speak only from a new coherent database read.
pub(super) async fn wait_reply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    key: &str,
) -> Result<Option<(u16, Vec<String>)>, ()> {
    let callback = reward_callbacks::load()
        .map_err(|_| ())?
        .into_iter()
        .find(|c| c.key == key)
        .ok_or(())?;
    if callback.clock_wait_reply.is_empty() {
        return Ok(None);
    }
    let catalogue = admission.quest_catalogue.as_deref().ok_or(())?;
    let fence = admission.current_quest_fence(session).await?.ok_or(())?;
    let (copy, now) = admission
        .root
        .read_character_quest_state_at(admission.character, fence.character_id)
        .await
        .map_err(|_| ())?;
    let value = |key: &str| {
        catalogue
            .track(key)
            .map(|track| copy.tracks().get(key).copied().unwrap_or(track.initial))
    };
    if callback
        .storage
        .iter()
        .any(|s| !value(&s.track).is_some_and(|v| (s.minimum..=s.maximum).contains(&v)))
        || callback
            .requires
            .iter()
            .any(|r| !r.holds(catalogue, copy.tracks()))
    {
        return Ok(None);
    }
    let remaining = callback.clock_effects().find_map(|clock| {
        let seconds = clock.elapsed_seconds?;
        let remaining = i128::from(seconds) - (i128::from(now) - i128::from(value(&clock.track)?));
        (remaining > 0).then_some(remaining)
    });
    let Some(remaining) = remaining else {
        return Ok(None);
    };
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
    Ok(Some((
        callback.next_topic,
        callback
            .clock_wait_reply
            .iter()
            .map(|line| line.replace("|REMAINING|", &time))
            .collect(),
    )))
}
