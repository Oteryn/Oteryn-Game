//! S06 source-bound greetings; stateful acquisition remains in the fenced NPC owner.
pub(super) const SOURCE: &str =
    include_str!("../../../../../tools/content-schema/cosmetics-authoring/srv7-npc-grants.json");
pub(super) const PROGRAM_SOURCE: &str =
    include_str!("../../../../../content/cosmetics/npc-acquisitions.json");
pub(super) fn messages(key: &str) -> Option<(&'static str, &'static str)> {
    static PROGRAM: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let programme = PROGRAM
        .get_or_init(|| serde_json::from_str(PROGRAM_SOURCE).unwrap_or(serde_json::Value::Null));
    if let Some(npc) = programme
        .get("npcs")
        .and_then(serde_json::Value::as_array)
        .and_then(|npcs| npcs.iter().find(|npc| npc["npc"].as_str() == Some(key)))
    {
        return Some((npc["greeting"].as_str()?, npc["farewell"].as_str()?));
    }

    let greeting = match key {
        // Crystal 201189f data-global/npc/{amber:428,norma:189,tom:74,
        // cledwyn:212,kesar_the_younger_day:286}.lua.
        "oteryn:npc.amber" => {
            return Some((
                "Oh hello, nice to see you |PLAYERNAME|. Are you here to hear some stories of my {adventures} or do you need {help}?",
                "See you later, |PLAYERNAME|.",
            ));
        }
        "oteryn:npc.norma" => {
            return Some((
                "Welcome, welcome! Have a seat! If you like a drink or something to eat, just ask me for a {trade}!",
                "Come back soon!",
            ));
        }
        "oteryn:npc.tom" => {
            "Hey there, |PLAYERNAME|. I'm Tom the tanner. If you have fresh {corpses}, leather, paws or other animal body parts, {trade} with me."
        }
        "oteryn:npc.cledwyn" => {
            "Blessings, Player! How may I be of service? Do you wish to trade some {token}s, or would you like some {information} or {talk}? Should I {enchant} certain items for you?"
        }
        "oteryn:npc.kesar_the_younger_day" => {
            "Hail, traveller. What brings you here during these dire times under {siege}? The fair Bounac is suffering greatly."
        }
        _ => return None,
    };
    // npc_system/npc_handler.lua:82 library default. Tom's sex-specific farewell
    // remains outside this acquisition slice; opening the exchange is source literal.
    Some((greeting, "Good bye, |PLAYERNAME|."))
}
