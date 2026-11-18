use super::TestResult;
use super::fixture::{Harness, run};
use super::solo_quest_room_ejection::{BossRoom, exercise};

const ANCIENT_FEUD: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_ancient_feud.lua:1-126";

#[test]
fn ancient_feud_step_admits_at_level_250_and_refuses_the_active_boss_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let trial = BossRoom {
        minimum_level: 250,
        player_level: Some(250),
        already_fought: false,
        with_occupant: false,
        allowed: true,
        source: ANCIENT_FEUD,
    };
    case(admin.clone(), "ancient_feud_at_level", trial)?;
    case(
        admin,
        "ancient_feud_cooldown",
        BossRoom {
            already_fought: true,
            allowed: false,
            ..trial
        },
    )
}

#[test]
fn curse_spreads_step_records_the_boss_cooldown_and_refuses_early_reentry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    // Crystal actions_portal_minis_grimvale.lua:102-134.
    let source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua:102-134";
    case(
        admin.clone(),
        "curse_spreads_step",
        BossRoom {
            minimum_level: 1,
            player_level: None,
            already_fought: false,
            with_occupant: false,
            allowed: true,
            source,
        },
    )?;
    case(
        admin,
        "curse_spreads_cooldown",
        BossRoom {
            minimum_level: 1,
            player_level: None,
            already_fought: true,
            with_occupant: false,
            allowed: false,
            source,
        },
    )
}

#[test]
fn ancient_feud_step_refuses_the_level_gate_without_writing_a_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    case(
        admin,
        "ancient_feud_level",
        BossRoom {
            minimum_level: 250,
            player_level: Some(249),
            already_fought: false,
            with_occupant: false,
            allowed: false,
            source: ANCIENT_FEUD,
        },
    )
}

#[test]
fn occupied_boss_room_refuses_entry_then_ejects_current_players_without_removing_boss() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/actions_portal_minis_grimvale.lua:102-134";
    case(
        admin,
        "curse_spreads_room_occupied",
        BossRoom {
            minimum_level: 1,
            player_level: None,
            already_fought: false,
            with_occupant: true,
            allowed: true,
            source,
        },
    )
}

fn case(admin: String, name: &'static str, room: BossRoom) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
                let result = Box::pin(exercise(&h, Some(room))).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Grimvale solo boss case panicked")??;
    Ok(())
}
