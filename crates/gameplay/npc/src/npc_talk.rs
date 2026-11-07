//! NPC talk over the chat path (NPC-TALK-1, CHAT-0 §3, NPC-0 §3.3).
//!
//! A `say` near a runtime NPC is answered by that NPC from catalogue data: a greeting starts a
//! conversation, a topic keyword answers while the speaker stays in range, a farewell ends it.
//! The reply is an ordinary local line spoken by the NPC, so it reaches every session that hears
//! the NPC. Trade and bank are not here (NPC-TRADE-1, BANK-NPC-1). Nothing is durable.

use std::collections::HashMap;

mod cosmetics;
mod dark_trails;
mod isle_of_evil;
mod summer;

use crate::content::{NpcServiceModel, ProjectV2ServiceOffer};
use crate::foundation::speech::ChatPosition;
use crate::greeting::{DEFAULT_TALK_RANGE, contains_words, words};
use crate::greeting::{GreetingNpc, greeted_npc};

/// Words that start a conversation, until an NPC carries its own.
const GREETINGS: [&str; 3] = ["hi", "hello", "hail"];
/// Words that end a conversation.
const FAREWELLS: [&str; 3] = ["bye", "farewell", "goodbye"];
/// Words that ask what the NPC offers.
const TOPICS: [&str; 4] = ["job", "offer", "trade", "travel"];

const REWARD_CALLBACKS: &str =
    include_str!("../../../../content/quests/shared-reward-callbacks.json");
const REWARD_GREETINGS: &str =
    include_str!("../../../../content/dialogues/quests/shared-reward-greetings.json");

#[derive(serde::Deserialize)]
struct RewardMessages {
    npc: String,
    greet: String,
    farewell: String,
    walkaway: String,
}

fn reward_messages(key: &str) -> Option<&'static RewardMessages> {
    #[derive(serde::Deserialize)]
    struct Document {
        schema: u32,
        npcs: Vec<RewardMessages>,
    }
    static DATA: std::sync::OnceLock<Option<Vec<RewardMessages>>> = std::sync::OnceLock::new();
    let rows = DATA.get_or_init(|| {
        serde_json::from_str::<Document>(REWARD_GREETINGS)
            .ok()
            .filter(|d| d.schema == 1 && d.npcs.len() <= 256)
            .map(|d| d.npcs)
    });
    reward_callback_npc(key).then_some(())?;
    rows.as_ref()?.iter().find(|r| r.npc == key)
}

// Project only the identities here. The Quest loader owns validation of the
// complete callback; admitting a greeting grants no durable reward authority.
fn reward_callback_npc(key: &str) -> bool {
    #[derive(serde::Deserialize)]
    struct Document {
        schema: u32,
        callbacks: Vec<Identity>,
    }
    #[derive(serde::Deserialize)]
    struct Identity {
        npc: String,
    }
    static NPCS: std::sync::OnceLock<std::collections::BTreeSet<String>> =
        std::sync::OnceLock::new();
    NPCS.get_or_init(|| {
        serde_json::from_str::<Document>(REWARD_CALLBACKS)
            .ok()
            .filter(|d| d.schema == 1 && d.callbacks.len() <= 256)
            .map(|d| d.callbacks.into_iter().map(|c| c.npc).collect())
            .unwrap_or_default()
    })
    .contains(key)
}

fn literal_keyword(keyword: &crate::content::ProjectV2DialogueKeyword) -> bool {
    keyword.reply.len() == 1
        && keyword.children.is_empty()
        && !keyword.fallback
        && !keyword.only_unfocus
        && !keyword.reset
        && !keyword.ungreet
        && keyword.move_up.is_none()
        && keyword.links.is_empty()
}

/// What one NPC says, from its generated catalogue replies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcTalkEntry {
    pub name: String,
    bank: bool,
    admitted_dialogue: bool,
    dialogue_keywords: Vec<crate::content::ProjectV2DialogueKeyword>,
    greetings: Vec<String>,
    greeting_reply: String,
    farewell_reply: String,
    source_name_templates: bool,
    source_walkaway: Option<String>,
    /// The service lines, joined; `None` when the NPC has no service.
    topic_reply: Option<String>,
    /// The admitted offers of every Service of the NPC, in source order (NPC-CMD-1b).
    offers: Vec<ProjectV2ServiceOffer>,
    pub travel_routes: Vec<crate::content::npc_catalogue::travel::TravelRoute>,
    pub premium_services: Vec<crate::content::npc_catalogue::premium_services::PremiumNpcService>,
}

impl NpcTalkEntry {
    pub fn bank(&self) -> bool {
        self.bank
    }

    /// The admitted offers the trade window projects.
    pub fn offers(&self) -> &[ProjectV2ServiceOffer] {
        &self.offers
    }

    /// The line that ends a conversation.
    pub fn farewell(&self) -> &str {
        &self.farewell_reply
    }

    /// Render a qualified source template once using the admitted customer name.
    pub fn render_source_text(&self, text: &str, player_name: &str) -> String {
        if self.source_name_templates {
            crate::quest_greetings::render_player_name(text, player_name)
        } else {
            text.to_owned()
        }
    }

    pub fn walkaway(&self) -> &str {
        self.source_walkaway.as_deref().unwrap_or(self.farewell())
    }

    pub fn walkaway_for(&self, player_name: &str) -> String {
        self.render_source_text(self.walkaway(), player_name)
    }

    #[doc(hidden)]
    pub fn new(name: &str, topic: Option<&str>) -> Self {
        Self {
            name: name.to_owned(),
            bank: false,
            admitted_dialogue: false,
            dialogue_keywords: Vec::new(),
            greetings: GREETINGS.iter().map(|word| (*word).to_owned()).collect(),
            greeting_reply: format!("Greetings, I am {name}."),
            farewell_reply: "Farewell.".to_owned(),
            source_name_templates: false,
            source_walkaway: None,
            topic_reply: topic.map(str::to_owned),
            offers: Vec::new(),
            premium_services: Vec::new(),
            travel_routes: Vec::new(),
        }
    }

    #[doc(hidden)]
    pub fn with_bank(mut self) -> Self {
        self.bank = true;
        self
    }

    #[doc(hidden)]
    pub fn with_offers(mut self, offers: Vec<ProjectV2ServiceOffer>) -> Self {
        self.offers = offers;
        self
    }
}

/// Catalogue display names and the qualified native talk entries, by NPC key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NpcTalkBook {
    /// Source-qualified automatic speech, installed only by the catalogue/bundle boot binding.
    source_voices: std::collections::BTreeMap<String, crate::npc_voice::VoiceProfile>,
    /// S-ENTNAME: all admitted catalogue names, including NPCs without native dialogue.
    display_names: HashMap<String, String>,
    npcs: HashMap<String, NpcTalkEntry>,
    /// The revision the trade windows carry: derived from the catalogue's source tree digest, so
    /// it changes exactly when the content the offers come from changes. Never 0.
    catalogue_revision: u64,
}

impl NpcTalkBook {
    pub fn bind_source_voices(
        &mut self,
        catalogue: &crate::content::NpcDataCatalogue,
        model: &NpcServiceModel,
        qualify: impl Fn(&str) -> Option<String>,
    ) -> Result<(), crate::npc_voice::VoiceError> {
        if self.catalogue_revision != Self::from_model(model).catalogue_revision {
            return Err(crate::npc_voice::VoiceError::ContentSourceMismatch);
        }
        self.source_voices = crate::npc_voice::profiles_from_catalogue(catalogue, model, qualify)?;
        Ok(())
    }

    pub fn source_voices(
        &self,
    ) -> &std::collections::BTreeMap<String, crate::npc_voice::VoiceProfile> {
        &self.source_voices
    }

    /// Entries for every NPC with generated replies. An NPC with an admitted Dialogue is left out
    /// until a dialogue packet reads it.
    pub fn from_model(model: &NpcServiceModel) -> Self {
        let mut book = Self {
            catalogue_revision: revision_of(&format!(
                "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
                crate::content::npc_catalogue::grizzly_shop::book_digest(
                    model.source_tree_digest()
                ),
                dark_trails::TEXT,
                isle_of_evil::revision_material(),
                REWARD_CALLBACKS,
                REWARD_GREETINGS,
                cosmetics::SOURCE,
                cosmetics::PROGRAM_SOURCE,
                summer::SOURCE,
                include_str!("quest_greetings/seasonal.json"),
                include_str!("quest_greetings/horse_rental.json"),
                crate::quest_greetings::FURNITURE_SOURCE,
            )),
            ..Self::default()
        };
        for (reference, npc) in model.npcs() {
            book.display_names
                .insert(reference.key.clone(), npc.name.clone());
            let cosmetic_messages = cosmetics::messages(&reference.key);
            let dark_messages = dark_trails::messages(&reference.key);
            let isle_messages = isle_of_evil::messages(&reference.key);
            let reward_messages = reward_messages(&reference.key);
            let premium_services = crate::content::npc_catalogue::premium_services::resolve(
                &reference.key,
                &npc.services,
            );
            let travel_routes = crate::content::npc_catalogue::travel::resolve(
                model,
                &reference.key,
                &npc.services,
            );
            let bank = npc
                .services
                .iter()
                .filter_map(|reference| model.services().get(reference))
                .any(|service| service.bank);
            let (greeting_reply, farewell_reply, topic_reply, dialogue_keywords) = if reference.key
                == "oteryn:npc.grizzly_adams"
            {
                // W15c: the existing NPC owner opens the conversation; confirmed task
                // choices and their stateful replies belong to native Grizzly dispatch.
                (
                    "Welcome to the Paw and Fur - Hunting Elite. Would you like to join?".into(),
                    "Good hunting!".into(),
                    Some("Ask me about tasks, points, rank, promotion or bosses.".into()),
                    Vec::new(),
                )
            } else if matches!(
                reference.key.as_str(),
                "oteryn:npc.ambassador_manop" | "oteryn:npc.enpa_deia_pema"
            ) {
                // MONKQUEST3: greeting belongs to the NPC owner; stateful pilgrimage
                // replies and all writes belong to the native quest caller in dispatch.
                (
                    format!(
                        "Harmony, friend. I am {}. Ask me about the pilgrimage.",
                        npc.name
                    ),
                    "May your path lead to enlightenment.".into(),
                    Some("Ask me about your pilgrimage or progress on the Three-Fold Path.".into()),
                    Vec::new(),
                )
            } else if let Some(greeting) = summer::get(&reference.key) {
                // Source literals open every admitted Summer NPC. Only canonical,
                // stateless single-line lore keywords reach this talk owner.
                let keywords = if npc.services.is_empty() {
                    npc.bank_dialogue
                        .as_ref()
                        .map(|dialogue| {
                            dialogue
                                .keywords
                                .iter()
                                .filter(|keyword| literal_keyword(keyword))
                                .cloned()
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                let topic_reply = if let Some(replies) = &npc.generated_replies {
                    let mut lines = Vec::new();
                    for (_, line) in &replies.services {
                        if !lines.contains(&line.text.as_str()) {
                            lines.push(line.text.as_str());
                        }
                    }
                    (!lines.is_empty()).then(|| lines.join(" "))
                } else if !travel_routes.is_empty() {
                    Some("Ask me for a passage to your destination.".into())
                } else if !premium_services.is_empty() {
                    Some("Ask me about promotion or a blessing.".into())
                } else {
                    None
                };
                (
                    greeting.greet.text.clone(),
                    greeting.farewell.text.clone(),
                    topic_reply,
                    keywords,
                )
            } else if let Some(greeting) = crate::quest_greetings::get(&reference.key)
                .filter(|_| reference.key != "oteryn:npc.spectulus")
            {
                // Only the closed source NPCs with native quest callers open here.
                // Their conditional topics and writes remain with dispatch.
                (
                    greeting.greet.text.clone(),
                    greeting.farewell.text.clone(),
                    None,
                    Vec::new(),
                )
            } else if let Some(greeting) = endgame_greeting(&reference.key) {
                // Only these source NPCs have native stateful Quest
                // consumers. This admits their greeting, not their Lua/script
                // branches; dispatch owns every confirmation and durable write.
                (greeting.into(), "Farewell.".into(), None, Vec::new())
            } else if let Some(messages) = reward_messages {
                // Stateful exchanges now have a real consumer in dispatch.
                // Open only their source-authored greeting; no scripted branch
                // or unconfirmed item/storage effect is admitted by this book.
                (
                    messages.greet.clone(),
                    messages.farewell.clone(),
                    None,
                    Vec::new(),
                )
            } else if let Some(replies) = &npc.generated_replies {
                let mut lines: Vec<&str> = Vec::new();
                for (_, line) in &replies.services {
                    if !lines.contains(&line.text.as_str()) {
                        lines.push(&line.text);
                    }
                }
                (
                    replies.greeting.text.clone(),
                    replies.farewell.text.clone(),
                    (!lines.is_empty()).then(|| lines.join(" ")),
                    Vec::new(),
                )
            } else if !travel_routes.is_empty() {
                (
                    format!("Greetings, I am {}.", npc.name),
                    "Farewell.".into(),
                    Some("Ask me for a passage to your destination.".into()),
                    Vec::new(),
                )
            } else if !premium_services.is_empty() {
                (
                    format!("Greetings, I am {}.", npc.name),
                    "Farewell.".into(),
                    Some("Ask me about promotion or a blessing.".into()),
                    Vec::new(),
                )
            } else if bank {
                // Only the Bank Dialogue's explicitly supported, authored message shape is
                // consumed. Missing/multipart/stateful dialogue stays held, never generated.
                let Some(dialogue) = &npc.bank_dialogue else {
                    continue;
                };
                if dialogue.greet.len() != 1
                    || dialogue.farewell.len() != 1
                    || dialogue
                        .keywords
                        .iter()
                        .any(|keyword| !literal_keyword(keyword))
                {
                    continue;
                }
                (
                    dialogue.greet[0].clone(),
                    dialogue.farewell[0].clone(),
                    None,
                    dialogue.keywords.clone(),
                )
            } else if let Some((greet, farewell)) = cosmetic_messages {
                (greet.into(), farewell.into(), None, Vec::new())
            } else if let Some((greet, farewell)) = &dark_messages {
                (greet.clone(), farewell.clone(), None, Vec::new())
            } else if let Some(messages) = &isle_messages {
                (
                    messages.greet.clone(),
                    messages.farewell.clone(),
                    None,
                    Vec::new(),
                )
            } else {
                continue;
            };
            book.npcs.insert(
                reference.key.clone(),
                NpcTalkEntry {
                    name: npc.name.clone(),
                    admitted_dialogue: npc.dialogue.is_some()
                        && premium_services.is_empty()
                        && travel_routes.is_empty(),
                    travel_routes,
                    premium_services,
                    bank: npc
                        .services
                        .iter()
                        .filter_map(|reference| model.services().get(reference))
                        .any(|service| service.bank),
                    greetings: GREETINGS.iter().map(|word| (*word).to_owned()).collect(),
                    greeting_reply,
                    farewell_reply,
                    source_name_templates: cosmetic_messages.is_some()
                        || dark_messages.is_some()
                        || isle_messages.is_some()
                        || crate::quest_greetings::is_furniture(&reference.key)
                        || crate::quest_greetings::is_seasonal(&reference.key)
                        || crate::quest_greetings::is_horse_rental(&reference.key)
                        || summer::get(&reference.key).is_some()
                        || reward_callback_npc(&reference.key),
                    source_walkaway: if dark_messages.is_some() {
                        Some(dark_trails::WALKAWAY.into())
                    } else if let Some(messages) = summer::get(&reference.key) {
                        Some(messages.walkaway.text.clone())
                    } else {
                        isle_messages
                            .map(|messages| messages.walkaway)
                            .or_else(|| reward_messages.map(|messages| messages.walkaway.clone()))
                    },
                    topic_reply,
                    dialogue_keywords,
                    offers: npc
                        .services
                        .iter()
                        .filter_map(|service| model.services().get(service))
                        .flat_map(|service| service.offers.iter().cloned())
                        .collect(),
                },
            );
        }
        book
    }

    #[doc(hidden)]
    pub fn with(mut self, key: &str, entry: NpcTalkEntry) -> Self {
        self.npcs.insert(key.to_owned(), entry);
        self
    }

    pub fn get(&self, key: &str) -> Option<&NpcTalkEntry> {
        self.npcs.get(key)
    }

    /// The catalogue's display name, even when this NPC has no qualified native dialogue.
    pub fn display_name(&self, key: &str) -> Option<&str> {
        self.npcs
            .get(key)
            .map(|entry| entry.name.as_str())
            .or_else(|| self.display_names.get(key).map(String::as_str))
    }

    pub fn catalogue_revision(&self) -> u64 {
        self.catalogue_revision.max(1)
    }
}

/// Source-qualified native finale speakers. A script-backed NPC outside this exact
/// set remains unavailable through the native conversation consumer.
fn endgame_greeting(key: &str) -> Option<&'static str> {
    Some(match key {
        // SRV6: exact final-report speaker; Crystal kesar_the_younger_day.lua:286.
        "oteryn:npc.kesar_the_younger_day" => {
            "Hail, traveller. What brings you here during these dire times under {siege}? The fair Bounac is suffering greatly."
        }
        // Retained current-Tibia source identifies this canonical NPC. Its final
        // sequence is native; unavailable intermediate story copy stays suppressed.
        "oteryn:npc.planestrider_npc" => "Planestrider",
        // data-global/npc/flickering_soul.lua:198.
        "oteryn:npc.flickering_soul" => "Be greeted, living soul!",
        // data-global/npc/torkada.lua:96.
        "oteryn:npc.torkada" => "Greetings! This isn't the {time} to chitchat though.",
        // data-global/npc/bloodshade_rotten.lua:148.
        "oteryn:npc.bloodshade_rotten" => {
            "Mortal! If you are on a {quest} to serve the blood god, my master - be greeted! I can also cleanse you from rotten {taints} that burden your soul."
        }
        // data-global/npc/bloodshade_sacrifice.lua:93.
        "oteryn:npc.bloodshade_sacrifice" => {
            "Mortal! If you are on a {quest} to serve the blood god, my master - be greeted!"
        }
        _ => return None,
    })
}

/// FNV-1a over the digest text; 1 in place of 0.
fn revision_of(digest: &str) -> u64 {
    let hash = digest
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    hash.max(1)
}

/// An NPC the speaker could talk to: where it stands and what it says.
pub struct TalkNpc<'a, K> {
    pub id: K,
    pub position: ChatPosition,
    pub entry: &'a NpcTalkEntry,
}

/// The NPC's reply to a `say`: the NPC that answers and its text.
pub struct NpcReply<K> {
    pub id: K,
    pub text: String,
}

/// Words that open the trade window (Canary modules.lua `FOCUS_TRADE_MESSAGE`).
const TRADE_WORDS: [&str; 2] = ["trade", "offers"];

/// Whether `text` asks the NPC to trade: the topic that opens the trade window.
pub fn asks_to_trade(text: &str) -> bool {
    says_any(&words(text), &TRADE_WORDS)
}

fn says_any(said: &[String], list: &[&str]) -> bool {
    list.iter()
        .any(|word| contains_words(said, &[(*word).to_owned()]))
}

/// Applies `text`, said at `speaker`, to the NPC conversation `engaged`.
///
/// A conversation ends when its NPC is gone or out of range. While it runs, a farewell ends it
/// and a topic is answered; otherwise a greeting within range starts one (the nearest NPC, then
/// the lowest index in `npcs`).
pub fn talk<K: Copy + PartialEq>(
    speaker: ChatPosition,
    text: &str,
    npcs: &[TalkNpc<'_, K>],
    engaged: &mut Option<K>,
) -> Option<NpcReply<K>> {
    let said = words(text);
    if let Some(current) = *engaged {
        let here = npcs.iter().find(|npc| {
            npc.id == current
                && speaker
                    .distance(npc.position)
                    .is_some_and(|d| d <= DEFAULT_TALK_RANGE)
        });
        let Some(npc) = here else {
            *engaged = None;
            return start(speaker, text, npcs, engaged);
        };
        if says_any(&said, &FAREWELLS) {
            *engaged = None;
            return Some(NpcReply {
                id: npc.id,
                text: npc.entry.farewell_reply.clone(),
            });
        }
        if let Some(keyword) = npc
            .entry
            .dialogue_keywords
            .iter()
            .find(|keyword| contains_words(&said, &keyword.triggers))
        {
            return Some(NpcReply {
                id: npc.id,
                text: keyword.reply[0].clone(),
            });
        }
        if says_any(&said, &TOPICS) && !npc.entry.admitted_dialogue {
            let line =
                npc.entry.topic_reply.clone().unwrap_or_else(|| {
                    format!("{} has nothing to offer right now.", npc.entry.name)
                });
            return Some(NpcReply {
                id: npc.id,
                text: line,
            });
        }
        // NPC-REGREET-1 (Canary npc_handler.lua `greet`): the NPC already talking to the speaker
        // does not greet it again; a greeting that reaches another NPC still starts there.
        return start(speaker, text, npcs, engaged).filter(|reply| reply.id != current);
    }
    start(speaker, text, npcs, engaged)
}

fn start<K: Copy + PartialEq>(
    speaker: ChatPosition,
    text: &str,
    npcs: &[TalkNpc<'_, K>],
    engaged: &mut Option<K>,
) -> Option<NpcReply<K>> {
    let greeting: Vec<GreetingNpc<'_>> = npcs
        .iter()
        .enumerate()
        .map(|(index, npc)| GreetingNpc {
            actor_id: index as u64,
            position: npc.position,
            greetings: &npc.entry.greetings,
            talk_range: DEFAULT_TALK_RANGE,
        })
        .collect();
    let index = usize::try_from(greeted_npc(speaker, text, &greeting)?).ok()?;
    let npc = npcs.get(index)?;
    *engaged = Some(npc.id);
    Some(NpcReply {
        id: npc.id,
        text: npc.entry.greeting_reply.clone(),
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures")]
mod tests {
    use super::*;

    #[test]
    fn qengine_reward_npcs_open_only_source_conversations() -> Result<(), String> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|e| format!("{e:?}"))?;
        let book = NpcTalkBook::from_model(&model);
        for name in ["morgan", "navigator", "costello", "budrik", "sam"] {
            let key = format!("oteryn:npc.{name}");
            let entry = book
                .get(&key)
                .ok_or_else(|| format!("missing reward conversation {key}"))?;
            assert!(!entry.greeting_reply.is_empty());
            assert!(
                entry.dialogue_keywords.is_empty(),
                "no scripted reward branch opens through talk rules"
            );
        }
        assert!(!reward_callback_npc("oteryn:npc.unqualified"));
        Ok(())
    }

    #[test]
    fn monk_pilgrimage_npcs_have_native_conversation_entries() -> Result<(), String> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|e| format!("{e:?}"))?;
        let book = NpcTalkBook::from_model(&model);
        for key in ["oteryn:npc.ambassador_manop", "oteryn:npc.enpa_deia_pema"] {
            let entry = book.get(key).ok_or(key)?;
            assert!(entry.greeting_reply.contains("pilgrimage"));
            assert!(!entry.admitted_dialogue);
        }
        Ok(())
    }

    #[test]
    fn endgame_npcs_have_only_their_source_qualified_native_conversation_entries()
    -> Result<(), String> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?;
        let book = NpcTalkBook::from_model(&model);
        for key in [
            "oteryn:npc.flickering_soul",
            "oteryn:npc.torkada",
            "oteryn:npc.bloodshade_rotten",
            "oteryn:npc.bloodshade_sacrifice",
        ] {
            let entry = book
                .get(key)
                .ok_or_else(|| format!("missing native conversation: {key}"))?;
            assert!(!entry.bank());
            assert!(entry.offers.is_empty());
            assert!(
                entry.dialogue_keywords.is_empty(),
                "stateful script branches belong to the Quest caller"
            );
            let npcs = [TalkNpc {
                id: 1_u8,
                position: at(101),
                entry,
            }];
            let mut engaged = None;
            assert!(talk(at(100), "hi", &npcs, &mut engaged).is_some());
            assert_eq!(engaged, Some(1));
            assert!(
                talk(at(100), "yes", &npcs, &mut engaged).is_none(),
                "greeting admission alone cannot confirm or grant a Quest action"
            );
            assert_eq!(engaged, Some(1));
        }
        for key in [
            "oteryn:npc.flickering_soul_fake",
            "oteryn:npc.bloodshade",
            "oteryn:npc.eremo",
            "",
        ] {
            assert!(
                endgame_greeting(key).is_none(),
                "the exception is an exact closed set"
            );
        }
        Ok(())
    }

    fn entry(name: &str, topic: Option<&str>) -> NpcTalkEntry {
        NpcTalkEntry::new(name, topic)
    }

    fn at(x: i32) -> ChatPosition {
        ChatPosition {
            x,
            y: 100,
            floor: 7,
        }
    }

    fn say(text: &str) -> String {
        text.to_owned()
    }

    #[test]
    fn a_conversation_runs_greeting_topic_farewell() {
        let alice = entry("Alice", Some("Alice buys and sells goods."));
        let npcs = [TalkNpc {
            id: 1_u8,
            position: at(102),
            entry: &alice,
        }];
        let mut engaged = None;
        // A topic before a greeting is not answered.
        assert!(talk(at(100), &say("trade"), &npcs, &mut engaged).is_none());
        let reply = talk(at(100), &say("Hello!"), &npcs, &mut engaged).expect("greeted");
        assert_eq!(reply.text, "Greetings, I am Alice.");
        assert_eq!(engaged, Some(1));
        let reply = talk(at(100), &say("what is your job"), &npcs, &mut engaged).expect("topic");
        assert_eq!(reply.text, "Alice buys and sells goods.");
        let reply = talk(at(100), &say("bye"), &npcs, &mut engaged).expect("farewell");
        assert_eq!(reply.text, "Farewell.");
        assert_eq!(engaged, None);
        assert!(talk(at(100), &say("trade"), &npcs, &mut engaged).is_none());
    }

    #[test]
    fn walking_out_of_range_ends_the_conversation() {
        let alice = entry("Alice", None);
        let npcs = [TalkNpc {
            id: 1_u8,
            position: at(102),
            entry: &alice,
        }];
        let mut engaged = Some(1);
        assert!(talk(at(110), &say("trade"), &npcs, &mut engaged).is_none());
        assert_eq!(engaged, None);
        assert!(talk(at(110), &say("hi"), &npcs, &mut engaged).is_none());
    }

    #[test]
    fn an_npc_without_a_service_says_so() {
        let bob = entry("Bob", None);
        let npcs = [TalkNpc {
            id: 2_u8,
            position: at(101),
            entry: &bob,
        }];
        let mut engaged = None;
        talk(at(100), &say("hi"), &npcs, &mut engaged).expect("greeted");
        let reply = talk(at(100), &say("offer"), &npcs, &mut engaged).expect("topic");
        assert_eq!(reply.text, "Bob has nothing to offer right now.");
    }

    #[test]
    fn a_greeting_while_engaged_is_not_answered_again() {
        let (alice, bob) = (entry("Alice", None), entry("Bob", None));
        let alice_only = [TalkNpc {
            id: 1_u8,
            position: at(101),
            entry: &alice,
        }];
        let mut engaged = None;
        talk(at(100), &say("hi"), &alice_only, &mut engaged).expect("greeted");
        assert!(talk(at(100), &say("hello"), &alice_only, &mut engaged).is_none());
        assert_eq!(engaged, Some(1));
        // A greeting that reaches another NPC still moves the conversation there.
        let both = [
            TalkNpc {
                id: 1_u8,
                position: at(104),
                entry: &alice,
            },
            TalkNpc {
                id: 2_u8,
                position: at(101),
                entry: &bob,
            },
        ];
        let reply = talk(at(100), &say("hi"), &both, &mut engaged).expect("greeted");
        assert_eq!((reply.id, engaged), (2, Some(2)));
    }

    #[test]
    fn trade_and_offers_ask_to_trade() {
        assert!(asks_to_trade("trade"));
        assert!(asks_to_trade("what are your offers?"));
        assert!(!asks_to_trade("offer"));
        assert!(!asks_to_trade("hi"));
    }

    #[test]
    fn the_nearest_greeted_npc_answers() {
        let (far, near) = (entry("Far", None), entry("Near", None));
        let npcs = [
            TalkNpc {
                id: 1_u8,
                position: at(103),
                entry: &far,
            },
            TalkNpc {
                id: 2_u8,
                position: at(101),
                entry: &near,
            },
        ];
        let mut engaged = None;
        let reply = talk(at(100), &say("hi"), &npcs, &mut engaged).expect("greeted");
        assert_eq!((reply.id, engaged), (2, Some(2)));
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod bank_dialogue_tests {
    use super::*;
    #[test]
    fn admitted_bank_dialogue_never_generates_an_unmatched_topic_answer() {
        let mut entry = NpcTalkEntry::new("Paulie", None);
        entry.bank = true;
        entry.admitted_dialogue = true;
        entry.greeting_reply = "Welcome to the bank.".into();
        entry.farewell_reply = "Farewell from the admitted Dialogue.".into();
        let position = ChatPosition {
            x: 1,
            y: 1,
            floor: 0,
        };
        let npcs = [TalkNpc {
            id: 1_u8,
            position,
            entry: &entry,
        }];
        let mut engaged = None;
        let greet = "hi";
        assert_eq!(
            talk(position, greet, &npcs, &mut engaged).map(|r| r.text),
            Some("Welcome to the bank.".into())
        );
        for topic in ["job", "offer", "trade", "travel"] {
            assert!(talk(position, topic, &npcs, &mut engaged).is_none());
            assert_eq!(engaged, Some(1));
        }
        let farewell = talk(position, "bye", &npcs, &mut engaged);
        assert_eq!(
            farewell.map(|r| r.text),
            Some("Farewell from the admitted Dialogue.".into())
        );
        assert_eq!(engaged, None);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "repository catalogue qualification")]
mod summer_dialogue_tests {
    use super::*;

    #[test]
    fn summer_population_lore_book_retains_only_literal_admitted_topics() {
        let content =
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content"));
        let catalogue = crate::content::NpcDataCatalogue::load(
            content,
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .expect("catalogue");
        let model = catalogue.service_model_for_project().expect("model");
        let book = NpcTalkBook::from_model(&model);
        for key in ["oteryn:npc.mayor_pocaro", "oteryn:npc.wayland_smythers"] {
            let npc = book.get(key).expect("admitted Summer lore");
            assert!(npc.admitted_dialogue);
            assert!(npc.offers.is_empty());
            assert!(npc.dialogue_keywords.iter().all(literal_keyword));
            let source = model
                .npcs()
                .iter()
                .find(|(reference, _)| reference.key == key)
                .and_then(|(_, npc)| npc.bank_dialogue.as_ref())
                .expect("authored Dialogue");
            let held = source
                .keywords
                .iter()
                .filter(|keyword| !literal_keyword(keyword))
                .collect::<Vec<_>>();
            assert!(
                !held.is_empty(),
                "the source has multipart branches outside this slice"
            );
            for keyword in held {
                assert!(
                    !npc.dialogue_keywords
                        .iter()
                        .any(|admitted| admitted.key == keyword.key)
                );
            }
        }
        // NPC-TRAVEL-1 already exposes Corsarah's passage service. That talk entry
        // must not admit her authored Dialogue through the Mayor's lore consumer.
        let corsarah = book
            .get("oteryn:npc.captain_corsarah")
            .expect("existing travel service");
        assert!(!corsarah.admitted_dialogue);
        assert!(corsarah.dialogue_keywords.is_empty());
        assert!(corsarah.offers.is_empty());
        assert_eq!(
            corsarah.topic_reply.as_deref(),
            Some("Ask me for a passage to your destination.")
        );
        assert_eq!(
            corsarah
                .travel_routes
                .iter()
                .map(|route| route.keyword.as_str())
                .collect::<Vec<_>>(),
            ["venore"]
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "repository Grizzly dialogue qualification"
)]
mod grizzly_dialogue_tests {
    use super::*;

    #[test]
    fn grizzly_native_task_npc_is_reachable_from_the_real_dialogue_catalogue() {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .expect("catalogue");
        let model = catalogue.service_model_for_project().expect("model");
        let book = NpcTalkBook::from_model(&model);
        let entry = book
            .get("oteryn:npc.grizzly_adams")
            .expect("native Grizzly dialogue");
        assert_eq!(
            entry.offers().len(),
            56,
            "all sourced offers pass native Item admission"
        );
        for (points, count) in [(9, 0), (10, 40), (20, 41), (40, 46), (70, 56)] {
            assert_eq!(
                entry
                    .offers()
                    .iter()
                    .filter(
                        |offer| crate::content::npc_catalogue::grizzly_shop::visible(
                            "oteryn:npc.grizzly_adams",
                            offer,
                            Some(points)
                        )
                    )
                    .count(),
                count
            );
        }
        let position = ChatPosition {
            x: 1,
            y: 1,
            floor: 0,
        };
        let npcs = [TalkNpc {
            id: 1_u8,
            position,
            entry,
        }];
        let mut engaged = None;
        let greeting = talk(position, "hi", &npcs, &mut engaged).expect("greeting");
        assert!(greeting.text.contains("Paw and Fur"));
        assert_eq!(engaged, Some(1));
        assert!(
            entry
                .topic_reply
                .as_deref()
                .is_some_and(|text| text.contains("tasks"))
        );
    }
    #[test]
    fn liquid_black_source_npcs_open_without_granting_access() -> Result<(), String> {
        let at = |x| ChatPosition { x, y: 1, floor: 0 };
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?;
        let book = NpcTalkBook::from_model(&model);
        let key = "oteryn:npc.scrutinon";
        let entry = book.get(key).ok_or(key)?;
        let npcs = [TalkNpc {
            id: 1_u8,
            position: at(101),
            entry,
        }];
        let mut engaged = None;
        let greeting = talk(at(100), "hi", &npcs, &mut engaged).ok_or("greeting")?;
        assert!(greeting.text.contains("watching your fate"));
        assert_eq!(engaged, Some(1));
        assert!(talk(at(100), "yes", &npcs, &mut engaged).is_none());
        // Spectulus's greeting reads Sea of Light, outside batch2; coordinator
        // https://github.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-6589521017 §6.
        assert!(book.get("oteryn:npc.spectulus").is_none());
        Ok(())
    }
}

#[cfg(test)]
#[path = "npc_talk/entity_names_tests.rs"]
mod entity_names_tests;
#[cfg(test)]
mod dark_trails_admission_tests {
    #[test]
    fn dark_trails_ezebeth_opens_the_real_conversation() -> Result<(), String> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|e| format!("{e:?}"))?;
        let book = super::NpcTalkBook::from_model(&model);
        let entry = book
            .get("oteryn:npc.ezebeth")
            .ok_or("Ezebeth conversation is not admitted")?;
        assert_eq!(entry.farewell(), "Good bye, |PLAYERNAME|.");
        for key in [
            "a_beggar",
            "barazbaz",
            "elliott",
            "jacob",
            "quandons_ghost",
            "roswitha",
            "sholley",
            "terrence",
        ] {
            assert!(
                book.get(&format!("oteryn:npc.{key}")).is_some(),
                "missing {key}"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "npc_talk/furniture_tests.rs"]
mod furniture_tests;
