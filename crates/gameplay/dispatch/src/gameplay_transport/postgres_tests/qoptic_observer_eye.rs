//! A committed wire STEP drives a second admitted actor on the west field.
use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

#[test]
fn qoptic_observer_eye_wire_step_moves_only_the_paired_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run_async(async move |_| {
        let h =
            Harness::create_with_admission_seed(admin, "qoptic_observer_eye", false, false).await?;
        prepare_current_session(&h).await?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        use crate::world_party_runtime_postgres_cases::{
            FreshPlayerOwners, committed_fresh_player_owned,
        };
        let partner_authority = committed_fresh_player_owned(
            &FreshPlayerOwners {
                root: h.root.clone(),
                node: h.node.clone(),
                recovery: &h.recovery,
            },
            60,
            62,
            70,
            now,
        )
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let fixture = Dependencies::new()?;
        let world = crate::map::spell_tiles::tests::movement_arena::rectangle(
            (32775, 32907),
            (32785, 32918),
            -14,
        );
        let cells = world
            .movement_cells(fixture.room.movement_cells())
            .map_err(debug)?;
        let start = crate::content::NativeEntryStart {
            x: 32782,
            y: 32912,
            floor: 14,
        };
        let room = fixture
            .room
            .bundle_view(
                cells.clone(),
                world.frame_binding_digest(fixture.admitted.world_id),
                world.map_revision_digest(),
                start,
            )
            .ok_or("Observer Eye bundle room")?;
        let pin = ChannelContentPin::from_activation(
            fixture.admitted.world_id,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let mut deps = Dependencies::with_content(pin, room.clone(), fixture.spells)?;
        deps.combat_room = Some(room);
        let partner_session = partner_authority.current_game_session_id();
        let partner_start = MovementLocalPosition {
            x: 32777,
            y: 32907,
            floor: 14,
        };
        let partner = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(partner_session)
                .map_err(debug)?;
            let partner = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    partner,
                    partner_session,
                    partner_authority.current_character_lease(),
                )
                .map_err(debug)?;
            runtime
                .initialize_pinned_test_position(partner, partner_start)
                .map_err(debug)?;
            partner
        };
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        deps.initialize_player(&owner, partner, partner_session)
            .await;
        let mut admitted = deps.admitted;
        for (command, expected_partner_y) in [(1, 32907), (2, 32908), (3, 32908)] {
            let result = super::quest_world_callbacks::wire(
                &owner,
                &mut admitted,
                command,
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                encode_step_intent(StepDirection::South),
            )
            .await?;
            assert_eq!(
                decode_step_result(&result).map_err(debug)?,
                StepDisposition::Moved
            );
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(partner)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    y: expected_partner_y,
                    ..partner_start
                },
            );
        }
        Ok(())
    })
}
