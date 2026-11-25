//! The Crystal reward portal must consult the current Character's completion.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::srx3_activity_source_fixture::prepare_player;
use super::srx3_soul_war_world_fixture_fixtures::source_world;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::SeamRooms;
use oteryn_gameplay_quests::quest::soul_war::MINIMUM_LEVEL;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

#[test]
fn w11_soul_war_reward_portal_refuses_before_completion() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_async(async |admin| {
        super::reset();
        // Crystal moveevent-teleport_entrance_reward.lua registers the tile
        // immediately north of this walkable tile in the published hub.
        let scene = source_world(MovementLocalPosition {
            x: 33621,
            y: 31417,
            floor: 10,
        })?;
        let deps = scene.dependencies()?;
        let h =
            Harness::create_with_admission_seed(admin, "w11_soul_war_reward_gate", false, false)
                .await?;
        let evidence = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
        super::house_transfer_fixtures::prepare_owned_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &evidence.evidence;
        owner.active_generation = scene.controller.active();
        owner.rooms = SeamRooms::new(Some(&scene.room), Some(&scene.room));
        super::boss_activity_fixtures::issue_content_and_reset(&h, &scene.pin).await?;
        let mut admitted = deps.admitted;
        prepare_player(&h, &owner, &mut admitted, MINIMUM_LEVEL).await?;
        let session = admitted.game_session_id;
        let actor = admitted.runtime_actor.ok_or("current World actor")?;
        let position = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        let fence = super::soul_war_fixtures::current(&owner, session).await?;
        let state = h
            .root
            .read_soul_war_state(&authority, &h.node, fence)
            .await?;
        assert!(!state.completed());
        eprintln!("W11_SOUL_WAR_REWARD_GATE_POSTGRES_READY");

        let (step, _) = owner
            .ordinary_paced_step(actor, session, StepDirection::North, Some(position), None)
            .await;
        assert_eq!(
            step.disposition,
            StepDisposition::Moved,
            "physical STEP reaches the hub portal"
        );
        let after = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        assert_eq!(
            after.position(),
            position.position(),
            "the completion gate returns the player to the preceding tile"
        );
        assert_eq!(
            after.facing(),
            Some(crate::foundation::MovementFacing::North)
        );
        assert_eq!(
            h.root
                .read_soul_war_state(&authority, &h.node, fence)
                .await?,
            state
        );
        let character = fence.character_id;
        assert!(
            h.root
                .pending_soul_war_reward(&authority, character)
                .await?
                .is_none()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
