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
    #[serde(default)]
    msg_contains: bool,
    topic: u16,
    #[serde(default)]
    any_topic: bool,
    #[serde(default)]
    preserve_topic: bool,
    #[serde(default)]
    open_shop: bool,
    #[serde(default)]
    reward_order: Option<RewardOrder>,
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

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RewardOrder {
    Before,
    Alongside,
}

impl Offer {
    fn choice(&self) -> Choice {
        Choice {
            transition: None,
            next_topic: (!self.preserve_topic).then(|| Topic::from(self.next_topic)),
            reply: self.reply.clone(),
        }
    }
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Rewards,
    Offers,
}

pub(super) fn cledwyn_silver_trade(npc: &str, text: &str) -> bool {
    use oteryn_gameplay_quests::quest::npc_rules::msg_contains;

    npc == "oteryn:npc.cledwyn"
        && msg_contains(text, "silver")
        && !["token", "tokens", "information", "talk"]
            .iter()
            .any(|word| msg_contains(text, word))
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
    let text = intent.text.trim().to_ascii_lowercase();
    if text == "no"
        || (!matches!(text.as_str(), "yes" | "hi" | "hello" | "bye" | "farewell")
            && offers.offers.iter().any(|offer| {
                offer.npc == context.2
                    && (phase == Phase::Offers || offer.reward_order.is_some())
                    && offer.matches_keyword(&text)
            }))
    {
        let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
        sessions
            .get_mut(&command.game_session_id)?
            .quest_dialogue
            .reward_request = (text != "no").then_some(context);
    }
    let catalogue = admission.quest_catalogue.as_deref()?;
    let mut failure_reply = Vec::new();
    let mut refused = false;
    let mut open_shop = false;
    let mut clear_topic_on_refusal = false;
    let refusal_topic = |callback: &reward_callbacks::RewardCallback| {
        callback.clear_topic_on_refusal.then(Topic::default)
    };
    let step = quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _, tracks| {
            let text = callback_keyword(&context.2, &intent.text);
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
            if phase == Phase::Rewards {
                let earlier = offers.offers.iter().find(|offer| {
                    offer.reward_order.is_some()
                        && offer.admits(&context.2, topic.number, &text, catalogue, tracks)
                });
                if let Some(offer) = earlier {
                    if !meets_level(offer.minimum_level) {
                        return refusal(&offer.failure_reply);
                    }
                    if offer.reward_order == Some(RewardOrder::Before) {
                        open_shop = offer.open_shop;
                        return Some(offer.choice());
                    }
                }
                let topic = if context.2 == oteryn_gameplay_quests::quest::classic_talk::ROTTIN
                    && text == "mission"
                {
                    0
                } else {
                    topic.number
                };
                let mut choice = (|| {
                    let matching_stage = callbacks.iter().filter(|c| {
                        c.carried_item.is_none()
                            && c.npc == context.2
                            && c.topic == topic
                            && c.matches_keyword(&text)
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
                        c.inventory.iter().all(|rule| {
                            inventory.is_some_and(|facts| rule.holds(&facts.quantities))
                        })
                    });
                    if let Some(callback) = selected.next() {
                        if !callback.accepts_npc_match(selected.next().is_some()) {
                            return None;
                        }
                        if !meets_level(callback.minimum_level)
                            && let Some(mut choice) = refusal(&callback.failure_reply)
                        {
                            choice.next_topic = refusal_topic(callback);
                            return Some(choice);
                        }
                        // Without a source refusal line, keep the exact selected
                        // transition: its locked owner must reject the level gate.
                        // Falling through would return the ordinary dialogue's Ok.
                        failure_reply = callback.failure_reply.clone();
                        clear_topic_on_refusal = callback.clear_topic_on_refusal;
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
                        && callback.accepts_npc_match(blocked.next().is_some())
                        && let Some(mut choice) = refusal(&callback.failure_reply)
                    {
                        choice.next_topic = refusal_topic(callback);
                        return Some(choice);
                    }
                    None
                })();
                if let Some(offer) = earlier {
                    if let Some(choice) = &mut choice {
                        choice.reply.splice(..0, offer.reply.iter().cloned());
                        failure_reply.splice(..0, offer.reply.iter().cloned());
                    } else {
                        choice = Some(offer.choice());
                    }
                }
                return choice;
            }
            if context.2 == oteryn_gameplay_quests::quest::classic_talk::ROTTIN {
                return rottin_dialogue(topic, &text, tracks, refusal);
            }
            if let Some(offer) = offers.offers.iter().find(|c| {
                c.reward_order.is_none()
                    && c.admits(&context.2, topic.number, &text, catalogue, tracks)
            }) {
                if !meets_level(offer.minimum_level) {
                    return refusal(&offer.failure_reply);
                }
                return Some(offer.choice());
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
                    && c.matches_keyword(&text)
            });
            let callback = matching.next()?;
            let other = matching.next();
            if !callback.accepts_npc_match(other.is_some()) {
                // Eroth uses "mission" both before accepting and for the terminal
                // report. When neither stage applies, the terminal source owns
                // the refusal, including on a new connection after completion.
                if let Some(other) = other
                    && context.2 == "oteryn:npc.eroth"
                    && callback.quest == other.quest
                    && matching.next().is_none()
                    && let Some(terminal) = [callback, other].into_iter().find(|c| c.completes)
                {
                    return refusal(&terminal.failure_reply);
                }
                return None;
            }
            if (!meets_level(callback.minimum_level)
                || !callback
                    .inventory
                    .iter()
                    .all(|rule| inventory.is_some_and(|facts| rule.holds(&facts.quantities))))
                && let Some(mut choice) = refusal(&callback.failure_reply)
            {
                choice.next_topic = refusal_topic(callback);
                return Some(choice);
            }
            failure_reply = callback.failure_reply.clone();
            clear_topic_on_refusal = callback.clear_topic_on_refusal;
            Some(Choice {
                transition: Some(callback.key.clone()),
                next_topic: Some(Topic::from(callback.next_topic)),
                reply: callback.reply.clone(),
            })
        },
    );
    step.map(|step| {
        step.with_failure_reply(failure_reply)
            .with_refusal(refused)
            .with_quote_retirement(clear_topic_on_refusal)
            .with_shop_window(open_shop)
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

#[cfg(test)]
#[test]
fn regional_hunt_dialogue_earlier_offers_admit_mixed_keywords() -> Result<(), String> {
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("hunt-offers")
        .map_err(|error| format!("{error:?}"))?;
    let offers: Offers = serde_json::from_str(include_str!(
        "../../../../../../content/quests/shared-reward-offers.json"
    ))
    .map_err(|error| error.to_string())?;
    let mut missed = Vec::new();
    for npc in ["oteryn:npc.lugri", "oteryn:npc.gabel", "oteryn:npc.malor"] {
        let mut matching = offers
            .offers
            .iter()
            .filter(|offer| offer.npc == npc && offer.topic == 0)
            .peekable();
        assert!(matching.peek().is_some(), "missing offers for {npc}");
        for offer in matching {
            let tracks = offer
                .storage
                .iter()
                .map(|s| (s.track.clone(), s.minimum))
                .chain(offer.requires.iter().map(|r| (r.track.clone(), r.minimum)))
                .collect();
            for keyword in &offer.keywords {
                let message = format!("{keyword} task");
                if !offer.admits(npc, 0, &message, loaded.catalogue(), &tracks) {
                    missed.push((npc, message));
                }
            }
        }
    }
    assert!(missed.is_empty(), "earlier offers missed: {missed:?}");
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
    reward_callbacks::offered_for(catalogue).is_ok_and(|callbacks| {
        callbacks.iter().any(|callback| {
            callback.npc == context.2
                && (callback.topic == topic
                    || requested
                    // The forge requires a specific offer before it accepts yes.
                    || callback.key == "oteryn:quest-reward/qc-obsidian-draconian-steel")
                && callback.matches_keyword(&text)
        })
    })
}

fn callback_keyword(npc: &str, text: &str) -> String {
    use oteryn_gameplay_quests::quest::classic_talk;
    if npc == classic_talk::ROTTIN {
        return classic_talk::rottin_keyword(text)
            .unwrap_or_default()
            .into();
    }
    text.trim().to_ascii_lowercase()
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
