//! ENGCLAIM: native authored server content through the production dispatch owner and PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

fn claim_case(
    claim: &str,
    chest: (u16, u16, i8),
    choices: &[(u16, u16)],
    appearance: u16,
    reward: &str,
    quantity: u32,
    filled: bool,
) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "qready1_claim", false, false).await?;
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
        let positions = choices
            .iter()
            .enumerate()
            .map(|(index, (_, uid))| {
                Ok((
                    chest.0,
                    chest
                        .1
                        .checked_add(u16::try_from(index)?)
                        .ok_or("position")?,
                    *uid,
                ))
            })
            .collect::<TestResult<Vec<_>>>()?;
        let world = super::quest_callbacks::source_world_positions(chest, &positions, appearance)?;
        let placements = positions.iter().enumerate().map(|(index, (x, y, _uid))| {
            let mut reward = serde_json::json!({"items": [{"item": {"family": "Item", "key": if !filled && index > 0 { "oteryn:item.tibia.i3354" } else { reward }, "revision": "definition-r1"}, "count": quantity}]});
            if filled {
                reward["contents"] = serde_json::json!([
                    {"item": {"family": "Item", "key": "oteryn:item.tibia.i3031", "revision": "definition-r1"}, "count": 100},
                    {"item": {"family": "Item", "key": "oteryn:item.tibia.i3031", "revision": "definition-r1"}, "count": 100},
                    {"item": {"family": "Item", "key": "oteryn:item.tibia.i3031", "revision": "definition-r1"}, "count": 1}
                ]);
            }
            let mut placement = serde_json::json!({"appearance_tibia_id": appearance,
                "source_binding": {"project_position": {"x": x, "y": y, "z": -chest.2},
                    "native_entry_ordinal": 1}, "reward": reward});
            if !filled { placement["achievement"] = "oteryn:achievement/annihilator".into(); }
            placement
        }).collect::<Vec<_>>();
        let document = serde_json::json!({"records": [{"definition": {
            "identity": {"key": claim, "revision": "1"}, "definition_profile": "native_authored_v1", "readiness": "ready",
            "claim": {"per": "character", "repeat": {"kind": "once"}}, "placements": placements
        }}]}).to_string();
        let shards = [document.as_str()];
        let start = world.start();
        let report = crate::content::world_reward_claims::bind_reward_claims(
            world.base().as_ref(),
            &shards,
            &|_| true,
        )?;
        assert!(
            report
                .served
                .iter()
                .any(|entry| entry.key.as_str() == claim),
            "{claim}: {:?}",
            report.unbound
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
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(start.floor),
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
                    x: i32::from(start.x),
                    y: i32::from(start.y),
                    floor: -i16::from(start.floor),
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
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated_pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
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
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
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
        let key = report
            .served
            .iter()
            .find(|entry| entry.key.as_str() == claim)
            .and_then(|entry| {
                entry
                    .placements
                    .iter()
                    .find(|p| p.cell.x == chest.0 && p.cell.y == chest.1)
            })
            .ok_or("bound source placement")?
            .bundle_placement_key;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        let mut alternative_placement = world.base().digest().to_vec();
        alternative_placement.extend_from_slice(
            &report.served[0].placements[1]
                .bundle_placement_key
                .to_be_bytes(),
        );
        let alternative = WorldObjectTarget {
            placement: alternative_placement,
            expected_revision: 0,
        };
        // A different placement of this claim offers a different allocation.
        // Cold-owner replay must still return the first committed allocation.
        for (commands, expected) in [
            (
                &[1, 2][..],
                &[UseDisposition::Committed, UseDisposition::NothingToUse][..],
            ),
            (&[1][..], &[UseDisposition::Committed][..]),
        ] {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let mut stale = fence;
            stale.character_lease_generation += 1;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(1))
                .ok_or("clock")?;
            let refused = owner
                .use_bundle_chest_from_shards(
                    deps.actor,
                    crate::gameplay_transport::connection::UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 99,
                        item_fence: Some(stale),
                    },
                    owner.bundle_map().ok_or("bundle map")?,
                    &target,
                    &shards,
                )
                .await
                .ok_or("binding")?;
            assert_eq!(refused.disposition, UseDisposition::Rejected);
            let before: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_reward_claim_mint_reservations WHERE command_id=99",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(
                before, 0,
                "stale authority cannot reserve an authored reward"
            );
            for (&command_id, &expected) in commands.iter().zip(expected) {
                // Call the production dispatch owner with server-authored content.
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("clock")?;
                let result = owner
                    .use_bundle_chest_from_shards(
                        deps.actor,
                        crate::gameplay_transport::connection::UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id,
                            item_fence: Some(fence),
                        },
                        owner.bundle_map().ok_or("bundle map")?,
                        if command_id == 2 {
                            &alternative
                        } else {
                            &target
                        },
                        &shards,
                    )
                    .await
                    .ok_or("authored placement did not bind")?;
                assert_eq!(result.disposition, expected);
            }
            let bag = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("bag")?;
            assert_eq!(bag.entries.len(), 1);
            assert_eq!(bag.entries[0].item.quantity, quantity);
            assert_eq!(bag.entries[0].item.definition.production_key, reward);
            if filled {
                let children: (i64, i64) = sqlx::query_as("SELECT count(*), sum(quantity)::bigint FROM game_item_instances WHERE item_instance_id IN (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)")
                    .bind(bag.entries[0].item.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(children, (3, 201));
            }
        }
        let claims: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_production_key=$2), (SELECT count(*) FROM game_reward_claim_mint_receipts)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(claim).fetch_one(&harness.pool).await?;
        assert_eq!(claims, (1, 1));
        let achievements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/annihilator'").fetch_one(&harness.pool).await?;
        assert_eq!(achievements, i64::from(!filled));
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

fn execute(filled: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            claim_case(
                "oteryn:reward-claim.authored.engclaim",
                (100, 100, -7),
                &[(100, 6000), (101, 6001)],
                2472,
                if filled {
                    "oteryn:item.tibia.i2853"
                } else {
                    "oteryn:item.tibia.i3288"
                },
                1,
                filled,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native authored claim scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED engclaim_native filled={filled} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn engclaim_native_container_allocation_once_and_cold_replay() -> TestResult {
    execute(true)
}

#[test]
fn engclaim_native_vault_reward_achievement_once_and_cold_replay() -> TestResult {
    execute(false)
}
