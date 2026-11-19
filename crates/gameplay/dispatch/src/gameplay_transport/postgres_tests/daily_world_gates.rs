//! W05b selected native attributes reach the production step event and sealed movement owner.
use super::daily_world_gates_fixtures::{scenario};

use super::TestResult;

use crate::foundation::MovementLocalPosition;

#[test]
fn w05b_daily_gate_real_step_uses_selected_native_ids_and_refuses_unqualified_fury() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("daily-native-gates", || {
        scenario(
            None,
            Some(64001),
            "nightmare_isle/darashia-north",
            true,
            MovementLocalPosition {
                x: 33497,
                y: 32616,
                floor: 8,
            },
            false,
        )?;
        scenario(
            None,
            Some(64001),
            "nightmare_isle/darashia-north",
            false,
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            false,
        )?;
        scenario(
            None,
            Some(35020),
            "nightmare_isle/darashia-north",
            true,
            MovementLocalPosition {
                x: 33215,
                y: 32273,
                floor: 7,
            },
            false,
        )?;
        scenario(
            Some(64103),
            None,
            "nightmare_isle/darashia-north",
            true,
            MovementLocalPosition {
                x: 33475,
                y: 32641,
                floor: 10,
            },
            false,
        )?;
        scenario(
            Some(9715),
            None,
            "fury_gates/thais",
            true,
            MovementLocalPosition {
                x: 32265,
                y: 32164,
                floor: 7,
            },
            false,
        )?;
        scenario(
            Some(9710),
            None,
            "fury_gates/thais",
            true,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            false,
        )?;
        scenario(
            Some(9710),
            None,
            "fury_gates/thais",
            true,
            MovementLocalPosition {
                x: 33290,
                y: 31786,
                floor: 13,
            },
            true,
        )
    })
}

#[test]
fn w05b_fury_offday_exit_keeps_city_return_without_enabling_the_entrance() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("daily-offday-exit", || {
        scenario(
            Some(9715),
            None,
            "fury_gates/thais",
            false,
            MovementLocalPosition {
                x: 32265,
                y: 32164,
                floor: 7,
            },
            false,
        )?;
        scenario(
            Some(9710),
            None,
            "fury_gates/thais",
            false,
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            true,
        )
    })
}
