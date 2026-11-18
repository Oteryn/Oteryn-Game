//! Crystal 201189f movements_blood_entrance.lua:7-44 through live World StepIn.
use super::fixture::{Harness, debug};
use super::{TestResult, reset};
use crate::foundation::MovementLocalPosition;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

#[test]
fn rblood2_unpaid_blood_vestibule_step_relocates_and_costs_current_health() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("rblood2-vestibule", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(scenario(&admin))
    })
}

async fn scenario(admin: &str) -> TestResult {
    reset();
    let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
        x: 32954,
        y: 32398,
        floor: 9,
    })?;
    let deps = scene.dependencies()?;
    let h = Harness::create_with_admission_seed(admin, "rblood2_vestibule", false, false).await?;
    let evidence = super::house_transfer::RecoverySource::start(&h).await?;
    super::house_transfer::prepare_owned_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.evidence = &evidence.evidence;
    owner.active_generation = scene.controller.active();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
    super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
    let mut admitted = deps.admitted;
    super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 250).await?;
    let actor = admitted.runtime_actor.ok_or("Vestibule player")?;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "Vestibule gameplay fence unavailable")?
        .ok_or("Vestibule gameplay fence")?;
    let access = async |fence| {
        h.root
            .read_rotten_blood_state(&authority, &h.node, fence)
            .await
            .map(|state| state.access_stage())
            .map_err(debug)
    };
    assert_eq!(access(fence).await?, 0, "durable access starts unpaid");
    let health = || async {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        states
            .get(&runtime, actor, session)
            .map(|state| state.vitals().health)
            .ok_or("Vestibule health")
    };
    let before = health().await?;
    assert!(before > 13);
    let (step, _) = owner
        .ordinary_paced_step(actor, session, StepDirection::West, None, None)
        .await;
    assert_eq!(step.disposition, StepDisposition::Moved);
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(
        position,
        MovementLocalPosition {
            x: 32955,
            y: 32398,
            floor: 9
        }
    );
    assert_eq!(health().await?, before - before / 5);
    assert_eq!(access(fence).await?, 0, "StepIn leaves access unpaid");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
