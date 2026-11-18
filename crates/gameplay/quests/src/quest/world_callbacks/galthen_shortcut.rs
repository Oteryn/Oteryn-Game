//! Crystal actions_yselda_shortcut.lua:1–30 uses the shared boss cooldown.
use super::{Callback, Position};

const BOSS: &str = "oteryn:boss.megasylvan_yselda";
const ROUTES: [(&str, Position, Position, u8); 2] = [
    (
        "oteryn:quest-callback/galthen-yselda-shortcut-1",
        Position {
            x: 32419,
            y: 32458,
            floor: -7,
        },
        Position {
            x: 32528,
            y: 32489,
            floor: -11,
        },
        2,
    ),
    (
        "oteryn:quest-callback/galthen-yselda-shortcut-2",
        Position {
            x: 32528,
            y: 32488,
            floor: -11,
        },
        Position {
            x: 32418,
            y: 32458,
            floor: -7,
        },
        3,
    ),
];

pub(super) fn boss_for_callback(key: &str) -> Option<&'static str> {
    ROUTES
        .iter()
        .any(|(route, ..)| *route == key)
        .then_some(BOSS)
}

pub(super) fn callbacks() -> Result<Vec<Callback>, String> {
    let rows = ROUTES.map(|(key, source, destination, line)| {
        serde_json::json!({
            "key": key,
            "quest": "oteryn:quest.adventures_of_galthen_quest",
            "trigger": "use",
            "selector": {"position": source, "unique": null, "action": null, "appearance": null},
            "lever": null,
            "reversible": false,
            "passages": [],
            "teleport": destination,
            "effects": {"teleport_effect": 11},
            "storage": [],
            "offerings": [],
            "return_on_failure": true,
            "failure_message": "This root seems to thicken and wreathe the moment you try to grab it.",
            "sources": [format!(
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/adventures_of_galthen/actions_yselda_shortcut.lua:{line}"
            )],
        })
    });
    super::parse(&serde_json::json!({"schema": 1, "callbacks": rows}).to_string())
}
