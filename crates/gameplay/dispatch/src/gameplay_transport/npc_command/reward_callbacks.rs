//! Content selects a bounded exchange; the shared quest transaction owns every effect.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::Storage;
use oteryn_gameplay_quests::quest::{dark_trails::message_contains, reward_callbacks};
use serde::Deserialize;

#[cfg(test)]
mod any_topic_tests;
#[cfg(test)]
mod fixture_offers;
mod offer_match;
mod offer_topic;
mod offers;
mod selection_facts;
mod storkus;
pub(super) use selection_facts::needs_facts;

pub fn validate_quest_reward_offers() -> bool {
    offers::embedded().is_some()
}

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
    source_dialogue: bool,
    #[serde(default)]
    reward_order: Option<RewardOrder>,
    next_topic: u16,
    storage: Vec<Storage>,
    #[serde(default)]
    requires: Vec<reward_callbacks::TrackRequirement>,
    #[serde(default)]
    inventory: Vec<reward_callbacks::InventoryRequirement>,
    reply: Vec<String>,
    #[serde(default)]
    minimum_level: Option<u32>,
    #[serde(default, alias = "premium")]
    requires_premium: Option<bool>,
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
    Rewards { skip_opening: Option<&'static str> },
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

pub(super) fn alternate_opening(
    catalogue: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
    npc: &str,
    text: &str,
) -> Option<&'static str> {
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog;

    let (authored, has_source) = match (npc, text) {
        ("oteryn:npc.hairycles", "mission") => (
            "oteryn:quest-reward/ape-city/start",
            NpcRuleCatalog::runtime()
                .and_then(|rules| rules.admitted_for_npc(npc, catalogue))
                .is_ok_and(|rules| rules.is_some()),
        ),
        ("oteryn:npc.angus", "join") => (
            "oteryn:quest-reward/explorer-society/enrol-angus",
            catalogue
                .transition("oteryn:quest-reward/explorer-society/angus/enroll")
                .is_some(),
        ),
        ("oteryn:npc.mortimer", "join") => (
            "oteryn:quest-reward/explorer-society/enrol-mortimer",
            catalogue
                .transition("oteryn:quest-reward/explorer-society/mortimer/enroll")
                .is_some(),
        ),
        _ => return None,
    };
    (has_source && catalogue.transition(authored).is_some()).then_some(authored)
}

pub(super) async fn prefer_source_start(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> bool {
    match (
        admission.session_account(session),
        admission.premium_coordinator,
    ) {
        (None, _) => true,
        (Some(account), Some(owner)) => !owner.premium_gate(account).await.premium_current(),
        (Some(_), None) => false,
    }
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
    let text = callback_keyword(admission, command, &context.2, &intent.text);
    let raw_text = intent.text.trim().to_ascii_lowercase();
    let requests_offer = if context.2 == storkus::NPC {
        &text
    } else {
        &raw_text
    };
    let says_no = raw_text == "no";
    if says_no
        || (!matches!(
            requests_offer.as_str(),
            "yes" | "hi" | "hello" | "bye" | "farewell"
        ) && offers.iter().any(|offer| {
            offer.npc == context.2
                && (phase == Phase::Offers || offer.reward_order.is_some())
                && offer.matches_keyword(requests_offer)
        }))
    {
        let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
        sessions
            .get_mut(&command.game_session_id)?
            .quest_dialogue
            .reward_request = (!says_no).then_some(context);
    }
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
            const EROTH_AUTHORED: &str = "oteryn:quest-reward/to-blind-the-enemy/";
            const EROTH_LEGACY: &str = "oteryn:quest-reward/elvenbane-mirror-";
            const EROTH_LEGACY_TRACK: &str =
                "oteryn:quest-progress/quest/u8_1/to_blind_the_enemy/questline";
            // Both Eroth programmes use the same NPC and confirmation topic.
            // New players start the authored journey; existing legacy saves
            // keep their route through the mirror and report.
            let eroth_uses_authored = (context.2 == "oteryn:npc.eroth"
                && catalogue
                    .transition("oteryn:quest-reward/to-blind-the-enemy/offer")
                    .is_some())
            .then(|| {
                tracks
                    .get(EROTH_LEGACY_TRACK)
                    .is_none_or(|stage| *stage <= 0)
                    || tracks
                        .get("oteryn:quest-progress/to-blind-the-enemy/mission")
                        .is_some_and(|stage| *stage > 0)
            });
            let callbacks = callbacks
                .iter()
                .filter(|callback| match eroth_uses_authored {
                    Some(true) => !callback.key.starts_with(EROTH_LEGACY),
                    Some(false) => !callback.key.starts_with(EROTH_AUTHORED),
                    None => true,
                });
            let offers = offers.iter().filter(|offer| {
                eroth_uses_authored != Some(true)
                    || offer.npc != context.2
                    || !offer
                        .storage
                        .iter()
                        .any(|gate| gate.track == EROTH_LEGACY_TRACK)
            });
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
            let track_value = |key: &str| {
                tracks
                    .get(key)
                    .copied()
                    .or_else(|| catalogue.track(key).map(|track| track.initial))
            };
            let eligible = |storage: &[Storage]| {
                storage.iter().all(|s| {
                    track_value(&s.track)
                        .is_some_and(|value| (s.minimum..=s.maximum).contains(&value))
                })
            };
            let account_matches = |offer: &Offer| {
                offer.requires_premium.is_none_or(|required| {
                    let premium = admission
                        .session_account(command.game_session_id)
                        .zip(admission.premium_coordinator)
                        .is_some_and(|(account, owner)| {
                            owner.view(account).premium_gate().premium_current()
                        });
                    premium == required
                })
            };
            let carried_matches = |offer: &Offer| {
                offer
                    .inventory
                    .iter()
                    .all(|rule| inventory.is_some_and(|facts| rule.holds(&facts.quantities)))
            };
            if let Phase::Rewards { skip_opening } = phase {
                let earlier = offers.clone().find(|offer| {
                    offer.reward_order.is_some()
                        && offer.admits(&context.2, topic.number, &text, catalogue, tracks)
                        && carried_matches(offer)
                        && account_matches(offer)
                });
                if let Some(offer) = earlier {
                    if !meets_level(offer.minimum_level) {
                        return refusal(&offer.failure_reply);
                    }
                    if offer.source_dialogue {
                        return None;
                    }
                    if offer.reward_order == Some(RewardOrder::Before) {
                        open_shop = offer.open_shop;
                        return Some(offer.choice());
                    }
                }
                let topic = if text == "mission"
                    && matches!(
                        context.2.as_str(),
                        oteryn_gameplay_quests::quest::classic_talk::ROTTIN | storkus::NPC
                    ) {
                    0
                } else {
                    topic.number
                };
                let mut choice = (|| {
                    let matching_stage = callbacks.filter(|c| {
                        c.carried_item.is_none()
                        && c.npc == context.2
                        && c.topic == topic
                        && matches_reward_keyword(c, &text)
                        && skip_opening != Some(c.key.as_str())
                        // Hairycles continues a saved source journey before
                        // offering the start of the separate authored route.
                        && !(c.key == "oteryn:quest-reward/ape-city/start"
                            && tracks
                                .get("oteryn:quest-progress/quest/u7_6/the_ape_city/questline")
                                .is_some_and(|stage| *stage > 0))
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
                    // Both programmes can teach Melchior's single greeting. The
                    // active Appease stage owns it when its transition is eligible.
                    let appease_greeting = context.2 == "oteryn:npc.melchior"
                        && matching_stage.clone().any(|c| {
                            c.key == "oteryn:quest-reward/tibia-tales/appease-learn-greeting"
                        });
                    let matching_stage = matching_stage.filter(|c| {
                        !appease_greeting || c.key != "oteryn:quest-reward/marid-melchior-greeting"
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
                    // Crystal storkus.lua:121-141 displays 20 minus the saved counter
                    // for an empty deposit and resets the topic.
                    if context.2 == storkus::NPC
                        && topic == storkus::COLLECTION_TOPIC
                        && let Some(saved) = track_value(storkus::DUST)
                        && let Some(facts) = inventory
                        && let Some(payment) = matching_stage
                            .clone()
                            .find_map(|callback| callback.take.first())
                        && facts.quantities.get(&payment.item).copied().unwrap_or(0) == 0
                    {
                        return Some(Choice {
                            transition: None,
                            next_topic: Some(Topic::from(0)),
                            reply: vec![reward_callbacks::storkus_dust_reply(0, saved)],
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
            if let Some(offer) = offers.clone().find(|c| {
                c.reward_order.is_none()
                    && c.admits(&context.2, topic.number, &text, catalogue, tracks)
                    && carried_matches(c)
                    && account_matches(c)
            }) {
                if !meets_level(offer.minimum_level) {
                    return refusal(&offer.failure_reply);
                }
                let mut choice = offer.choice();
                if context.2 == storkus::NPC
                    && text == "mission"
                    && offer.next_topic == storkus::COLLECTION_TOPIC
                {
                    choice.reply = storkus::collection_prompt(&offer.reply, inventory?);
                }
                return Some(choice);
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
            let mut blocked = offers.filter(|offer| {
                offer.npc == context.2
                    && offer.matches_topic(topic.number)
                    && offer.matches_keyword(&text)
                    && eligible(&offer.storage)
                    && carried_matches(offer)
                    && account_matches(offer)
                    && !offer.requires.iter().all(|r| r.holds(catalogue, tracks))
            });
            if let Some(offer) = blocked.next()
                && blocked.next().is_none()
                && let Some(choice) = refusal(&offer.failure_reply)
            {
                return Some(choice);
            }
            // A unique exchange owns its keyword even before eligibility.
            // Let the locked writer return the stage refusal, rather than falling
            // through to an unrelated legacy dialogue with the same text.
            let mut matching = callbacks.filter(|c| {
                c.carried_item.is_none()
                    && c.npc == context.2
                    && c.topic == topic.number
                    && matches_reward_keyword(c, &text)
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
    received_at: oteryn_simulation_determinism::SemanticTimeMicros,
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
    if admission.has_pending_paid_confirmation(command, intent, &context, received_at) {
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
    let text = if context.2 == storkus::NPC {
        callback_keyword(admission, command, &context.2, &intent.text)
    } else {
        text
    };
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
                && matches_reward_keyword(callback, &text)
        })
    })
}

const JORGE: &str = "oteryn:npc.jorge";

fn matches_menu_or_exact_keyword(npc: &str, keywords: &[String], text: &str) -> bool {
    keywords.iter().any(|word| {
        word == text || (npc == JORGE && word == "souvenir" && message_contains(text, word))
    })
}

fn matches_reward_keyword(callback: &reward_callbacks::RewardCallback, text: &str) -> bool {
    if callback.npc == JORGE
        && callback.quest == "oteryn:quest.hunting_for_tokens"
        && callback.topic == 1
        && callback.give.is_some()
    {
        // Jorge opens the menu before checking item names, even at the purchase topic.
        return !message_contains(text, "souvenir")
            && callback
                .keywords
                .iter()
                .any(|word| message_contains(text, word));
    }
    callback.matches_keyword(text)
        || (callback.npc == JORGE
            && callback.exclude_keywords.is_empty()
            && matches_menu_or_exact_keyword(&callback.npc, &callback.keywords, text))
}

#[cfg(test)]
#[test]
fn jorge_matches_souvenir_and_all_replica_names_in_messages() -> Result<(), String> {
    let callbacks = reward_callbacks::load()?;
    let jorge = callbacks.iter().filter(|callback| callback.npc == JORGE);
    let menu = jorge
        .clone()
        .find(|callback| callback.key.ends_with("/questpc7-tokens-replica-rewards"))
        .ok_or("Jorge's initial menu")?;
    let repeat_menu = offers::embedded()
        .and_then(|document| document.offers.iter().find(|offer| offer.npc == JORGE))
        .ok_or("Jorge's repeat menu")?;
    let menu_message = "What souvenirs do you have?";
    assert!(matches_reward_keyword(menu, menu_message));
    assert!(repeat_menu.matches_keyword(menu_message));
    assert!(!matches_reward_keyword(menu, "xSouvenir souvenir"));
    assert!(!repeat_menu.matches_keyword("xSouvenir souvenir"));

    let exchanges = jorge.filter(|callback| callback.give.is_some());
    assert_eq!(exchanges.clone().count(), 74);
    for callback in exchanges {
        for word in &callback.keywords {
            assert!(
                matches_reward_keyword(
                    callback,
                    &format!("I'd like {}, please", word.to_ascii_uppercase())
                ),
                "{} must accept {word} in a message",
                callback.key
            );
            assert!(!matches_reward_keyword(callback, &format!("x{word}")));
            assert!(!matches_reward_keyword(
                callback,
                &format!("souvenir {word}")
            ));
        }
    }
    Ok(())
}

fn callback_keyword(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    npc: &str,
    text: &str,
) -> String {
    use oteryn_gameplay_quests::quest::classic_talk;
    if npc == classic_talk::ROTTIN {
        return classic_talk::rottin_keyword(text)
            .unwrap_or_default()
            .into();
    }
    use oteryn_gameplay_quests::quest::grave_danger;
    if npc == grave_danger::NPC && grave_danger::matches_report(text) {
        return "report".into();
    }
    if npc == storkus::NPC {
        return storkus::keyword(admission, command, text)
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
