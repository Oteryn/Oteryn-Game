use super::TestResult;
use super::reaudit_arena_lifecycle_fixtures::{Physical, physical_context};

#[test]
fn n5_drome_postgres_claim_after_rotation_and_channel_restart() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            physical_context(0, true, false, Physical::RotationRestart)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Drome rotation restart thread panicked")??;
    eprintln!(
        "N5_POSTGRES_CASE_COMPLETED drome_claim_after_rotation_and_channel_restart duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn n5_drome_postgres_buy_settles_points_before_item_admission() -> TestResult {
    reward_rollover(DromeAction::Buy(2))
}

#[test]
fn n5_drome_postgres_cube_purchase_spends_points_earned_at_rollover() -> TestResult {
    reward_rollover(DromeAction::Buy(1))
}

#[test]
fn n5_drome_postgres_potion_quote_before_cut_mints_after_cut() -> TestResult {
    reward_rollover(DromeAction::ClaimPotion)
}

use oteryn_protocol_oteryn::arena::{self as wire, ArenaAction, DromeAction};

fn reward_rollover(action: DromeAction) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || reward_rollover_case(action).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Drome reward rollover thread panicked")??;
    eprintln!(
        "N5_POSTGRES_CASE_COMPLETED drome_reward_rollover_{action:?} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn reward_rollover_case(action: DromeAction) -> TestResult {
    use super::fixture::{Harness, debug, id, run};
    use super::{Dependencies, prepare_current_session};
    use crate::durability::tibiadrome::{Progress, ROTATION_SECONDS, Record, shop_reward};
    use crate::foundation::{
        CharacterId, CharacterLease, CommandId, CommandRef, MovementLocalPosition,
    };
    use crate::gameplay_transport::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;

    run(async move |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "n5_drome_cut", false, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let mut owner = deps.owner(&harness, &authority);
        let (controller, _) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        owner.active_generation = controller.active();
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[170])
                .ok_or("Drome capability")?;
        let session = admitted.game_session_id;
        let mut fence = owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    session,
                    CharacterLease::new(
                        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let position = runtime.read_actor_position(deps.actor).map_err(debug)?;
            runtime
                .relocate_arena_player(
                    deps.actor,
                    session,
                    position,
                    MovementLocalPosition {
                        x: 32251,
                        y: 31098,
                        floor: 6,
                    },
                )
                .map_err(debug)?;
        }
        owner.admit_progression_session(&mut admitted).await;
        super::item_look_fixtures::seed_main_backpack(&harness, &[]).await?;

        let rotation = 1_500_u32;
        let cutoff = (rotation + 1) * ROTATION_SECONDS;
        let (level, disposition, points, potions) = match action {
            DromeAction::ClaimPotion => (25, 1, 250, 5),
            DromeAction::Buy(1) => (250, 1, 0, 3),
            // The shipped badge is admitted to Ground. A refused backpack mint
            // still records the current rotation and all 250 unspent points.
            DromeAction::Buy(2) => (25, 8, 250, 3),
            _ => return Err("unsupported rollover case".into()),
        };
        let record = Record {
            progress: Progress {
                rotation,
                level,
                start: level,
                waves: 1,
                ..Progress::default()
            },
            ..Record::default()
        };
        sqlx::query("INSERT INTO game_character_tibiadrome(character_id,body) VALUES(encode($1,'hex')::uuid,$2)")
            .bind(id(41).as_slice()).bind(serde_json::to_value(record)?).execute(&harness.pool).await?;
        let claim = action == DromeAction::ClaimPotion;
        for scored_rotation in (rotation - u32::from(claim))..=rotation {
            sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,1234)")
                .bind(id(42).as_slice()).bind(i64::from(scored_rotation)).bind(id(41).as_slice())
                .bind(i64::from(level))
                .execute(&harness.pool).await?;
        }

        let payload = wire::encode_match_request(ArenaAction::Drome(action)).map_err(debug)?;
        let command_id = admitted.continuity.next_command_id;
        let (bytes, item) = if claim {
            harness.root.set_test_quest_clock(i64::from(cutoff - 1))?;
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            fence.expected_character_revision =
                slot.cursor(owner.root, &authority).await.map_err(debug)?;
            let command = CommandRef::new(session, CommandId::new(command_id).map_err(debug)?);
            let read = owner
                .root
                .read_tibiadrome(&authority, owner.holder, fence, command, &payload)
                .await
                .map_err(debug)?;
            assert_eq!(
                (
                    read.now,
                    read.state.progress.level,
                    read.state.progress.points
                ),
                (cutoff - 1, 25, 0)
            );
            let (award_rotation, claimed, item) = read.potion.ok_or("older rotation potion")?;
            assert_eq!((award_rotation, claimed), (rotation - 1, 0));
            let request = owner
                .drome_reward_request(
                    command,
                    format!("oteryn:reward.tibiadrome.{award_rotation}.{claimed}"),
                    item,
                )
                .await
                .ok_or("potion quote")?;
            assert_eq!(harness.root.advance_test_quest_clock(1)?, i64::from(cutoff));
            let native = owner
                .active_generation
                .and_then(|active| active.native_gameplay())
                .ok_or("native content")?;
            (
                owner
                    .root
                    .claim_drome_reward(
                        &authority,
                        owner.holder,
                        fence,
                        request,
                        payload.clone(),
                        None,
                        &NativeItemWeights(native),
                    )
                    .await
                    .map_err(debug)?,
                item,
            )
        } else if let DromeAction::Buy(selector) = action {
            harness.root.set_test_quest_clock(i64::from(cutoff))?;
            (
                owner
                    .arena_match_command(admitted, command_id, &payload)
                    .await
                    .ok_or("Drome purchase reply")?,
                shop_reward(selector).ok_or("Drome shop item")?.0,
            )
        } else {
            return Err("unsupported rollover action".into());
        };

        let reply = wire::DromeResult::decode(&bytes).map_err(debug)?;
        assert_eq!(
            (
                reply.disposition,
                reply.level,
                reply.start,
                reply.points,
                reply.rank,
                reply.potions,
                reply.rotation_ends
            ),
            (
                disposition,
                0,
                level - 5,
                points,
                0,
                potions,
                cutoff + ROTATION_SECONDS
            ),
            "the mint reply must use the rotation that authorised the reward",
        );
        assert_eq!(
            owner
                .arena_match_command(admitted, command_id, &payload)
                .await
                .ok_or("reward replay")?,
            bytes,
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id) WHERE m.game_session_id=encode($1,'hex')::uuid AND m.command_id=$2::text::numeric AND i.definition_production_key=$3",
        ).bind(session.as_bytes().as_slice()).bind(command_id.to_string())
         .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&harness.pool).await?;
        assert_eq!(receipts, i64::from(disposition == 1));
        Ok(())
    })
}
