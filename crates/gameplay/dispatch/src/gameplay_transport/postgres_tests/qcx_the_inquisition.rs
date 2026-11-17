//! The Eclipse Grimoire chest uses its original character-once claim on the native map.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn grimoire_chest_without_legacy_uid_claims_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-inquisition-grimoire", scenario);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx-inquisition-grimoire duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario() -> TestResult {
    run(async |admin| {
        let chest = (32649, 31932, -1);
        let harness =
            Harness::create_with_admission_seed(admin, "qcx_inquisition", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native gameplay")?;
        deps.room = activated.qualified_room().clone();
        // The committed B3 tile has the chest at ordinal 1 and no UID.
        let world = super::quest_callbacks::source_world(chest, 0, 2472, 0)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            world.frame_binding_digest(deps.admitted.world_id),
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
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(world.frame_binding_digest(deps.admitted.world_id).as_slice()).execute(&harness.pool).await?;
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
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let key = oteryn_world_bundle::bundle::placement_key(chest.2, chest.0, chest.1, 1)
            .ok_or("key")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        assert_eq!(
            super::quest_callbacks::wire_uses(&owner, admitted, &[1, 2], &target).await?,
            [UseDisposition::Committed, UseDisposition::NothingToUse],
        );
        let bag = harness
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("backpack")?;
        assert_eq!(bag.entries.len(), 1);
        assert_eq!(
            bag.entries[0].item.definition.production_key,
            "oteryn:item.tibia.i7874"
        );
        assert_eq!(bag.entries[0].item.quantity, 1);
        let claims: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1",
        )
        .bind("oteryn:reward-claim.quest.u8_2.the_inquisition_quest.witches_grimoire")
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(claims, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
