//! Closed admission for every NPC selected by the Dark Trails caller.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub(super) const TEXT: &str =
    include_str!("../../../../../content/dialogues/quests/dark-trails.json");

#[derive(Deserialize)]
struct Document {
    replies: BTreeMap<String, Reply>,
}
#[derive(Deserialize)]
struct Reply {
    lines: Vec<String>,
}

pub(super) fn messages(npc: &str) -> Option<(String, String)> {
    // Crystal201189f setMessage lines; the ghost's conditional greeting is
    // selected by dispatch, so admission opens with its literal fallback.
    let (greet, farewell) = match npc {
        "oteryn:npc.a_beggar" => ("a_beggar:82", None),
        "oteryn:npc.barazbaz" => ("barazbaz:93", None),
        "oteryn:npc.elliott" => ("elliott:129", Some("elliott:130")),
        "oteryn:npc.ezebeth" => ("ezebeth:118", None),
        "oteryn:npc.jacob" => ("jacob:132", Some("jacob:133")),
        "oteryn:npc.quandons_ghost" => return Some(("Hi!".into(), DEFAULT_FAREWELL.into())),
        "oteryn:npc.roswitha" => ("roswitha:69", None),
        "oteryn:npc.sholley" => ("sholley:83", None),
        "oteryn:npc.terrence" => ("terrence:75", None),
        _ => return None,
    };
    static DATA: OnceLock<Option<Document>> = OnceLock::new();
    let data = DATA
        .get_or_init(|| serde_json::from_str(TEXT).ok())
        .as_ref()?;
    let line = |key| {
        let reply = data.replies.get(&format!("source-only:{key}"))?;
        (reply.lines.len() == 1).then(|| reply.lines[0].clone())
    };
    Some((
        line(greet)?,
        match farewell {
            Some(key) => line(key)?,
            None => DEFAULT_FAREWELL.into(),
        },
    ))
}

// Crystal201189f data/npclib/npc_system/npc_handler.lua:82,90.
const DEFAULT_FAREWELL: &str = "Good bye, |PLAYERNAME|.";
pub(super) const WALKAWAY: &str = "Good bye.";
