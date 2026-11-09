//! NPC talk over the chat path (NPC-TALK-1, CHAT-0 §3, NPC-0 §3.3).
//!
//! A `say` near a runtime NPC is answered by that NPC from catalogue data: a greeting starts a
//! conversation, a topic keyword answers while the speaker stays in range, a farewell ends it.
//! The reply is an ordinary local line spoken by the NPC, so it reaches every session that hears
//! the NPC. Trade and bank are not here (NPC-TRADE-1, BANK-NPC-1). Nothing is durable.

use std::collections::HashMap;

mod dark_trails;
mod isle_of_evil;

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

const REWARD_CALLBACKS: &[&str] = &[
    include_str!("../../../../content/quests/npc-callbacks/qnpcfx-koshei.json"),
    include_str!("../../../../content/quests/npc-callbacks/qnpcfx-m10.json"),
    include_str!("../../../../content/quests/npc-callbacks/m39-stories.json"),
    include_str!("../../../../content/quests/shared-reward-callbacks.json"),
    include_str!("../../../../content/quests/npc-callbacks/l6-stories.json"),
    include_str!("../../../../content/quests/npc-callbacks/m16-targuna-marker.json"),
    include_str!("../../../../content/quests/npc-callbacks/m13-goblin-capture.json"),
];
const REWARD_GREETINGS: &[&str] = &[
    include_str!("../../../../content/dialogues/quests/qnpcfx-m10-greetings.json"),
    include_str!("../../../../content/dialogues/quests/shared-reward-greetings.json"),
    include_str!("../../../../content/dialogues/quests/l6-story-greetings.json"),
    include_str!("../../../../content/dialogues/quests/questpc5-targuna-map-greetings.json"),
    include_str!("../../../../content/dialogues/quests/m13-goblin-greetings.json"),
];

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
        let mut rows = Vec::new();
        let mut keys = std::collections::BTreeSet::new();
        for source in REWARD_GREETINGS {
            let document = serde_json::from_str::<Document>(source).ok()?;
            if document.schema != 1 {
                return None;
            }
            for row in document.npcs {
                if !keys.insert(row.npc.clone()) {
                    return None;
                }
                rows.push(row);
            }
        }
        (rows.len() <= 256).then_some(rows)
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
        let parse = || -> Option<std::collections::BTreeSet<String>> {
            let mut count = 0_usize;
            let mut keys = std::collections::BTreeSet::new();
            for source in REWARD_CALLBACKS {
                let document = serde_json::from_str::<Document>(source).ok()?;
                if document.schema != 1 {
                    return None;
                }
                count = count.checked_add(document.callbacks.len())?;
                if count > 256 {
                    return None;
                }
                keys.extend(document.callbacks.into_iter().map(|c| c.npc));
            }
            Some(keys)
        };
        parse().unwrap_or_default()
    })
    .contains(key)
}

// Closed World-change native NPC consumers, never a general Dialogue fallback.
// Crystal 9f5a72c: benevola.lua cdc852e9..:176-180; orockle.lua
// d2c5d365..:77-83 (six canonical offers, no greeting override).
// Shared default handler 8479e2aa..:81-82,92 supplies Orockle's text.
const YEO_GREET: &str = "Ah, it's you. Nice to see you.";
const BENE_GREET: &str = "Greetings. I hope you are here on a {mission} for nature!";
const OROCKLE: &str = "oteryn:npc.orockle";
const DEEP_SEAS_GUARDIAN: &str = "oteryn:npc.guardian_of_the_deep_seas";
const OROCKLE_FAREWELL: &str = "Good bye, |PLAYERNAME|.";
const OROCKLE_TRADE: &str = "Of course, just browse through my wares.";
const SHARED_WORLD_NPC_SOURCE: &str = "cdc852e9469d0afe12f4f15294f4f22659454359b5da667358b4e5a4c3115422:176-180;d2c5d365f1478af39aefe90776226f016ddd6f2fb595c94d38e0a557ae148b8d:77-83;75ad4f7af508e8ab40004b3413ff831e331072c605d1d2ba29dc303dc29dfdc5:26-60;8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a:81-82,92;wiki:69583@1149608:4e4593cc7f581654f984a86a66309f349dfb23f19a38adf9856c625a8522c0eb:158;yeo-retained-canonical-greeting-v1";

const YASIR: &str = "oteryn:npc.yasir";
// Retained Crystal 9f5a72c: npc/yasir.lua d189aab0.. L812-833 and
// npc_system/npc_handler.lua 8479e2aa.. L79-81 (unoverridden default greet).
// The ariki handler opens the shop silently; durable successful trade already owns Si, Ariki!.
const YASIR_GREET: &str = "Greetings, |PLAYERNAME|.";
const YASIR_FAREWELL: &str = "Si, jema ze harun. <waves>";
const YASIR_JOB: &str = "Tje hari ku ne finjala. {Ariki}?";
const YASIR_NAME: &str = "Me Yasir.";
const YASIR_PASSAGE: &str = "Soso yana. <shakes his head>";
const YASIR_SOURCE: &str = "d189aab0cdedb7f00bec2977c92e116240a9ca029569f537d9b24afcefb3f8a3:812-833;8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a:79-81";

fn yasir_keywords() -> Vec<crate::content::ProjectV2DialogueKeyword> {
    [
        ("name", YASIR_NAME),
        ("job", YASIR_JOB),
        ("passage", YASIR_PASSAGE),
    ]
    .into_iter()
    .map(|(key, reply)| crate::content::ProjectV2DialogueKeyword {
        key: key.into(),
        triggers: vec![key.into()],
        reply: vec![reply.into()],
        fallback: false,
        links: Vec::new(),
        only_focus: true,
        only_unfocus: false,
        reset: false,
        ungreet: false,
        move_up: None,
        children: Vec::new(),
    })
    .collect()
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

    /// Source greeting for an explicit conversation test fixture.
    #[doc(hidden)]
    pub fn with_source_greeting(mut self, greeting: &str) -> Self {
        self.greeting_reply = greeting.into();
        self.source_name_templates = true;
        self
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
    source_walks: Option<crate::npc_actor::NpcWalkBook>,
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
        let walks = crate::npc_actor::NpcWalkBook::from_catalogue(catalogue, model)
            .map_err(|_| crate::npc_voice::VoiceError::ContentSourceMismatch)?;
        self.source_voices = crate::npc_voice::profiles_from_catalogue(catalogue, model, qualify)?;
        self.source_walks = Some(walks);
        Ok(())
    }

    pub fn source_walks(&self) -> Option<&crate::npc_actor::NpcWalkBook> {
        self.source_walks.as_ref()
    }

    pub fn source_voices(
        &self,
    ) -> &std::collections::BTreeMap<String, crate::npc_voice::VoiceProfile> {
        &self.source_voices
    }

    /// Entries for every NPC with generated replies. An NPC with an admitted Dialogue is left out
    /// until a dialogue packet reads it.
    pub fn from_model(model: &NpcServiceModel) -> Self {
        Self::from_model_with_native_quests(model, &[])
    }

    /// Admit source greetings for NPCs whose stateful rules have a native executor.
    pub fn from_model_with_native_quests(model: &NpcServiceModel, native_npcs: &[&str]) -> Self {
        let mut book = Self {
            catalogue_revision: revision_of(&format!(
                "npc-trade-3:v1|npc-marriage-1\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
                crate::content::npc_catalogue::grizzly_shop::book_digest(
                    model.source_tree_digest()
                ),
                format_args!(
                    "{}\0{}\0{}\0{}\0{}",
                    dark_trails::TEXT,
                    format_args!("{}\0{}", SHARED_WORLD_NPC_SOURCE, YEO_GREET),
                    BENE_GREET,
                    OROCKLE_FAREWELL,
                    OROCKLE_TRADE
                ),
                isle_of_evil::revision_material(),
                REWARD_CALLBACKS.join("\0"),
                REWARD_GREETINGS.join("\0"),
                crate::content::npc_catalogue::healing::DATA,
                crate::content::npc_catalogue::travel::revision_material(),
                YASIR_SOURCE,
                YASIR_GREET,
                YASIR_FAREWELL,
                YASIR_JOB,
                YASIR_NAME,
                YASIR_PASSAGE,
            )),
            ..Self::default()
        };
        for (reference, npc) in model.npcs() {
            book.display_names
                .insert(reference.key.clone(), npc.name.clone());
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
            } else if reference.key == "oteryn:npc.lynda" {
                // Crystal201189f lynda.lua:307-309; only the native marriage caller opens.
                (
                    "Welcome in the name of the gods, pilgrim |PLAYERNAME|!".into(),
                    "Be careful on your journeys.".into(),
                    Some(
                        "Ask me about marriage, celebration, removing a proposal, or divorce."
                            .into(),
                    ),
                    Vec::new(),
                )
            } else if reference.key == DEEP_SEAS_GUARDIAN {
                // Retained Crystal Guardian is an NPC stub with FocusModule and no overrides
                // or shop. The same pinned default handler supplies greeting and farewell;
                // the native quest adapter owns its authored stateful mission conversation.
                (
                    YASIR_GREET.into(),
                    OROCKLE_FAREWELL.into(),
                    None,
                    Vec::new(),
                )
            } else if reference.key == "oteryn:npc.yeo" {
                // Retained canonical Yeo dialogue supplies these greeting/farewell lines.
                // Wiki supplies the ten-shrimp barter; native dispatch owns confirmation.
                (YEO_GREET.into(), "Mhm.".into(), None, Vec::new())
            } else if reference.key == "oteryn:npc.benevola" {
                (BENE_GREET.into(), "Good bye.".into(), None, Vec::new())
            } else if reference.key == OROCKLE {
                // Canonical catalogue offers remain the sole shop source; no mission mint.
                (
                    YASIR_GREET.into(),
                    OROCKLE_FAREWELL.into(),
                    Some(OROCKLE_TRADE.into()),
                    Vec::new(),
                )
            } else if reference.key == YASIR {
                // Closed native consumer: no other Dialogue-backed merchant is admitted.
                (
                    YASIR_GREET.into(),
                    YASIR_FAREWELL.into(),
                    Some(YASIR_JOB.into()),
                    yasir_keywords(),
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
            } else if matches!(
                reference.key.as_str(),
                "oteryn:npc.mayor_pocaro" | "oteryn:npc.wayland_smythers"
            ) && npc.services.is_empty()
            {
                // SUMMER: the model's historical `bank_dialogue` carrier holds the
                // admitted messages for every Dialogue-backed NPC. Only these Summer NPCs'
                // literal, single-line lore is qualified here. Quest callbacks and
                // multipart/stateful branches remain outside the talk consumer.
                let Some(dialogue) = &npc.bank_dialogue else {
                    continue;
                };
                if dialogue.greet.len() != 1 || dialogue.farewell.len() != 1 {
                    continue;
                }
                (
                    dialogue.greet[0].clone(),
                    dialogue.farewell[0].clone(),
                    None,
                    dialogue
                        .keywords
                        .iter()
                        .filter(|keyword| literal_keyword(keyword))
                        .cloned()
                        .collect(),
                )
            } else if crate::content::npc_catalogue::healing::profile(&reference.key).is_some() {
                (
                    format!(
                        "Greetings, I am {}. Ask me for healing if you are injured.",
                        npc.name
                    ),
                    "Farewell.".into(),
                    None,
                    Vec::new(),
                )
            } else if let Some((greet, farewell)) = &dark_messages {
                (greet.clone(), farewell.clone(), None, Vec::new())
            } else if let Some(messages) = &isle_messages {
                (
                    messages.greet.clone(),
                    messages.farewell.clone(),
                    None,
                    Vec::new(),
                )
            } else if native_npcs.contains(&reference.key.as_str()) && npc.dialogue.is_some() {
                // Conditional native greeting rules own the text. An absent literal
                // greeting still admits focus so that their reply or dismissal runs.
                let greeting = npc
                    .bank_dialogue
                    .as_ref()
                    .and_then(|dialogue| dialogue.greet.first())
                    .cloned()
                    .unwrap_or_default();
                let farewell = npc
                    .bank_dialogue
                    .as_ref()
                    .and_then(|dialogue| dialogue.farewell.first())
                    .cloned()
                    .unwrap_or_else(|| "Farewell.".into());
                (greeting, farewell, None, Vec::new())
            } else {
                continue;
            };
            book.npcs.insert(
                reference.key.clone(),
                NpcTalkEntry {
                    name: npc.name.clone(),
                    admitted_dialogue: !matches!(reference.key.as_str(), YASIR | OROCKLE)
                        && npc.dialogue.is_some()
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
                    source_name_templates: bank
                        || reference.key == "oteryn:npc.lynda"
                        || matches!(reference.key.as_str(), YASIR | OROCKLE | DEEP_SEAS_GUARDIAN)
                        || dark_messages.is_some()
                        || isle_messages.is_some()
                        || reward_callback_npc(&reference.key)
                        || native_npcs.contains(&reference.key.as_str()),
                    source_walkaway: if dark_messages.is_some() {
                        Some(dark_trails::WALKAWAY.into())
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

/// Source-specific shop keyword; ordinary merchants retain the standard trade words.
pub fn asks_to_trade_for(npc_key: &str, text: &str) -> bool {
    asks_to_trade(text) || (npc_key == YASIR && says_any(&words(text), &["ariki"]))
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
    fn goblin_capture_intro_has_source_rafzan_greeting() -> Result<(), String> {
        assert!(reward_callback_npc("oteryn:npc.rafzan"));
        let greeting = reward_messages("oteryn:npc.rafzan").ok_or("Rafzan greeting")?;
        assert_eq!(greeting.greet, "Greetings, |PLAYERNAME|.");
        Ok(())
    }

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
        for name in [
            "morgan",
            "navigator",
            "costello",
            "budrik",
            "sam",
            "oldrak",
            "zoltan",
            "rafzan",
        ] {
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
        assert!(
            book.get("oteryn:npc.zoltan")
                .ok_or("Zoltan")?
                .greeting_reply
                .contains("student of the arcane arts")
        );
        assert!(!reward_callback_npc("oteryn:npc.unqualified"));
        Ok(())
    }

    #[test]
    fn worldchg_shared_native_benevola_and_orockle_shop_are_closed() -> Result<(), String> {
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
        let guardian = book
            .get(DEEP_SEAS_GUARDIAN)
            .ok_or("Deep Seas Guardian native greeting")?;
        assert_eq!(
            guardian.render_source_text(&guardian.greeting_reply, "Helper"),
            "Greetings, Helper."
        );
        assert_eq!(
            guardian.render_source_text(guardian.farewell(), "Helper"),
            "Good bye, Helper."
        );
        assert!(guardian.offers().is_empty());
        assert!(!guardian.bank());
        assert!(guardian.travel_routes.is_empty());
        assert!(guardian.premium_services.is_empty());
        let yeo = book
            .get("oteryn:npc.yeo")
            .ok_or("Yeo native barter greeting")?;
        assert_eq!(yeo.greeting_reply, YEO_GREET);
        assert!(
            yeo.offers().is_empty(),
            "barter cannot enter a free generic shop window"
        );
        let benevola = book
            .get("oteryn:npc.benevola")
            .ok_or("Benevola native greeting")?;
        assert_eq!(benevola.greeting_reply, BENE_GREET);
        assert_eq!(benevola.farewell(), "Good bye.");
        assert!(benevola.offers().is_empty());
        let orockle = book.get(OROCKLE).ok_or("Orockle native shop")?;
        assert!(!orockle.admitted_dialogue);
        assert_eq!(
            orockle.render_source_text(&orockle.greeting_reply, "Helper"),
            "Greetings, Helper."
        );
        assert_eq!(
            orockle.render_source_text(orockle.farewell(), "Helper"),
            "Good bye, Helper."
        );
        assert_eq!(orockle.topic_reply.as_deref(), Some(OROCKLE_TRADE));
        let offers: std::collections::BTreeMap<_, _> = orockle
            .offers()
            .iter()
            .map(|offer| {
                assert_eq!(
                    offer.direction,
                    crate::content::ProjectV2ServiceOfferDirection::SellToPlayer
                );
                (offer.item.key.as_str(), offer.unit_price)
            })
            .collect();
        assert_eq!(orockle.offers().len(), 6);
        assert_eq!(
            offers,
            std::collections::BTreeMap::from([
                ("oteryn:item.tibia.i13974", 150),
                ("oteryn:item.tibia.i13976", 150),
                ("oteryn:item.tibia.i14051", 150),
                ("oteryn:item.tibia.i14052", 150),
                ("oteryn:item.tibia.i14054", 200),
                ("oteryn:item.tibia.i14255", 200),
            ])
        );
        // Sam is independently admitted by QENGINE2's bounded reward callbacks.
        // Alesar remains an unqualified Dialogue-backed merchant.
        assert!(reward_callback_npc("oteryn:npc.sam"));
        let sam = book
            .get("oteryn:npc.sam")
            .ok_or("qualified Sam reward NPC")?;
        assert!(sam.dialogue_keywords.is_empty());
        assert!(
            model
                .npcs()
                .iter()
                .any(|(key, npc)| { key.key == "oteryn:npc.alesar" && npc.dialogue.is_some() })
        );
        assert!(book.get("oteryn:npc.alesar").is_none());
        assert!(!asks_to_trade_for(OROCKLE, "ariki"));
        Ok(())
    }

    #[test]
    fn worldchg_trader_yasir_closed_native_book_and_keywords() -> Result<(), String> {
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
        let entry = book.get(YASIR).ok_or("native Yasir")?;
        assert!(!entry.admitted_dialogue);
        assert_eq!(entry.offers().len(), 757);
        let (_, source) = model
            .npcs()
            .iter()
            .find(|(key, _)| key.key == YASIR)
            .ok_or("source Yasir")?;
        let offers: Vec<_> = source
            .services
            .iter()
            .filter_map(|key| model.services().get(key))
            .flat_map(|service| service.offers.iter().cloned())
            .collect();
        assert_eq!(entry.offers(), offers.as_slice());
        // The closed branch must not admit another Dialogue-backed merchant incidentally.
        // Sam is independently admitted by QENGINE2's bounded reward callbacks.
        // Alesar remains an unqualified Dialogue-backed merchant.
        assert!(reward_callback_npc("oteryn:npc.sam"));
        let sam = book
            .get("oteryn:npc.sam")
            .ok_or("qualified Sam reward NPC")?;
        assert!(sam.dialogue_keywords.is_empty());
        assert!(
            model
                .npcs()
                .iter()
                .any(|(key, npc)| { key.key == "oteryn:npc.alesar" && npc.dialogue.is_some() })
        );
        assert!(book.get("oteryn:npc.alesar").is_none());
        assert_eq!(
            entry.render_source_text(&entry.greeting_reply, "Hero"),
            "Greetings, Hero."
        );
        assert_eq!(entry.farewell(), YASIR_FAREWELL);
        let position = ChatPosition {
            x: 0,
            y: 0,
            floor: 0,
        };
        let npcs = [TalkNpc {
            id: 1u8,
            position,
            entry,
        }];
        let mut engaged = None;
        assert_eq!(
            talk(position, "hi", &npcs, &mut engaged).map(|r| r.text),
            Some(YASIR_GREET.into())
        );
        for (text, expected) in [
            ("name", YASIR_NAME),
            ("job", YASIR_JOB),
            ("passage", YASIR_PASSAGE),
            ("trade", YASIR_JOB),
        ] {
            assert_eq!(
                talk(position, text, &npcs, &mut engaged).map(|r| r.text),
                Some(expected.into())
            );
        }
        // Source ariki opens the window without a speech line or conversation reset.
        assert!(talk(position, "ariki", &npcs, &mut engaged).is_none());
        assert_eq!(engaged, Some(1));
        assert!(asks_to_trade_for(YASIR, "Ariki?"));
        assert!(!asks_to_trade_for("oteryn:npc.sam", "ariki"));
        assert!(!asks_to_trade("ariki"));
        assert!(asks_to_trade_for("oteryn:npc.sam", "trade"));
        assert!(!asks_to_trade_for(YASIR, "marikina"));
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
mod s10_tests {
    #[test]
    fn s10_healers_reach_production_talk_book() -> Result<(), String> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?;
        let book = super::NpcTalkBook::from_model(&model);
        for key in [
            "isimov",
            "rahkem",
            "rock_with_a_soft_spot",
            "tibra",
            "tyrias",
            "cipfried",
            "dallheim",
            "zerbrus",
            "hector_the_mentor",
        ] {
            let key = format!("oteryn:npc.{key}");
            assert!(book.get(&key).is_some(), "healer missing: {key}");
        }
        Ok(())
    }
    #[test]
    fn s10_nonpremium_blessing_givers_reach_production_talk_book() -> Result<(), String> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?;
        let book = super::NpcTalkBook::from_model(&model);
        for stem in ["norf", "humphrey", "edala", "quentin", "kawill", "pydar"] {
            let key = format!("oteryn:npc.{stem}");
            let entry = book
                .get(&key)
                .ok_or_else(|| format!("unavailable blessing NPC {key}"))?;
            assert!(
                crate::npc_promotion::select("blessing", &entry.premium_services).is_some(),
                "{key}"
            );
        }
        for stem in ["ishebad", "enpa_rudra", "emperor_rehal"] {
            let key = format!("oteryn:npc.{stem}");
            let entry = book
                .get(&key)
                .ok_or_else(|| format!("unavailable promotion NPC {key}"))?;
            assert!(
                crate::npc_promotion::select("promotion", &entry.premium_services).is_some(),
                "{key}"
            );
        }
        Ok(())
    }

    #[test]
    fn s10_ungated_source_travel_reaches_the_production_talk_book() -> Result<(), String> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?;
        let book = super::NpcTalkBook::from_model(&model);
        for (stem, keyword, price, x, y, floor) in [
            ("scrutinon", "ab'dendriel", 0, 32734, 31668, 6),
            ("scrutinon", "edron", 0, 33175, 31764, 6),
            ("scrutinon", "venore", 0, 32954, 32022, 6),
            ("scrutinon", "darashia", 0, 33289, 32480, 6),
            ("rapanaio_isle_of_evil", "kazordoon", 0, 32700, 31990, 15),
            ("a_ghostly_sage", "help", 0, 32834, 32275, 9),
        ] {
            let key = format!("oteryn:npc.{stem}");
            let entry = book
                .get(&key)
                .ok_or_else(|| format!("unavailable travel NPC {key}"))?;
            let route = entry
                .travel_routes
                .iter()
                .find(|route| route.keyword == keyword)
                .ok_or_else(|| format!("missing {key}/{keyword}"))?;
            assert!(!route.held, "{key}/{keyword}");
            assert!(!route.route.premium);
            assert_eq!(route.route.price, price);
            assert_eq!(
                (
                    route.route.destination.x,
                    route.route.destination.y,
                    route.route.destination.floor
                ),
                (x, y, floor)
            );
        }
        for key in ["oteryn:npc.rapanaio_boat", "oteryn:npc.dalbrect"] {
            assert!(
                book.get(key)
                    .is_none_or(|entry| entry.travel_routes.is_empty()),
                "quest-only route escaped admission: {key}"
            );
        }
        Ok(())
    }

    #[test]
    fn s10_source_registered_bankers_reach_the_production_talk_book() -> Result<(), String> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?;
        let book = super::NpcTalkBook::from_model(&model);
        for stem in [
            "atur",
            "ebenizer",
            "eighty",
            "elgar",
            "eva",
            "ferks",
            "finarfin",
            "gnomillion",
            "jefrey",
            "jessica",
            "kaya",
            "kepar",
            "lokur",
            "murim",
            "muzir",
            "naji",
            "raffael",
            "rokyn",
            "siestaar",
            "sissek",
            "suzy",
            "tesha",
            "virgil",
            "znozel",
            "wentworth",
        ] {
            let key = format!("oteryn:npc.{stem}");
            let entry = book
                .get(&key)
                .ok_or_else(|| format!("unavailable banker {key}"))?;
            assert!(entry.bank(), "bank service missing for {key}");
        }
        // The owner excludes Dawnport; an existing NPC must not gain this new service.
        let dawnport = model
            .npcs()
            .iter()
            .find(|(reference, _)| reference.key == "oteryn:npc.plunderpurse")
            .map(|(_, npc)| npc)
            .ok_or("Plunderpurse source NPC absent")?;
        assert!(
            !dawnport
                .services
                .iter()
                .any(|service| service.key == "oteryn:service.bank.standard"),
            "Dawnport banker escaped the S10 scope"
        );
        Ok(())
    }
}

#[cfg(test)]
mod qnpcfx_m10_greeting_tests {
    use super::*;
    #[test]
    fn production_reward_greetings_admit_all_m10_npcs_without_fixture_overrides() {
        for key in [
            "oldrak",
            "henricus",
            "kulag_the_guard",
            "grof_the_guard",
            "tim_the_guard",
            "walter_the_guard",
            "miles_the_guard",
        ] {
            let key = format!("oteryn:npc.{key}");
            assert!(reward_callback_npc(&key));
            assert!(
                reward_messages(&key)
                    .is_some_and(|r| !r.greet.is_empty() && !r.farewell.is_empty())
            );
        }
    }
}
