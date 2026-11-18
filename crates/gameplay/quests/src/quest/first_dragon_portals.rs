//! Crystal 201189f8, movements_entrance_teleport.lua:1–97.
use super::world_callbacks::{Callback, Position};

const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
pub(super) const TRACK_PREFIX: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/";

pub(super) fn callbacks() -> Result<Vec<Callback>, String> {
    let position = |(x, y)| Position { x, y, floor: -8 };
    let rows: Vec<_> = [
        (35001, "tazhadur", "dragon_counter", 200, (32015, 32466), (32018, 32465)),
        (35002, "kalyassa", "chest_counter", 4, (32078, 32456), (32079, 32459)),
        (35003, "zorvorax", "secrets_counter", 3, (32008, 32396), (32015, 32396)),
        (35004, "gelidrazah_the_frozen", "gelidrazah_access", 1, (32076, 32402), (32078, 32400)),
    ]
    .into_iter()
    .map(|(uid, creature, track, value, entry, (x, y))| {
        serde_json::json!({
            "key": format!("oteryn:quest-callback/first-dragon-entry-{uid}"),
            "quest": QUEST,
            "trigger": "step_in",
            "selector": {"unique": uid},
            "lever": null,
            "reversible": false,
            "requires_world_admission": true,
            "passages": [],
            "teleport": null,
            "storage": [
                {"track": format!("{TRACK_PREFIX}questline"), "minimum": 1, "maximum": i32::MAX, "set": 1, "preserve": true},
                {"track": format!("{TRACK_PREFIX}{track}"), "minimum": value, "maximum": i32::MAX, "set": value, "preserve": true}
            ],
            "state_operations": [{
                "track": format!("{TRACK_PREFIX}{creature}_last_entry"),
                "from": {"kind": "elapsed_at_least", "seconds": 7200},
                "effect": {"kind": "set_now"}
            }],
            "effects": {"encounter": {
                "minimum_level": 0,
                "players": [],
                "step_destination": position(entry),
                "monsters": [{"creature": format!("oteryn:creature.{creature}"), "position": position((x, y))}],
                "room_min": position((x - 10, y - 10)),
                "room_max": position((x + 10, y + 10))
            }},
            "offerings": [],
            "return_on_failure": true,
            "failure_message": "Sorry, not possible.",
            "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/movements_entrance_teleport.lua:1-97"]
        })
    })
    .collect();
    super::world_callbacks::parse(&serde_json::json!({"schema": 1, "callbacks": rows}).to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn descendant_portals_register_fenced_step_encounters() -> Result<(), String> {
        let registered = super::super::world_callbacks::load()?;
        for portal in super::callbacks()? {
            assert!(registered.contains(&portal));
        }
        Ok(())
    }
    #[test]
    fn first_dragon_descendant_clock_reopens_at_two_hours() -> Result<(), String> {
        let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        for portal in super::callbacks()? {
            let transition = catalogue
                .transition(&portal.key)
                .ok_or("portal transition")?;
            let clock = &portal.state_operations[0].track;
            let mut tracks = std::collections::BTreeMap::new();
            assert!(catalogue.evaluate(transition, &tracks, 10_000).is_err());
            for state in portal.storage {
                tracks.insert(state.track, state.minimum);
            }
            assert!(catalogue.evaluate(transition, &tracks, 10_000).is_ok());
            tracks.insert(clock.clone(), 10_000);
            assert!(catalogue.evaluate(transition, &tracks, 17_199).is_err());
            assert!(catalogue.evaluate(transition, &tracks, 17_200).is_ok());
        }
        Ok(())
    }
}
