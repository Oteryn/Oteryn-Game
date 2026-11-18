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

/// The donor binds this bookstand by position (ghulosh.lua:1–42).
pub(super) fn ghulosh() -> Result<super::Callback, String> {
    let quest = "oteryn:quest.the_secret_library_quest";
    let at = |x, y| serde_json::json!({"x": x, "y": y, "floor": -10});
    let players = (32747..=32751)
        .map(|x| serde_json::json!({"from":at(x,32773),"to":at(32757,32727)}))
        .collect::<Vec<_>>();
    serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/secret-library-ghulosh",
        "quest": quest,
        "trigger": "use",
        "selector": {"position":at(32746,32773),"appearance":28888,"ordinal":2},
        "reversible": false,
        "passages": [],
        "storage": [],
        "offerings": [],
        "return_on_failure": false,
        "effects": {"encounter": {
            "minimum_level": 250,
            "players": players,
            "monsters": [
                {"creature":"oteryn:creature.ghulosh","position":at(32756,32720)},
                {"creature":"oteryn:creature.bone_jaw","position":at(32755,32721)},
                {"creature":"oteryn:creature.the_book_of_death","position":at(32755,32716)}
            ],
            "room_min":at(32748,32713),
            "room_max":at(32763,32729),
            "phase":"library_ghulosh"
        }},
        "sources": [
            concat!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:",
                "data-global/scripts/actions/bosses_levers/ghulosh.lua:1-42"),
            concat!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:",
                "data-global/scripts/quests/the_secret_library_quest/library_area/",
                "creaturescripts_ghulosh.lua:1-64")
        ]
    }))
    .map_err(|error| error.to_string())
}
