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
mod offer_match;
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
    #[serde(default)]
    crystal_keyword: bool,
    next_topic: u16,
    // A dialogue alias can select the NPC's existing fenced reward callback.
    #[serde(default)]
    transition: Option<String>,
    storage: Vec<Storage>,
    #[serde(default)]
    requires: Vec<reward_callbacks::TrackRequirement>,
    reply: Vec<String>,
    #[serde(default)]
    minimum_level: Option<u32>,
    #[serde(default)]
    failure_reply: Vec<String>,
    #[serde(default)]
    failure_topic: Option<u16>,
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

const TWENTY_MILES: &str = "oteryn:quest.twenty_miles_beneath_the_sea_quest";

fn twenty_miles_placement(
    npc: &str,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    named: EntityRef,
) -> bool {
    let at = match npc {
        "oteryn:npc.guardian_of_the_deep_seas" => (33547, 31632, -14),
        "oteryn:npc.yeo" => (33660, 31989, -7),
        _ => return true,
    };
    service_context::at_placement(runtime, actor, session, named, at)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Rewards,
    Offers,
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    inventory: Option<&super::reward_facts::RewardFacts>,
    phase: Phase,
) -> Option<Step> {
    let catalogue = admission.quest_catalogue.as_deref()?;
    let callbacks = reward_callbacks::offered_for(catalogue).ok()?;
    #[cfg(test)]
    let callbacks = admission
        .states
        .npc_conversations
        .fixture_callback_rows
        .get()
        .unwrap_or(&callbacks);
    let offers = offers::load(admission)?;
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    let placement = twenty_miles_placement(
        &context.2,
        runtime,
        actor,
        command.game_session_id,
        intent.npc_actor,
    );
    let text = intent.text.trim().to_ascii_lowercase();
    let text = if context.2 == offer_topic::CYCLOPS {
        offer_topic::amulet_keyword(&text)
    } else {
        text.as_str()
    };
    if phase == Phase::Rewards {
        offer_topic::retire_cyclops_offer(
            admission,
            command.game_session_id,
            &context,
            &offers,
            text,
        )?;
    }
    if text == "no"
        || (phase == Phase::Offers
            && !matches!(text, "yes" | "hi" | "hello" | "bye" | "farewell")
            && placement
            && offers
                .offers
                .iter()
                .any(|offer| offer.npc == context.2 && offer.matches_keyword(text)))
    {
        let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
        sessions
            .get_mut(&command.game_session_id)?
            .quest_dialogue
            .reward_request = (text != "no").then_some(context);
    }
    let catalogue = admission.quest_catalogue.as_deref()?;
    let mut failure_reply = Vec::new();
    let mut failure_topic = None;
    let mut refused = false;
    let step = quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _, tracks| {
            let text = callback_keyword(&admission.states.chat, &context.2, &intent.text);
            // The Cyclops keeps each metal quote on refusal. Apply a failure
            // topic where the source changes it, including skirt delivery.
            failure_topic = match (context.2.as_str(), topic.number, text.as_str()) {
                (offer_topic::CYCLOPS, 12, "yes") | ("oteryn:npc.yeo", 1, "yes") => Some(0),
                _ => None,
            };
            let meets_level = |minimum: Option<u32>| {
                minimum.is_none_or(|minimum| inventory.is_some_and(|facts| facts.level >= minimum))
            };
            let mut refusal = |lines: &[String], topic: Option<u16>| {
                refused = !lines.is_empty();
                (!lines.is_empty()).then(|| Choice {
                    transition: None,
                    next_topic: topic.map(Topic::from),
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
            if phase == Phase::Rewards {
                let topic = if context.2 == oteryn_gameplay_quests::quest::classic_talk::ROTTIN
                    && text == "mission"
                {
                    0
                } else {
                    topic.number
                };
                let meets_requirements = |c: &reward_callbacks::RewardCallback| {
                    c.requires.iter().all(|r| r.holds(catalogue, tracks))
                };
                let matching_stage = callbacks.iter().filter(|c| {
                    c.carried_item.is_none()
                        && (c.quest != TWENTY_MILES || placement)
                        && c.npc == context.2
                        && c.topic == topic
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
                        && (c.quest == TWENTY_MILES || meets_requirements(c))
                });
                let mut selected = matching_stage.clone().filter(|c| {
                    (c.quest != TWENTY_MILES || meets_requirements(c))
                        && c.inventory.iter().all(|rule| {
                            inventory.is_some_and(|facts| rule.holds(&facts.quantities))
                        })
                });
                if let Some(callback) = selected.next() {
                    if selected.next().is_some() {
                        return None;
                    }
                    if !meets_level(callback.minimum_level)
                        && let Some(choice) = refusal(&callback.failure_reply, failure_topic)
                    {
                        return Some(choice);
                    }
                    // Without a source refusal line, keep the exact selected
                    // transition: its locked owner must reject the level gate.
                    // Falling through would return the ordinary dialogue's Ok.
                    failure_reply = callback.failure_reply.clone();
                    return Some(Choice {
                        transition: Some(callback.key.clone()),
                        next_topic: Some(Topic::from(callback.next_topic)),
                        reply: callback.reply.clone(),
                    });
                }
                // A later stage with the same keyword cannot hide this stage's
                // explicit refusal. Successful branches retain priority.
                let mut blocked = matching_stage;
                if let Some(callback) = blocked.next()
                    && blocked.next().is_none()
                    && let Some(choice) = refusal(&callback.failure_reply, failure_topic)
                {
                    return Some(choice);
                }
                return None;
            }
            if context.2 == oteryn_gameplay_quests::quest::classic_talk::ROTTIN {
                return rottin_dialogue(topic, &text, tracks, |lines| refusal(lines, None));
            }
            if !placement {
                return None;
            }
            if let Some(offer) = offers
                .offers
                .iter()
                .find(|c| c.admits(&context.2, topic.number, &text, catalogue, tracks))
            {
                if !meets_level(offer.minimum_level) {
                    return refusal(&offer.failure_reply, offer.failure_topic.or(failure_topic));
                }
                if let Some(key) = &offer.transition {
                    // Keep the original callback, value plan and saved-definition pin.
                    callbacks.iter().find(|callback| {
                        callback.key == *key
                            && callback.npc == context.2
                            && callback.carried_item.is_none()
                            && callback.world_claim.is_none()
                    })?;
                    failure_reply = offer.failure_reply.clone();
                    failure_topic = offer.failure_topic.or(failure_topic);
                }
                return Some(Choice {
                    transition: offer.transition.clone(),
                    next_topic: Some(Topic::from(offer.next_topic)),
                    reply: offer.reply.clone(),
                });
            }
            // Earlier stages may share the terminal reward's keyword. Grizzly's
            // dedicated native dialogue and admitted programmes retain those stages;
            // the locked reward writer still proves every source predicate.
            if context.2 == oteryn_gameplay_quests::quest::grizzly_adams::NPC
                || oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog::runtime()
                    .ok()
                    .and_then(|source| {
                        source
                            .admitted_for_npc(&context.2, catalogue)
                            .ok()
                            .flatten()
                    })
                    .is_some()
            {
                return None;
            }
            // A source-owned offer may explain why a mission prerequisite is
            // missing. Select an unambiguous stage before falling back to lore.
            let mut blocked = offers.offers.iter().filter(|offer| {
                offer.npc == context.2
                    && offer.matches_topic(topic.number)
                    && offer.matches_keyword(&text)
                    && eligible(&offer.storage)
                    && !offer.requires.iter().all(|r| r.holds(catalogue, tracks))
            });
            if let Some(offer) = blocked.next()
                && blocked.next().is_none()
                && let Some(choice) =
                    refusal(&offer.failure_reply, offer.failure_topic.or(failure_topic))
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
            if let Some(other) = matching.next() {
                // Eroth uses "mission" both before accepting and for the terminal
                // report. When neither stage applies, the terminal source owns
                // the refusal, including on a new connection after completion.
                if context.2 == "oteryn:npc.eroth"
                    && callback.quest == other.quest
                    && matching.next().is_none()
                    && let Some(terminal) = [callback, other].into_iter().find(|c| c.completes)
                {
                    return refusal(&terminal.failure_reply, None);
                }
                return None;
            }
            if (!meets_level(callback.minimum_level)
                || !callback
                    .inventory
                    .iter()
                    .all(|rule| inventory.is_some_and(|facts| rule.holds(&facts.quantities))))
                && let Some(choice) = refusal(&callback.failure_reply, failure_topic)
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
    step.map(|step| {
        step.with_failure_reply(failure_reply)
            .with_failure_topic(failure_topic)
            .with_refusal(refused)
    })
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

#[cfg(any(test, feature = "test-support"))]
pub(in crate::gameplay_transport) fn fixture_rafzan_resume(
    npc: &str,
    topic: u16,
    text: &str,
    catalogue: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
    tracks: &std::collections::BTreeMap<String, i64>,
) -> Option<(u16, Vec<String>)> {
    let offers = offer_match::rafzan_offers()?;
    let offer = offers
        .offers
        .iter()
        .find(|o| o.admits(npc, topic, text, catalogue, tracks))?;
    Some((offer.next_topic, offer.reply.clone()))
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
    let text = if context.2 == oteryn_gameplay_quests::quest::classic_talk::ROTTIN {
        text
    } else {
        callback_keyword(&admission.states.chat, &context.2, &intent.text)
    };
    if admission.has_pending_paid_confirmation(command, intent, &context) {
        return false;
    }
    let Ok(sessions) = admission.states.npc_conversations.sessions.lock() else {
        return true;
    };
    let Some(state) = sessions.get(&command.game_session_id) else {
        return false;
    };
    let topic = state.quest_dialogue.topic(&context).number;
    let requested = state.quest_dialogue.reward_request.as_ref() == Some(&context);
    drop(sessions);
    let Some(catalogue) = admission.quest_catalogue.as_deref() else {
        return false;
    };
    let placement = twenty_miles_placement(
        &context.2,
        runtime,
        actor,
        command.game_session_id,
        intent.npc_actor,
    );
    reward_callbacks::offered_for(catalogue).is_ok_and(|callbacks| {
        callbacks.iter().any(|callback| {
            callback.npc == context.2
                && (callback.quest != TWENTY_MILES || placement)
                && (callback.topic == topic
                    || requested
                    // The forge requires a specific offer before it accepts yes.
                    || callback.key == "oteryn:quest-reward/qc-obsidian-draconian-steel")
                && callback.keywords.contains(&text)
        })
    })
}

/// Crystal MsgContains rejects a keyword if any occurrence follows a Lua %w byte.
pub(super) fn crystal_keyword(text: &str, word: &str) -> bool {
    let mut matches = text.match_indices(word);
    let allowed = |(index, _): (usize, &str)| {
        index == 0 || !text.as_bytes()[index - 1].is_ascii_alphanumeric()
    };
    matches.next().is_some_and(allowed) && matches.all(allowed)
}

fn callback_keyword(chat: &ChatRuntime, npc: &str, text: &str) -> String {
    use oteryn_gameplay_quests::quest::classic_talk;
    if npc == crate::durability::npc_trade::sinatuki_exchange::NPC {
        if super::npc_item_exchanges::sinatuki_default_keyword(chat, text) {
            return "nupi".into();
        }
        return String::new();
    }
    if npc == classic_talk::ROTTIN {
        return classic_talk::rottin_keyword(text)
            .unwrap_or_default()
            .into();
    }
    let trimmed = text.trim();
    if npc == offer_topic::CYCLOPS {
        if trimmed.eq_ignore_ascii_case("yes") {
            return text.to_ascii_lowercase();
        }
        return offer_topic::amulet_keyword(&trimmed.to_ascii_lowercase()).into();
    }
    trimmed.to_ascii_lowercase()
}

fn rottin_dialogue(
    topic: &Topic,
    text: &str,
    tracks: &std::collections::BTreeMap<String, i64>,
    refusal: impl FnOnce(&[String]) -> Option<Choice>,
) -> Option<Choice> {
    use oteryn_gameplay_quests::quest::{classic_dialogue, classic_talk};
    let topic = u8::try_from(topic.number)
        .ok()
        .map(classic_talk::Topic::Rottin);
    let selected = classic_talk::choose(classic_talk::ROTTIN, text, topic, tracks)?;
    if selected.deferred {
        return refusal(&["Sorry, not possible.".into()]);
    }
    let reply = selected
        .branches
        .iter()
        .map(|branch| classic_dialogue::reply(classic_talk::ROTTIN, branch))
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    let next_topic = match selected.topic {
        Some(classic_talk::Topic::Rottin(topic)) => u16::from(topic),
        _ => 0,
    };
    Some(Choice {
        transition: selected.transition.map(str::to_owned),
        next_topic: Some(Topic::from(next_topic)),
        reply,
    })
}
