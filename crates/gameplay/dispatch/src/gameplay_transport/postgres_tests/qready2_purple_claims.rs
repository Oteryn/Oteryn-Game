//! QREADY2: Purple Tome source book claims through literal dispatch USE.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::{source_world, wire_use_targets, wire_uses};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[derive(Clone, Copy)]
struct Case {
    claim: &'static str,
    chest: (u16, u16, i8),
    unique: u16,
    appearance: u16,
    reward: &'static str,
    quantity: u32,
}

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

fn execute(case: Case, marker: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "qready2 scenario panicked")??;
    println!(
        "POSTGRES_CASE_COMPLETED {marker} duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

fn scenario(case: Case) -> TestResult {
    let Case {
        claim,
        chest,
        unique,
        appearance,
        reward,
        quantity,
    } = case;
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "qready2_purple", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let world = source_world(chest, unique, appearance, 0)?;
        let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
        assert!(
            report.served.iter().any(|row| row.key.as_str() == claim),
            "source claim did not bind"
        );
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(chest.0) - 1,
                i32::from(chest.1),
                -i16::from(chest.2),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: i32::from(chest.0) - 1,
                    y: i32::from(chest.1),
                    floor: -i16::from(chest.2),
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated_pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        let weights = NativeItemWeights(native);
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        harness
            .root
            .initialize_admission_equipment(
                &authority,
                &harness.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: definition("oteryn:item.tibia.i2854"),
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        harness
            .root
            .grant_starter_items(&authority, &harness.node, &fence, &weights, &template)
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let target_at = |x| -> TestResult<WorldObjectTarget> {
            let key = oteryn_world_bundle::bundle::placement_key(chest.2, x, chest.1, 1)
                .ok_or("placement")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            Ok(WorldObjectTarget {
                placement,
                expected_revision: 0,
            })
        };
        let target = target_at(chest.0)?;
        {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let mut stale = admitted;
            let mut stale_fence = fence;
            stale_fence.character_lease_generation += 1;
            stale.item_fence = Some(stale_fence);
            assert_eq!(
                wire_uses(&owner, stale, &[1], &target).await?,
                [UseDisposition::Rejected]
            );
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(
                count, 0,
                "stale authority cannot reserve any part of the reward"
            );
        }
        let mut original = None;
        for cold in [false, true] {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let commands = if cold {
                vec![(1, &target)]
            } else {
                vec![(1, &target), (1, &target), (2, &target)]
            };
            let expected = if cold {
                vec![(CommandStatus::Accepted, UseDisposition::Committed)]
            } else {
                vec![
                    (CommandStatus::Accepted, UseDisposition::Committed),
                    (CommandStatus::DuplicateReplay, UseDisposition::Committed),
                    (CommandStatus::Accepted, UseDisposition::NothingToUse),
                ]
            };
            assert_eq!(
                wire_use_targets(&owner, admitted, &commands).await?,
                expected
            );
            let bag = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("backpack")?;
            assert_eq!(bag.entries.len(), 1, "exactly one reward stack");
            let item = &bag.entries[0].item;
            assert_eq!(item.definition.production_key, reward);
            assert_eq!(item.quantity, quantity);
            if let Some(first) = original {
                assert_eq!(item.item_instance_id, first);
            } else {
                original = Some(item.item_instance_id);
            }
            let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE claim_production_key=$1), (SELECT count(*) FROM game_reward_claim_mint_receipts), (SELECT count(*) FROM game_item_audit_outbox WHERE item_instance_id=encode($2,'hex')::uuid)")
                .bind(claim).bind(item.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(
                counts,
                (1, 1, 1),
                "claim, receipt and mint occur exactly once"
            );
        }
        Ok(())
    })
}

#[test]
fn qready2_purple_tome_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.purpletome1",
            chest: (32424, 31591, -15),
            unique: 6128,
            appearance: 2436,
            reward: "oteryn:item.tibia.i2822",
            quantity: 1,
        },
        "QREADY2_PURPLE_TOME_QUEST_1",
    )
}

#[test]
fn qready2_purple_tome_quest_2() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.purpletome2",
            chest: (32427, 31591, -15),
            unique: 6129,
            appearance: 2436,
            reward: "oteryn:item.tibia.i2823",
            quantity: 1,
        },
        "QREADY2_PURPLE_TOME_QUEST_2",
    )
}

#[test]
fn qready2_purple_tome_quest_3() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.purpletome3",
            chest: (32421, 31594, -15),
            unique: 6130,
            appearance: 2438,
            reward: "oteryn:item.tibia.i2848",
            quantity: 1,
        },
        "QREADY2_PURPLE_TOME_QUEST_3",
    )
}
