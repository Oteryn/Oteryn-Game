use super::*;
use super::super::quest_world_callbacks_fixtures::literal_uses::*;

#[test]
fn quest_world_callbacks_literal_use_accepts_clicked_nonzero_ordinal() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("literal-callback-use", || {
        for (name, appearance, ordinal, source, destination) in [
            (
                "blue-valley-2",
                49948,
                3,
                (33605, 31468, -7),
                (32456, 32291, -7),
            ),
            (
                "blue-valley-3",
                49948,
                3,
                (33601, 31468, -7),
                (32456, 32291, -7),
            ),
            (
                "blue-valley-4",
                49948,
                1,
                (32456, 32290, -7),
                (33614, 31494, -7),
            ),
            (
                "blue-valley-5",
                50310,
                2,
                (32203, 32303, -6),
                (33614, 31494, -7),
            ),
            (
                "pirates-tail-cheesy-exit",
                1760,
                1,
                (33925, 31355, -7),
                (33799, 31321, -7),
            ),
            (
                "order-of-the-stag-boat-return",
                6391,
                2,
                (33874, 32250, -7),
                (32395, 32511, -7),
            ),
            (
                "order-of-the-stag-crypt-exit",
                52656,
                1,
                (32402, 32472, -5),
                (32402, 32473, -4),
            ),
        ] {
            let callback = callback(name)?;
            assert_eq!(callback.selector.position, Some(position(source)));
            assert_eq!(callback.teleport, Some(position(destination)));
            exercise(callback, appearance, ordinal, None)?;
        }
        Ok(())
    })
}

#[test]
fn quest_world_callbacks_read_only_stag_uses_gate_without_quest_writes() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("read-only-stag-use", || {
        for (name, appearance, ordinal, minimum, source, destination) in [
            (
                "order-of-the-stag-boat-to-ada",
                6391,
                4,
                3,
                (32393, 32513, -7),
                (33875, 32249, -7),
            ),
            (
                "order-of-the-stag-mark-wall",
                52735,
                1,
                11,
                (33834, 32139, -4),
                (33833, 32138, -4),
            ),
        ] {
            let callback = callback(name)?;
            assert_eq!(callback.selector.position, Some(position(source)));
            assert_eq!(callback.teleport, Some(position(destination)));
            assert!(callback.is_read_only());
            assert_eq!(callback.storage[0].minimum, minimum);
            for stage in [None, Some(minimum - 1), Some(minimum), Some(13)] {
                exercise(callback.clone(), appearance, ordinal, stage)?;
            }
        }
        // The production map contains the pre-startup wall too; position-based
        // server dispatch must not depend on the donor's visual transform.
        exercise(callback("order-of-the-stag-crypt-exit")?, 1507, 1, None)?;
        Ok(())
    })
}
