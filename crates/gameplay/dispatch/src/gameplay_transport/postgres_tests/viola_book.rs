//! Real Book of Wisdom command-2 acquisition, durable repeat and replay.
use super::viola_book_fixtures::{definition, free_one, source_world, target};

use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks_fixtures::wire_uses;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::RewardClaimMintRequest;
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};

use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn viola_book_postgres_real_use_repeat_replay_refusal_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("viola-book", scenario)?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED viola_book_postgres_real_use_repeat_replay_refusal_and_fences duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "viola_book", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native generation")?;
        generation_item_facts(native, &definition(51977)).map_err(debug)?;
        let world = source_world()?;
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
            (32613, 32419, 7),
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
                    x: 32613,
                    y: 32419,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated_pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let weights = NativeItemWeights(native);
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: definition(2854),
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(&authority, &h.node, &fence, &weights, &template)
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let target = target(&world)?;
        // Every negative case keeps unrelated identity, content and custody valid.
        {
            let mut owner = deps.owner(&h, &authority);
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
        }
        // A reconstructed owner has a fresh ingress beginning at command 1.
        // Each cold stream sends contiguous IDs; durable refusals wrote no mint cause.
        {
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let mut wrong = target.clone();
            let key =
                oteryn_world_bundle::bundle::placement_key(-7, 32614, 32419, 1).ok_or("key")?;
            wrong.placement[32..].copy_from_slice(&key.to_be_bytes());
            assert_eq!(
                wire_uses(&owner, admitted, &[1], &wrong).await?,
                [UseDisposition::NothingToUse]
            );
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_viola_book_cycles")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(count, 0);
        }
        // No backpack room: refusal must not reserve a cycle or begin the timer.
        for index in 0..20 {
            let request = RewardClaimMintRequest {
                additional_items: Vec::new(),
                backpack_outputs: false,
                command: CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(100 + index).map_err(debug)?,
                ),
                claim: TypedDefinitionRef {
                    family: "RewardClaim".into(),
                    production_key: format!("fixture:viola-book-burden.{index}"),
                    revision_ref: "definition-r1".into(),
                },
                source_placement: "fixture:viola-book-burden".into(),
                item: generation_item_facts(native, &definition(3031)).map_err(debug)?,
                quantity: 1,
                contents: Vec::new(),
                root_text: None,
                root_fluid_subtype: None,
                root_charges_remaining: None,
                backpack: generation_item_facts(native, &template.definition).map_err(debug)?,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
                achievement: None,
                quest_transition: None,
                key_number: None,
            };
            let mut candidate = h
                .root
                .freeze_reward_claim_mint(&authority, &h.node, fence, request)
                .await
                .map_err(debug)?;
            h.root
                .commit_reward_claim_mint(&authority, &h.node, fence, &mut candidate)
                .await
                .map_err(debug)?;
        }
        {
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            assert_eq!(
                wire_uses(&owner, admitted, &[1], &target).await?,
                [UseDisposition::Rejected]
            );
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_viola_book_cycles")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(count, 0);
            let reservations:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations WHERE claim_production_key LIKE 'oteryn:reward-claim.tutorial.viola-book:%'").fetch_one(&h.pool).await?;
            assert_eq!(reservations, 0, "refusal cannot reserve an internal cycle");
            free_one(&owner, fence, 500).await?;
        }
        for _ in 0..2 {
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            assert_eq!(
                wire_uses(&owner, admitted, &[1, 1, 2], &target).await?,
                [
                    UseDisposition::Committed,
                    UseDisposition::Committed,
                    UseDisposition::NothingToUse
                ]
            );
        }
        let claimed: i64 = sqlx::query_scalar(
            "SELECT claimed_at FROM game_character_viola_book_cycles WHERE cycle=1",
        )
        .fetch_one(&h.pool)
        .await?;
        for (now, expected) in [
            (claimed + 299_999, UseDisposition::NothingToUse),
            (claimed + 300_000, UseDisposition::Committed),
        ] {
            sqlx::query(sqlx::AssertSqlSafe(format!("CREATE OR REPLACE FUNCTION game_viola_book_now_ms() RETURNS BIGINT LANGUAGE SQL VOLATILE AS 'SELECT {now}::bigint'"))).execute(&h.pool).await?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            if expected == UseDisposition::Committed {
                free_one(&owner, fence, 501).await?;
            }
            assert_eq!(
                wire_uses(&owner, admitted, &[1, 2], &target).await?,
                [UseDisposition::Committed, expected]
            );
        }
        {
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            assert_eq!(
                wire_uses(&owner, admitted, &[1, 2], &target).await?,
                [UseDisposition::Committed, UseDisposition::Committed]
            );
        }
        let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_viola_book_cycles),(SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key LIKE 'oteryn:reward-claim.tutorial.viola-book:%'),(SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i51977')").fetch_one(&h.pool).await?;
        assert_eq!(counts, (2, 2, 2));
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
