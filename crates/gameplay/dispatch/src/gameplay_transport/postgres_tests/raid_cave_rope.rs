use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult};
use crate::foundation::MovementLocalPosition;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

#[test]
fn man_in_the_cave_ropes_a_player_stepping_below_him() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, true)
}

#[test]
fn man_in_the_cave_ignores_the_hole_without_a_live_boss() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, false)
}

fn run(admin: String, boss_present: bool) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("man-in-the-cave", move || {
        crate::bestiary_postgres_harness::runtime()?.block_on(async {
            let h = Harness::create_with_admission_seed(admin, "cave_rope", false, false).await?;
            let result = exercise(&h, boss_present).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup
        })
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED man_in_the_cave_rope boss_present={boss_present}");
    Ok(())
}

async fn exercise(h: &Harness, boss_present: bool) -> TestResult {
    let scene = super::srx3_soul_war_world_fixture_fixtures::source_world(MovementLocalPosition {
        x: 32130,
        y: 31147,
        floor: 3,
    })?;
    let deps = Dependencies::with_content_and_capacity(
        scene.pin.clone(),
        scene.room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        16,
    )?;
    let evidence = super::house_transfer_fixtures::RecoverySource::start(h).await?;
    super::house_transfer_fixtures::prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &evidence.evidence;
    owner.active_generation = scene.controller.active();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
    let mut admitted = deps.admitted;
    super::srx3_activity_source_fixture::prepare_player(h, &owner, &mut admitted, 1999).await?;
    let actor = admitted.runtime_actor.ok_or("current player")?;
    let before = {
        let mut runtime = owner.runtime.lock().await;
        let boss = runtime
            .admit_pinned_monster_lab_creature(
                MovementLocalPosition {
                    x: 32137,
                    y: 31147,
                    floor: 2,
                },
                "oteryn:creature.man_in_the_cave",
                485,
            )
            .map_err(debug)?;
        if !boss_present {
            runtime.set_test_creature_health(boss, 0).map_err(debug)?;
        }
        runtime.read_actor_position(actor).map_err(debug)?
    };
    let (outcome, _) = owner
        .ordinary_paced_step(
            actor,
            admitted.game_session_id,
            StepDirection::East,
            Some(before),
            None,
        )
        .await;
    assert_eq!(outcome.disposition, StepDisposition::Moved);
    let runtime = owner.runtime.lock().await;
    let after = runtime.read_actor_position(actor).map_err(debug)?;
    let (y, floor) = if boss_present { (31146, 2) } else { (31147, 3) };
    assert_eq!(
        after.position(),
        MovementLocalPosition { x: 32131, y, floor }
    );
    let map = owner.bundle_map().ok_or("current World map")?;
    assert_eq!(
        map.facts
            .movement_speech_since(0)
            .iter()
            .any(|(_, speech)| {
                speech.speaker == actor && speech.text == "You have been roped up!"
            }),
        boss_present
    );
    Ok(())
}
