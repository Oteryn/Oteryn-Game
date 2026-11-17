//! Real source-bound bundle chest USE grants, duplicate refusal and durable replay.
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
) -> TestResult {
    run(async |admin| {
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        // Doublet binds the real ground entry (ordinal zero). Other source
        // chests reuse the production dispatch fixture; the companion Python
        // check verifies every UID/appearance against the shipped region files.
        let harness =
            Harness::create_with_admission_seed(admin, "qready1_claim", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = super::quest_callbacks::source_world_choices(chest, choices, appearance)?;
        let start = world.start();
        let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
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
        // The second USE is a new command on the live connection. Rebuild the owner
        // for the replay so no Foundation command-result cache can satisfy it.
        let catalogue = std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        );
        for (commands, expected) in [
            (
                &[1, 2][..],
                &[UseDisposition::Committed, UseDisposition::NothingToUse][..],
            ),
            (&[1][..], &[UseDisposition::Committed][..]),
        ] {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            owner.admit_quest_session(&admitted).await;
            deps.initialize(&owner).await;
            assert_eq!(
                super::quest_callbacks::wire_uses(&owner, admitted, commands, &target)
                    .await
                    .map_err(|e| format!("claim={claim} commands={commands:?}: {e}"))?,
                expected
            );
            let bag = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("bag")?;
            assert_eq!(bag.entries.len(), 1);
            assert_eq!(bag.entries[0].item.quantity, quantity);
            assert_eq!(bag.entries[0].item.definition.production_key, reward);
        }
        let claims: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_production_key=$2), (SELECT count(*) FROM game_reward_claim_mint_receipts)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(claim).fetch_one(&harness.pool).await?;
        assert_eq!(claims, (1, 1));
        if reward == "oteryn:item.tibia.i5956" {
            let text: String = sqlx::query_scalar("SELECT t.text FROM game_item_texts t JOIN game_reward_claim_mint_receipts m USING(item_instance_id) WHERE m.claim_production_key=$1")
                .bind(claim).fetch_one(&harness.pool).await?;
            assert_eq!(
                text,
                "Damn you Thais! Damn you TBI! I am no longer an agent or a noble but a black knight. A dark avenger that will have his revenge!"
            );
        }
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn qaitems_boots_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.quest.u8_4.the_hidden_city_of_beregar.firewalker_boots",
                (32582, 31405, -15),
                &[(32582, 6284)],
                2472,
                "oteryn:item.tibia.i9019",
                1,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "qaitems dispatch panicked")?
        .map_err(Into::into);
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "QAITEMS_DISPATCH_EXECUTED boots duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qaitems_hammer_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.hammer",
                (32776, 32253, -8),
                &[(32776, 6078)],
                2472,
                "oteryn:item.tibia.i3208",
                1,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "qaitems dispatch panicked")?
        .map_err(Into::into);
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "QAITEMS_DISPATCH_EXECUTED hammer duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qaitems_lottery_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.quest.u8_1.secret_service.lottery_ticket",
                (32872, 31958, -11),
                &[(32872, 6250)],
                2469,
                "oteryn:item.tibia.i5956",
                1,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "qaitems dispatch panicked")?
        .map_err(Into::into);
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "QAITEMS_DISPATCH_EXECUTED lottery duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qaitems_poem_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.quest.u8_1.secret_service.poem_scroll",
                (32643, 32733, -7),
                &[(32643, 6251)],
                2472,
                "oteryn:item.tibia.i347",
                1,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "qaitems dispatch panicked")?
        .map_err(Into::into);
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "QAITEMS_DISPATCH_EXECUTED poem duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
