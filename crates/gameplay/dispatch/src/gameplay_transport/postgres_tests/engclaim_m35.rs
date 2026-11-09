//! ENGCLAIM: native authored server content through the production dispatch owner and PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

fn claim_case(quest: &'static str) -> TestResult {
    let suffixes = match quest {
        "oteryn:quest.circle_room_quest" => ["circleroom1", "circleroom2"],
        "oteryn:quest.crystal_wand_quest" => ["crystalwand1", "crystalwand2"],
        _ => return Err("selected aggregate quest".into()),
    };
    let claim_keys = suffixes.map(|suffix| format!("oteryn:reward-claim.kv.{suffix}"));
    let transitions = suffixes.map(|suffix| format!("oteryn:quest-transition/kv/{suffix}/chest"));
    let definitions = crate::content::world_reward_claims::REWARD_CLAIM_SHARDS
        .iter()
        .map(|s| serde_json::from_str::<serde_json::Value>(s))
        .collect::<Result<Vec<_>, _>>()?;
    let rows = claim_keys
        .iter()
        .map(|claim| {
            definitions
                .iter()
                .flat_map(|d| d["records"].as_array().into_iter().flatten())
                .find(|row| row["definition"]["identity"]["key"] == *claim)
                .cloned()
                .ok_or("source claim")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let source = &rows[0]["definition"]["placements"][0];
    let p = &source["source_binding"]["project_position"];
    let chest = (
        u16::try_from(p["x"].as_u64().ok_or("x")?)?,
        u16::try_from(p["y"].as_u64().ok_or("y")?)?,
        -i8::try_from(p["z"].as_u64().ok_or("z")?)?,
    );
    let appearance = u16::try_from(source["appearance_tibia_id"].as_u64().ok_or("appearance")?)?;
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "engclaim_m35", false, false).await?;
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
        let positions = rows
            .iter()
            .map(|row| -> TestResult<_> {
                let p = &row["definition"]["placements"][0];
                let pos = &p["source_binding"]["project_position"];
                let uid = p["source_binding"]["legacy_unique_ids"]
                    .as_array()
                    .ok_or("selectors")?
                    .iter()
                    .find(|u| u["server"] == "crystalserver")
                    .ok_or("source selector")?["unique_id"]
                    .as_u64()
                    .ok_or("uid")?;
                assert_eq!(
                    p["appearance_tibia_id"].as_u64(),
                    Some(u64::from(appearance))
                );
                Ok((
                    u16::try_from(pos["x"].as_u64().ok_or("x")?)?,
                    u16::try_from(pos["y"].as_u64().ok_or("y")?)?,
                    u16::try_from(uid)?,
                ))
            })
            .collect::<TestResult<Vec<_>>>()?;
        let world = super::quest_callbacks::source_world_positions(chest, &positions, appearance)?;
        let mut start = world.start();
        start.x += 2;
        let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
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
        let targets = claim_keys
            .iter()
            .map(|claim| -> TestResult<_> {
                let bound = report
                    .served
                    .iter()
                    .find(|r| r.key.as_str() == claim.as_str())
                    .ok_or("source binding")?;
                let mut placement = world.base().digest().to_vec();
                placement
                    .extend_from_slice(&bound.placements[0].bundle_placement_key.to_be_bytes());
                Ok(WorldObjectTarget {
                    placement,
                    expected_revision: 0,
                })
            })
            .collect::<TestResult<Vec<_>>>()?;
        let catalogue = std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        );
        let clock_origin = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("clock")?;
        let admission_start = admitted;
        for (step, &index) in [1_usize, 0].iter().enumerate() {
            admitted = admission_start;
            let mut owner = deps.owner(&harness, &authority);
            owner.clock_origin = clock_origin;
            owner.active_generation = Some(generation);
            owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            owner.admit_quest_session(&admitted).await;
            deps.initialize(&owner).await;
            if step == 0 {
                use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
                use crate::foundation::{CommandId, CommandRef};
                let f = owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(debug)?
                    .ok_or("fence")?;
                let forged = owner
                    .revision_sequencer
                    .acquire(f.character_id)
                    .await
                    .commit_quest_transition(
                        owner.root,
                        owner.character,
                        owner.holder,
                        f,
                        QuestTransitionRequest {
                            transition_key: transitions[index].clone(),
                            cause: QuestCause::Command(CommandRef::new(
                                admitted.game_session_id,
                                CommandId::new(1000).map_err(debug)?,
                            )),
                        },
                        std::sync::Arc::clone(&catalogue),
                    )
                    .await;
                assert!(
                    forged.is_err(),
                    "a generic command cannot turn a claim condition into completion"
                );
            }
            let command = 1 + u64::try_from(step)? * 2;
            // A cold owner starts a new command window and replays prior receipts.
            let mut commands = Vec::new();
            if step > 0 {
                commands.extend([(1, &targets[1]), (2, &targets[1])]);
            }
            commands.extend([(command, &targets[index]), (command + 1, &targets[index])]);
            let results =
                super::quest_callbacks::wire_use_targets_retained(&owner, &mut admitted, &commands)
                    .await?;
            assert_eq!(
                results.iter().map(|(_, d)| *d).collect::<Vec<_>>(),
                vec![UseDisposition::Committed, UseDisposition::NothingToUse].repeat(step + 1)
            );
            let done:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL),(SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes)").bind(quest).fetch_one(&harness.pool).await?;
            assert_eq!(
                done,
                (i64::from(step == 1), i64::from(step == 1)),
                "all claims are required, regardless of order"
            );
        }
        let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims),(SELECT count(*) FROM game_reward_claim_mint_receipts)").fetch_one(&harness.pool).await?;
        assert_eq!(counts, (2, 2));
        let bag = harness
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("bag")?;
        assert_eq!(bag.entries.len(), 2);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
fn execute(quest: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || claim_case(quest).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M35 PG panic")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED engclaim_m35 {quest} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
#[test]
fn engclaim_m35_circle_room_all_claims_cold_owner_and_once_completion() -> TestResult {
    execute("oteryn:quest.circle_room_quest")
}
#[test]
fn engclaim_m35_crystal_wand_all_claims_cold_owner_and_once_completion() -> TestResult {
    execute("oteryn:quest.crystal_wand_quest")
}
