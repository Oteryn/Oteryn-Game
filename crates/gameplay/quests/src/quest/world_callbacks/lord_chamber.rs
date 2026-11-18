//! The four source switches share one encounter in the Edron chamber.
use super::*;

const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/elemental_spheres/actions_lord_of_elements_lever.lua:63-129";
const KEY: &str = "oteryn:quest-callback/elemental-spheres/lord-switch";

pub(super) fn callbacks() -> Result<Vec<Callback>, String> {
    let room = |x, y| Position { x, y, floor: -12 };
    let horizontal = |y| std::array::from_fn(|i| room(33266 + i as u16, y));
    let vertical = |x| std::array::from_fn(|i| room(x, 31834 + i as u16));
    let stations = [
        EncounterSwitch {
            lever: room(33273, 31840),
            stand: room(33272, 31840),
            walls: vertical(33275),
            appearance: 5072,
        },
        EncounterSwitch {
            lever: room(33262, 31840),
            stand: room(33263, 31840),
            walls: horizontal(31843),
            appearance: 5071,
        },
        EncounterSwitch {
            lever: room(33262, 31831),
            stand: room(33263, 31831),
            walls: vertical(33260),
            appearance: 5072,
        },
        EncounterSwitch {
            lever: room(33273, 31831),
            stand: room(33272, 31831),
            walls: horizontal(31828),
            appearance: 5071,
        },
    ];
    let mut callbacks: Vec<_> = stations
        .iter()
        .enumerate()
        .map(|(index, station)| {
            serde_json::json!({
                "key": format!("{KEY}-{}", index + 1),
                "quest": crate::quest::elemental_spheres::QUEST,
                "trigger": "use",
                "selector": {
                    "position": station.lever,
                    "appearance": 2772
                },
                "lever": [2772, 2773],
                "reversible": false,
                "passages": [],
                "storage": [],
                "offerings": [],
                "return_on_failure": false,
                "sources": [SOURCE],
                "effects": {
                    "encounter": {
                        "minimum_level": 0,
                        "players": [],
                        "monsters": [{
                            "position": room(33267, 31836),
                            "creature": "oteryn:creature.lord_of_the_elements"
                        }],
                        "room_min": room(33238, 31806),
                        "room_max": room(33297, 31865),
                        "timing": { "spawn_after_ms": 1, "reset_after_ms": 600000 },
                        "switches": {
                            "stations": stations,
                            "exit": Position { x: 33265, y: 31838, floor: -10 }
                        }
                    }
                }
            })
        })
        .collect();
    let entrance = |x, y| Position { x, y, floor: -10 };
    let stations = [
        (entrance(33268, 31833), room(33268, 31833), "sorcerer", 946),
        (entrance(33268, 31838), room(33267, 31838), "druid", 947),
        (entrance(33266, 31835), room(33265, 31835), "knight", 948),
        (entrance(33270, 31835), room(33270, 31835), "paladin", 942),
    ]
    .map(|(from, to, vocation, item)| PartyStation {
        from,
        to,
        vocation: vocation.into(),
        offering: None,
        carried_item: Some(format!("oteryn:item.tibia.i{item}")),
    });
    callbacks.insert(0, serde_json::json!({
        "key": "oteryn:quest-callback/elemental-spheres/lord-entry",
        "quest": crate::quest::elemental_spheres::QUEST,
        "trigger": "use",
        "selector": { "position": entrance(33269, 31835), "appearance": 2772 },
        "lever": [2772, 2773],
        "reversible": true,
        "passages": [],
        "storage": [{
            "track": "oteryn:quest-progress/elemental_spheres/quest_line",
            "minimum": 1,
            "maximum": 3,
            "set": 1,
            "preserve": true
        }],
        "offerings": [],
        "return_on_failure": false,
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/elemental_spheres/actions_lever.lua:1-67"],
        "effects": {
            "party_transfer": {
                "activation_appearance": 2773,
                "stations": stations,
                "vacant_room": [room(33238, 31806), room(33297, 31865)],
                "initiator_at": entrance(33270, 31835)
            }
        }
    }));
    parse(&serde_json::json!({ "schema": 1, "callbacks": callbacks }).to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn elemental_lord_switches_register_on_four_source_positions() -> Result<(), String> {
        let loaded = super::super::load()?;
        let callbacks: Vec<_> = loaded
            .iter()
            .filter(|callback| callback.key.starts_with(super::KEY))
            .collect();
        assert_eq!(callbacks.len(), 4);
        let encounter = callbacks[0]
            .effects
            .encounter
            .as_ref()
            .ok_or("Lord encounter")?;
        let group = encounter.switches.as_ref().ok_or("coordinated switches")?;
        for (station, callback) in group.stations.iter().zip(callbacks) {
            assert!(callback.selector.matches_position(station.lever));
            assert_eq!(callback.effects.encounter.as_ref(), Some(encounter));
        }
        assert_eq!(encounter.timing.ok_or("timer")?.reset_after_ms, 600_000);
        Ok(())
    }
}
