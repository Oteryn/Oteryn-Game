use super::*;

#[test]
fn quest_world_callbacks_real_use_opens_all_walls_and_toggle_restores_collision() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("quest-world-levers", || {
        scenario(
            "behemoth",
            Position {
                x: 33294,
                y: 31676,
                floor: -15,
            },
            false,
        )?;
        scenario(
            "giant-smithhammer",
            Position {
                x: 32779,
                y: 32230,
                floor: -8,
            },
            false,
        )?;
        scenario(
            "hellgate-bridge",
            Position {
                x: 32626,
                y: 31698,
                floor: -10,
            },
            false,
        )
    })
}

#[test]
fn quest_world_callbacks_real_step_checks_storage_and_teleports() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("quest-world-step", || {
        scenario(
            "liquid-black-shortcut",
            Position {
                x: 33476,
                y: 31314,
                floor: -7,
            },
            false,
        )
    })
}

#[test]
fn quest_world_callbacks_real_paradox_step_consumes_the_four_skulls() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("quest-world-paradox-step", || {
        scenario(
            "paradox-skulls",
            Position {
                x: 32564,
                y: 31958,
                floor: -1,
            },
            false,
        )
    })
}

#[test]
fn quest_world_callbacks_multi_item_lever_consumes_all_altars_and_grants_atomically() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("quest-world-offerings", || {
        scenario(
            "paradox-skulls",
            Position {
                x: 32564,
                y: 31958,
                floor: -1,
            },
            true,
        )
    })
}

#[test]
fn quest_world_callbacks_source_binding_and_visible_collision_batch_are_atomic() -> TestResult {
    use crate::map::view::MapFacts;
    let c = callback("behemoth")?;
    let trigger = Position {
        x: 33294,
        y: 31676,
        floor: -15,
    };
    let w = world(&c, trigger)?;
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("key")?;
    assert!(
        w.facts()
            .quest_callback_matches(key, Some(30009), None, Some(2772))
    );
    assert!(
        !w.facts()
            .quest_callback_matches(key, Some(30010), None, Some(2772))
    );
    let pos = crate::map::overlay::TilePos {
        x: trigger.x,
        y: trigger.y,
        floor: trigger.floor,
    };
    let mut after = w.facts().base_entry(pos, 1, 1).ok_or("lever facts")?;
    after.appearance_id = 2773;
    let passages = c
        .passages
        .iter()
        .map(|p| {
            oteryn_world_bundle::bundle::placement_key(
                p.position.floor,
                p.position.x,
                p.position.y,
                1,
            )
            .ok_or("passage")
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert!(w.facts().quest_callback_capacity(key, 0, passages.len()));
    for p in &c.passages {
        assert!(!w.enterable(crate::map::overlay::TilePos {
            x: p.position.x,
            y: p.position.y,
            floor: p.position.floor
        }));
    }
    assert!(
        w.facts()
            .apply_quest_callback(key, 0, after, &passages, &[], false)
    );
    let pinned = w.facts().pinned();
    for (&key, p) in passages.iter().zip(&c.passages) {
        assert!(pinned.base_hidden(key));
        assert!(w.enterable(crate::map::overlay::TilePos {
            x: p.position.x,
            y: p.position.y,
            floor: p.position.floor
        }));
    }
    assert!(
        !w.facts()
            .apply_quest_callback(key, 0, after, &passages, &[], true)
    );
    assert_eq!(w.facts().object_revision(key), 1);
    assert!(
        w.facts()
            .apply_quest_callback(key, 1, after, &passages, &[], true)
    );
    // A previously pinned view remains coherent while the current one closes.
    assert!(passages.iter().all(|k| pinned.base_hidden(*k)));
    assert!(passages.iter().all(|k| !w.facts().base_hidden(*k)));
    for p in &c.passages {
        assert!(!w.enterable(crate::map::overlay::TilePos {
            x: p.position.x,
            y: p.position.y,
            floor: p.position.floor
        }));
    }
    Ok(())
}
