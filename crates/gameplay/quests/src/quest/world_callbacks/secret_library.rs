//! Crystal 201189f8 movement-owned Furious Scorpion room.

pub(super) fn scorpion() -> Result<super::Callback, String> {
    let key = "oteryn:quest-callback/secret-library-darashia-scorpion";
    let quest = "oteryn:quest.the_secret_library_quest";
    let track = "oteryn:quest-progress/quest/u11_80/the_secret_library/darashia/scorpion_timer";
    let source = concat!(
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:",
        "data-global/scripts/quests/the_secret_library_quest/",
        "the_parth_of_defiances_desert/movements_teleportTo.lua:65-129"
    );
    let entry = serde_json::json!({"x":32963,"y":32312,"floor":-8});
    serde_json::from_value(serde_json::json!({
        "key": key,
        "quest": quest,
        "trigger": "step_in",
        "selector": {"position":entry,"appearance":1949},
        "reversible": false,
        "passages": [],
        "storage": [],
        "offerings": [],
        "requires_world_admission": true,
        "return_on_failure": true,
        "success_message": "You have to master this very last challenge within 5 minutes!",
        "failure_message": "You are still exhausted from your last battle.",
        "state_operations": [{
            "track": track,
            "from": {"kind":"elapsed_at_least","seconds":72000},
            "effect": {"kind":"set_now"}
        }],
        "effects": {"encounter": {
            "minimum_level": 1,
            "players": [{"from":entry,"to":{"x":32958,"y":32309,"floor":-8}}],
            "monsters": [{
                "creature":"oteryn:creature.furious_scorpion",
                "position":{"x":32951,"y":32309,"floor":-8}
            }],
            "room_min":{"x":32943,"y":32303,"floor":-8},
            "room_max":{"x":32960,"y":32315,"floor":-8},
            "room_timeout":{"after_ms":300000,"exit":{"x":32963,"y":32307,"floor":-8}}
        }},
        "sources": [source]
    }))
    .map_err(|error| error.to_string())
}
