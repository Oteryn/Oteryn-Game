use oteryn_gameplay_quests::quest::{loader, shared_world_progress as shared};
use std::collections::BTreeMap;

#[test]
fn worldchg_hive_native_turnins_credit_personal_favour_once() -> Result<(), String> {
    let loaded = loader::load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    for (task, reward) in [(0, 1), (4, 2)] {
        let binding = &shared::BINDINGS[task];
        let transition = catalogue.transition(binding.key).ok_or("turnin")?;
        let mut stored = BTreeMap::from([
            (binding.track.to_owned(), binding.required),
            (shared::ACTIVE_TRACKS[task].to_owned(), 1),
        ]);
        let changes = catalogue
            .evaluate(transition, &stored, 1_000_000)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            changes
                .iter()
                .find(|c| c.track == "oteryn:quest-progress/hive/favour")
                .map(|c| (c.before, c.after)),
            Some((0, reward))
        );
        for change in changes {
            stored.insert(change.track, change.after);
        }
        assert!(
            catalogue.evaluate(transition, &stored, 1_000_001).is_err(),
            "same objective cannot award favour twice"
        );
    }
    Ok(())
}

#[test]
fn worldchg_hive_purchase_spends_fifty_and_uses_existing_server_timestamp() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::hive_access as access;
    let loaded = loader::load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let purchase = catalogue.transition(access::BUY).ok_or("scent purchase")?;
    let mut stored = BTreeMap::from([(access::FAVOUR.into(), 49)]);
    assert!(catalogue.evaluate(purchase, &stored, 1_000_000).is_err());
    stored.insert(access::FAVOUR.into(), 50);
    let changes = catalogue
        .evaluate(purchase, &stored, 1_000_000)
        .map_err(|e| format!("{e:?}"))?;
    for change in changes {
        stored.insert(change.track, change.after);
    }
    assert_eq!(stored.get(access::FAVOUR), Some(&0));
    assert_eq!(stored.get(access::PURCHASED), Some(&1_000_000));
    assert!(access::permit_valid(1_000_000, 1_604_799));
    assert!(!access::permit_valid(1_000_000, 1_604_800));
    assert!(!access::permit_valid(1_000_000, 999_999));
    assert!(!access::permit_valid(0, 100));
    assert!(catalogue.evaluate(purchase, &stored, 1_000_001).is_err());
    Ok(())
}

#[test]
fn worldchg_hive_actual_native_gates_have_source_axes_and_world_requirements() {
    use oteryn_gameplay_quests::quest::{
        hive_access as access,
        world_callbacks::{self, Position},
    };
    assert_eq!(access::callbacks().len(), 48);
    let mut selectors = std::collections::BTreeSet::new();
    for (index, gate) in access::GATES.iter().enumerate() {
        let key = access::gate_key(index);
        assert!(selectors.insert((gate.position, gate.appearance)));
        assert_eq!(
            world_callbacks::world_change_requirement(&key),
            Some(("hive_born", None))
        );
        let west_x = [33495, 33491, 33492, 33492, 33534, 33514, 33484, 33506]
            [usize::from(gate.position.floor.unsigned_abs() - 1)];
        let western = if gate.appearance < 13290 {
            gate.position.x.abs_diff(west_x) <= 1
        } else {
            gate.position.x == west_x
        };
        assert_eq!(gate.minimum_stage, if western { 3 } else { 2 });
        let actor = if gate.appearance < 13290 {
            Position {
                y: gate.position.y + 1,
                ..gate.position
            }
        } else {
            Position {
                x: gate.position.x + 1,
                ..gate.position
            }
        };
        let target = access::destination(&key, actor);
        let expected = if gate.appearance < 13290 {
            Position {
                y: gate.position.y - 1,
                ..gate.position
            }
        } else {
            Position {
                x: gate.position.x - 1,
                ..gate.position
            }
        };
        assert_eq!(target, Some(expected));
        assert_eq!(access::destination(&key, gate.position), None);
        assert_eq!(
            access::destination(&key, Position { floor: 0, ..actor }),
            None
        );
    }
}

#[test]
fn worldchg_hive_expired_scent_and_closed_phase_keep_only_outward_crossings() {
    use oteryn_gameplay_quests::quest::{
        hive_access as access,
        world_callbacks::{self, Position},
    };
    assert!(!access::permit_valid(1_000_000, 1_604_800));
    for (index, gate) in access::GATES.iter().enumerate() {
        let key = access::gate_key(index);
        let (inner, outer) = if gate.appearance < 13290 {
            let outward = gate.outer_route.y > gate.position.y;
            (
                Position {
                    y: if outward {
                        gate.position.y - 1
                    } else {
                        gate.position.y + 1
                    },
                    ..gate.position
                },
                Position {
                    y: if outward {
                        gate.position.y + 1
                    } else {
                        gate.position.y - 1
                    },
                    ..gate.position
                },
            )
        } else {
            let outward = gate.outer_route.x > gate.position.x;
            (
                Position {
                    x: if outward {
                        gate.position.x - 1
                    } else {
                        gate.position.x + 1
                    },
                    ..gate.position
                },
                Position {
                    x: if outward {
                        gate.position.x + 1
                    } else {
                        gate.position.x - 1
                    },
                    ..gate.position
                },
            )
        };
        assert!(access::is_egress(&key, inner));
        assert!(!access::is_egress(&key, outer));
        assert_eq!(access::observation_key(&key, inner), format!("{key}/exit"));
        assert_eq!(access::observation_key(&key, outer), key);
        assert_eq!(access::destination(&key, inner), Some(outer));
        assert_eq!(
            world_callbacks::world_change_requirement(&key),
            Some(("hive_born", None))
        );
        assert_eq!(
            world_callbacks::world_change_requirement(&format!("{key}/exit")),
            None
        );
        assert!(access::exit_gate(&format!("{key}/exit")).is_some());
        assert_eq!(
            access::callbacks()
                .iter()
                .filter(|c| c.selector.position == Some(gate.position)
                    && c.selector.appearance == Some(gate.appearance))
                .count(),
            1
        );
    }
}
