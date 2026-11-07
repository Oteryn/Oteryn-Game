//! Literal greetings for the closed set of batch-2 native quest NPC callers.
//! Crystal201189f source paths and lines travel with every generated message.
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

mod furniture;
mod horse_rental;
mod seasonal;
/// Immutable source-qualified literal furniture-shop messages.
pub const FURNITURE_SOURCE: &str = furniture::SOURCE;
/// Existing canonical Ape City track used by Hairycles greeting and shop access.
pub const FURNITURE_TRADE_TRACK: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/questline";
/// Whether the NPC belongs to the closed source furniture-shop census.
pub fn is_furniture(npc: &str) -> bool {
    furniture::get(npc).is_some()
}

const PIN: &str = "201189f871d23597b82caf88fd611454a2ddefb8";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    repository: String,
    revision: String,
    path: String,
    line: u32,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub text: String,
    source: Source,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Greeting {
    pub npc: String,
    pub greet: Message,
    pub farewell: Message,
    pub walkaway: Message,
    pub variants: BTreeMap<String, Message>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u8,
    source_revision: String,
    npcs: Vec<Greeting>,
    placeholder_source: Source,
}

fn valid_source(source: &Source) -> bool {
    source.repository == "zimbadev/crystalserver"
        && source.revision == PIN
        && source.line > 0
        && source.sha256.len() == 64
        && source.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        && (source.path.starts_with("data-global/npc/")
            || source.path == "data/npclib/npc_system/npc_handler.lua")
        && !source.path.contains("..")
}

fn parse(text: &str) -> Option<Vec<Greeting>> {
    parse_with_count(text, 23)
}

pub(crate) fn parse_with_count(text: &str, expected: usize) -> Option<Vec<Greeting>> {
    let data: Document = serde_json::from_str(text).ok()?;
    if data.schema != 1
        || data.source_revision != PIN
        || data.npcs.len() != expected
        || !valid_source(&data.placeholder_source)
    {
        return None;
    }
    let mut keys = BTreeSet::new();
    for entry in &data.npcs {
        if !entry.npc.starts_with("oteryn:npc.")
            || !keys.insert(&entry.npc)
            || entry.variants.len() > 1
            || [&entry.greet, &entry.farewell, &entry.walkaway]
                .into_iter()
                .chain(entry.variants.values())
                .any(|message| {
                    message.text.is_empty()
                        || message.text.len() > 1024
                        || message.text.contains(['\0', '\r', '\n'])
                        || !valid_source(&message.source)
                })
        {
            return None;
        }
    }
    Some(data.npcs)
}

pub fn get(npc: &str) -> Option<&'static Greeting> {
    static DATA: LazyLock<Option<Vec<Greeting>>> = LazyLock::new(|| {
        parse(include_str!(
            "../../../../content/dialogues/quests-2/npc-greetings.json"
        ))
    });
    DATA.as_ref()?
        .iter()
        .find(|entry| entry.npc == npc)
        .or_else(|| horse_rental::get(npc))
        .or_else(|| seasonal::get(npc))
        .or_else(|| furniture::get(npc))
}

pub fn is_horse_rental(npc: &str) -> bool {
    horse_rental::get(npc).is_some()
}

pub fn is_seasonal(npc: &str) -> bool {
    seasonal::get(npc).is_some()
}

/// Existing canonical tracks for the three source-owned conditional greetings.
/// zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
/// data-global/npc/a_sleeping_dragon.lua:50-53; data-global/npc/jack_springer.lua:64-67;
/// data-global/npc/hairycles.lua:55-59.
pub const CONDITIONAL_QUEST_TRACKS: [&str; 3] = [
    "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline",
    "oteryn:quest-progress/quest/u12_20/grave_danger/questline",
    FURNITURE_TRADE_TRACK,
];

pub fn uses_quest_progress(npc: &str) -> bool {
    matches!(
        npc,
        "oteryn:npc.a_sleeping_dragon" | "oteryn:npc.jack_springer" | "oteryn:npc.hairycles"
    )
}

/// `None` cache is unreadable, not a new character. A readable copy with an
/// absent track selects the source's default branch; neither guard treats -1 as started.
pub fn select(npc: &str, tracks: Option<&BTreeMap<String, i64>>) -> Option<&'static str> {
    let source = get(npc)?;
    let variant = match npc {
        "oteryn:npc.a_sleeping_dragon" => {
            (tracks?.get(CONDITIONAL_QUEST_TRACKS[0]).copied() == Some(27)).then_some("dream")
        }
        "oteryn:npc.jack_springer" => tracks?
            .get(CONDITIONAL_QUEST_TRACKS[1])
            .is_some_and(|value| *value >= 1)
            .then_some("started"),
        "oteryn:npc.hairycles" => {
            (tracks?.get(FURNITURE_TRADE_TRACK).copied().unwrap_or(0) >= 12).then_some("ape_friend")
        }
        // Sea of Light is not a batch-2 track and grants no greeting authority here.
        "oteryn:npc.spectulus" => return None,
        _ => None,
    };
    Some(match variant {
        Some(key) => source.variants.get(key)?.text.as_str(),
        None => source.greet.text.as_str(),
    })
}

/// The source distinguishes an explicit farewell from leaving the talk radius.
pub fn walkaway(npc: &str) -> Option<&'static str> {
    if npc == "oteryn:npc.spectulus" {
        return None;
    }
    Some(get(npc)?.walkaway.text.as_str())
}

/// Crystal npc_handler.lua:383-391, :407-410 and :546-551. The caller supplies the
/// admitted character name, never a name parsed from the player's message.
pub fn render_player_name(text: &str, name: &str) -> String {
    text.replace("|PLAYERNAME|", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_player_name_tag_uses_the_admitted_name_only() {
        assert_eq!(
            render_player_name("Welcome, |PLAYERNAME|!", "Ada"),
            "Welcome, Ada!"
        );
        assert_eq!(
            render_player_name("|PLAYERNAME| / |PLAYERNAME|", "A&B"),
            "A&B / A&B"
        );
        assert_eq!(render_player_name("|ITEMCOUNT|", "Ada"), "|ITEMCOUNT|");
    }

    #[test]
    fn native_quest_greetings_keep_source_variants_without_admitting_keywords() -> Result<(), String>
    {
        let dragon = get("oteryn:npc.a_sleeping_dragon").ok_or("dragon")?;
        assert_eq!(dragon.greet.text, "Greetings, {wayfarer}.");
        assert_eq!(
            dragon.variants.get("dream").ok_or("dream")?.text,
            "ZzzzZzzZz...chrrr..."
        );
        assert!(get("oteryn:npc.a_sleeping_dragon_fake").is_none());
        assert!(
            get("oteryn:npc.jack_springer")
                .ok_or("Jack")?
                .variants
                .contains_key("started")
        );
        Ok(())
    }
}
