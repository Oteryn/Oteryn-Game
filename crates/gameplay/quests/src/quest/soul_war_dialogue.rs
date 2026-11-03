//! Flickering Soul's ordered Crystal201189f branches and complete source replies.
//! The existing dispatch consumer retains NPC/session authority and durable writes.
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

const EMBEDDED: &str = include_str!("../../../../../content/dialogues/soul-war.json");
const PIN: &str = "201189f871d23597b82caf88fd611454a2ddefb8";
const NPC_SOURCE: &str = "data-global/npc/flickering_soul.lua";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Reply {
    Living,
    Mortal,
    Goshnar,
    Ambition,
    Milestone,
    Everything,
    Accomplish,
    Dead,
    Confident,
    Peace,
    Soul,
    Weariness,
    Knowledge,
    Return,
    Fetters,
    Powerful,
    TaskComplete,
    TaskOffer,
    Accept,
    Burden,
    Shards,
    Hate,
    Fermuba,
    Ferumbras,
    Grandson,
    PaleWorm,
    NecromantKing,
    Followers,
    Cleansed,
    Untainted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Reply(Reply),
    Task,
    Accept,
    Cleanse,
}

// Crystal npc/flickering_soul.lua:62-191. The first shards branch at144
// shadows the later computed branch at169; both source expressions are retained
// as provenance, but the unreachable branch must not replace this reply.
const BRANCHES: &[(&[&str], Choice)] = &[
    (&["living"], Choice::Reply(Reply::Living)),
    (&["mortal"], Choice::Reply(Reply::Mortal)),
    (&["goshnar"], Choice::Reply(Reply::Goshnar)),
    (&["ambition"], Choice::Reply(Reply::Ambition)),
    (&["milestone"], Choice::Reply(Reply::Milestone)),
    (&["everything"], Choice::Reply(Reply::Everything)),
    (&["accomplish"], Choice::Reply(Reply::Accomplish)),
    (&["dead"], Choice::Reply(Reply::Dead)),
    (&["confident"], Choice::Reply(Reply::Confident)),
    (&["peace"], Choice::Reply(Reply::Peace)),
    (&["soul"], Choice::Reply(Reply::Soul)),
    (&["weariness"], Choice::Reply(Reply::Weariness)),
    (&["knowledge"], Choice::Reply(Reply::Knowledge)),
    (&["return"], Choice::Reply(Reply::Return)),
    (&["fetters"], Choice::Reply(Reply::Fetters)),
    (&["powerful"], Choice::Reply(Reply::Powerful)),
    (&["task"], Choice::Task),
    (&["yes"], Choice::Accept),
    (&["burden"], Choice::Reply(Reply::Burden)),
    (&["shards"], Choice::Reply(Reply::Shards)),
    (&["hate"], Choice::Reply(Reply::Hate)),
    (&["fermuba"], Choice::Reply(Reply::Fermuba)),
    (&["ferumbras"], Choice::Reply(Reply::Ferumbras)),
    (&["grandson"], Choice::Reply(Reply::Grandson)),
    (&["pale worm"], Choice::Reply(Reply::PaleWorm)),
    (&["necromant king"], Choice::Reply(Reply::NecromantKing)),
    (&["minions", "followers"], Choice::Reply(Reply::Followers)),
    (&["taints", "penalties"], Choice::Cleanse),
];

/// Crystal data/npclib/npc.lua:13-20: a keyword must occur, and no occurrence
/// may immediately follow an ASCII alphanumeric character (%w in the donor).
/// There is no required trailing word boundary. These source keywords contain
/// no Lua pattern metacharacters.
fn message_contains(message: &str, keyword: &str) -> bool {
    message == keyword
        || (message.contains(keyword)
            && !message.match_indices(keyword).any(|(start, _)| {
                start > 0 && message.as_bytes()[start - 1].is_ascii_alphanumeric()
            }))
}

#[must_use]
pub fn choose(message: &str, accepting_task: bool) -> Option<Choice> {
    if super::quests_2_focus::intercepts(message) {
        return None;
    }
    let message = message.to_ascii_lowercase();
    BRANCHES.iter().find_map(|(keywords, choice)| {
        if *choice == Choice::Accept && !accepting_task {
            return None;
        }
        keywords
            .iter()
            .any(|keyword| message_contains(&message, keyword))
            .then_some(*choice)
    })
}

#[derive(Deserialize)]
struct Document {
    schema: String,
    family: String,
    records: Vec<Record>,
}

#[derive(Deserialize)]
struct Record {
    declaration: Declaration,
}

#[derive(Deserialize)]
struct Declaration {
    identity: Identity,
    greet: Vec<String>,
    keywords: Vec<Keyword>,
    fields: Vec<Field>,
}

#[derive(Deserialize)]
struct Identity {
    key: String,
}

#[derive(Deserialize)]
struct Keyword {
    key: Reply,
    reply: Vec<String>,
}

#[derive(Deserialize)]
struct Field {
    field_path: String,
    value: TextField,
}

#[derive(Deserialize)]
struct TextField {
    value: String,
}

#[derive(Deserialize)]
struct Provenance {
    repository: String,
    revision: String,
    path: String,
}

fn parse(text: &str) -> Result<Declaration, String> {
    let mut document: Document = serde_json::from_str(text).map_err(|error| error.to_string())?;
    if document.schema != "OTERYN_DIALOGUE_AUTHORING_SHARD/v1"
        || document.family != "Dialogue"
        || document.records.len() != 1
    {
        return Err("Soul War dialogue envelope".into());
    }
    let record = document.records.pop().ok_or("Soul War dialogue record")?;
    let declaration = record.declaration;
    let unique: BTreeSet<_> = declaration.keywords.iter().map(|entry| entry.key).collect();
    if declaration.identity.key != "oteryn:dialogue.npc.flickering_soul"
        || declaration.greet.len() != 1
        || declaration.greet[0] != "Be greeted, living soul!"
        || declaration.keywords.len() != 30
        || unique.len() != 30
        || declaration.keywords.iter().any(|entry| {
            entry.reply.is_empty()
                || entry.reply.len() > 4
                || entry
                    .reply
                    .iter()
                    .any(|line| line.is_empty() || line.len() > 2048 || line.contains(['\0', '\r']))
        })
    {
        return Err("Soul War dialogue replies".into());
    }
    let field = declaration
        .fields
        .iter()
        .find(|field| field.field_path == "oteryn:source.quest.soul_war.dialogue")
        .ok_or("Soul War dialogue source")?;
    let source: Provenance =
        serde_json::from_str(&field.value.value).map_err(|error| error.to_string())?;
    if source.repository != "zimbadev/crystalserver"
        || source.revision != PIN
        || source.path != NPC_SOURCE
    {
        return Err("Soul War dialogue donor pin".into());
    }
    Ok(declaration)
}

/// A malformed authored catalogue refuses dialogue; it never substitutes an
/// invented reply or makes a state-changing choice succeed.
#[must_use]
pub fn reply(key: Reply) -> Option<&'static [String]> {
    static DATA: OnceLock<Result<Declaration, String>> = OnceLock::new();
    DATA.get_or_init(|| parse(EMBEDDED))
        .as_ref()
        .ok()?
        .keywords
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.reply.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_focus_precedes_soul_default_callback() {
        // Crystal201189f flickering_soul.lua:200 installs FocusModule before
        // CALLBACK_MESSAGE_DEFAULT dispatch (npc_handler.lua:446-451).
        for message in ["yes hi", "task hello", "taints trade", "yes _hi_"] {
            assert_eq!(
                choose(message, true),
                None,
                "FocusModule must intercept {message}"
            );
        }
        // modules.lua:379-387 rejects the whole focus word if any occurrence
        // has an alphanumeric or '+' neighbour. '_' is not in that class.
        for message in ["yes high", "yes +hi", "yes hi xhi", "yes hi hi+"] {
            assert_eq!(
                choose(message, true),
                Some(Choice::Accept),
                "source focus boundary: {message}"
            );
        }
    }

    #[test]
    fn soul_war_dialogue_preserves_donor_order_and_topic_gate() {
        assert_eq!(
            choose("living task", false),
            Some(Choice::Reply(Reply::Living))
        );
        assert_eq!(choose("please TASK", false), Some(Choice::Task));
        assert_eq!(choose("yes", false), None);
        assert_eq!(choose("yes", true), Some(Choice::Accept));
        assert_eq!(
            choose("yes burden", false),
            Some(Choice::Reply(Reply::Burden))
        );
        assert_eq!(choose("yes burden", true), Some(Choice::Accept));
        assert_eq!(choose("shards", true), Some(Choice::Reply(Reply::Shards)));
        assert_eq!(choose("no", true), None);
        assert_eq!(choose("penalties", false), Some(Choice::Cleanse));
    }

    #[test]
    fn soul_war_dialogue_matches_the_actual_msg_contains_helper() {
        assert!(!message_contains("immortal", "mortal"));
        assert!(!message_contains("mortal immortal", "mortal"));
        assert!(message_contains("mortalities", "mortal"));
        assert!(message_contains("_mortal", "mortal"));
        assert!(message_contains("the mortal", "mortal"));
        assert_eq!(
            choose("GOSHNAR", false),
            Some(Choice::Reply(Reply::Goshnar))
        );
    }

    #[test]
    fn soul_war_dialogue_retains_every_multiline_source_reply() -> Result<(), String> {
        let data = parse(EMBEDDED)?;
        assert_eq!(data.keywords.len(), 30);
        let peace = reply(Reply::Peace).ok_or("peace reply")?;
        assert_eq!(peace.len(), 4);
        assert!(peace[3].contains("weighed heavily on my soul"));
        let completed = reply(Reply::TaskComplete).ok_or("complete reply")?;
        assert_eq!(completed.len(), 2);
        assert!(completed[1].contains("My eternal gratitude and blessings"));
        let shards = reply(Reply::Shards).ok_or("shards reply")?;
        assert_eq!(shards.len(), 1);
        assert!(shards[0].contains("You haven't killed Greed yet."));
        Ok(())
    }

    #[test]
    fn soul_war_dialogue_rejects_missing_duplicate_and_unpinned_source_data() -> Result<(), String>
    {
        let original: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|error| error.to_string())?;
        let mut missing = original.clone();
        missing["records"][0]["declaration"]["keywords"][0]["reply"] = serde_json::json!([]);
        assert!(parse(&missing.to_string()).is_err());
        let mut duplicate = original.clone();
        duplicate["records"][0]["declaration"]["keywords"][1]["key"] = serde_json::json!("living");
        assert!(parse(&duplicate.to_string()).is_err());
        assert!(parse(&EMBEDDED.replace(PIN, "unverified")).is_err());
        Ok(())
    }
}
