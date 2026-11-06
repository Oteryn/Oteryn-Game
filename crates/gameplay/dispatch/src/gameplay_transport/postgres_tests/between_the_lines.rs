//! Every pinned book-world portal through framed production USE dispatch.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::{Position, Trigger};

#[test]
fn between_the_lines_all_eight_book_portals_preserve_quest_state() -> TestResult {
    let expected = [
        (
            "edron-to-one",
            (33185, 31794, -7),
            50978,
            (32520, 32514, -8),
        ),
        (
            "four-to-one",
            (32585, 32557, -11),
            51756,
            (32520, 32514, -8),
        ),
        (
            "one-to-edron",
            (32522, 32512, -8),
            51528,
            (33185, 31795, -7),
        ),
        (
            "one-to-four",
            (32540, 32512, -8),
            51626,
            (32585, 32558, -11),
        ),
        ("one-to-two", (32526, 32512, -8), 51624, (32526, 32513, -9)),
        (
            "third-labelled-to-two",
            (32536, 32512, -8),
            51625,
            (32526, 32513, -9),
        ),
        (
            "three-to-one",
            (32552, 32594, -10),
            51756,
            (32520, 32514, -8),
        ),
        ("two-to-one", (32526, 32512, -9), 51756, (32520, 32514, -8)),
    ];
    let position = |(x, y, floor)| Position { x, y, floor };
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    let source = callbacks
        .iter()
        .filter(|c| c.quest == "oteryn:quest.between_the_lines_quest")
        .collect::<Vec<_>>();
    assert_eq!(
        source.len(),
        expected.len(),
        "complete pinned portal inventory"
    );
    for (name, at, appearance, destination) in expected {
        let key = format!("oteryn:quest-callback/between-the-lines-{name}");
        let callback = source.iter().find(|c| c.key == key).ok_or("book portal")?;
        assert_eq!(callback.trigger, Trigger::Use);
        assert_eq!(callback.selector.position, Some(position(at)));
        assert_eq!(callback.selector.appearance, Some(appearance));
        assert_eq!(callback.selector.unique, None);
        assert_eq!(callback.selector.action, None);
        assert_eq!(callback.teleport, Some(position(destination)));
        assert!(callback.storage.is_empty() && callback.offerings.is_empty());
        assert!(callback.passages.is_empty() && callback.lever.is_none());
        assert!(callback.item_on_target.is_none() && callback.native_rule.is_none());
    }
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("between-the-lines-portals", move || {
        for (name, at, _, destination) in expected {
            super::quest_world_callbacks::stateless_scenario(
                &format!("between-the-lines-{name}"),
                position(at),
                position(destination),
            )?;
        }
        Ok(())
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED between_the_lines_all_eight_book_portals_preserve_quest_state duration={:?}",
        started.elapsed()
    );
    Ok(())
}
