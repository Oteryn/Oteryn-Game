//! Dream Courts pickup rewards through retained map placements and literal USE.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

// Compile the retained repository regions without injecting chest UIDs or appearances.
fn retained_world(
    chest: (u16, u16, i8),
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle_compiler::{bundle, compile, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let project = source::project_regions(&root, &[(chest.0 / 256, chest.1 / 256)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let (_, mut pins) = crate::map::boot::tests::bundle();
    let identity = oteryn_world_bundle::bundle::read(&crate::map::boot::tests::bundle().0)?
        .manifest
        .identity;
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        },
        &registry,
    )?;
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    for (dx, dy) in [
        (-1, 0),
        (0, 1),
        (1, 0),
        (0, -1),
        (-1, 1),
        (1, 1),
        (-1, -1),
        (1, -1),
    ] {
        let start = crate::map::overlay::TilePos {
            x: u16::try_from(i32::from(chest.0) + dx)?,
            y: u16::try_from(i32::from(chest.1) + dy)?,
            floor: chest.2,
        };
        if !base
            .tile(start.x, start.y, start.floor)
            .is_some_and(|tile| tile.walkable())
        {
            continue;
        }
        match crate::map::boot::boot(
            &compiled.bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins.clone(),
                start,
            },
            crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            |key| {
                use crate::content::{ReferenceItemField, ReferenceItemType};
                let index = native.item_index()?;
                let revision = index.revision(key)?;
                let reference = index.definition_ref("Item", key, revision)?;
                let profile = native
                    .item_policy(key, revision)
                    .map(|policy| policy.record());
                let pickupable = profile.is_some_and(|profile| {
                    matches!(
                    &profile.semantics.physical, ReferenceItemField::Known(physical)
                    if physical.pickupable == ReferenceItemField::Known(true))
                });
                let door = profile.is_some_and(|profile| matches!(
                    &profile.semantics.classification, ReferenceItemField::Known(classification)
                    if classification.item_type == ReferenceItemField::Known(ReferenceItemType::Door)));
                let attributes = profile.map(|profile| &profile.attributes);
                Some(crate::map::facts::ItemDefinition {
                    reference,
                    pickupable,
                    door,
                    solid: attributes.and_then(|a| a.blocks_movement),
                    blocks_projectile: attributes.and_then(|a| a.blocks_projectile).unwrap_or(true),
                })
            },
        ) {
            Ok(world) => return Ok(world),
            Err(crate::map::boot::BootRefusal::StartNotEnterable) => continue,
            Err(error) => return Err(debug(error)),
        }
    }
    Err("source chest has no enterable adjacent tile".into())
}

#[test]
fn anatomy_book_is_delivered_once() -> TestResult {
    pickup_case("dream-courts-anatomy-book", (33671, 32203, -7), 1, 29991)
}

#[test]
fn colourful_mushroom_is_delivered_once() -> TestResult {
    pickup_case(
        "dream-courts-colourful-mushroom",
        (33663, 32192, -7),
        1,
        30009,
    )
}

fn pickup_case(name: &'static str, chest: (u16, u16, i8), ordinal: u8, item: u16) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e::scenario_thread::run(name, move || pickup(chest, ordinal, item));
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn pickup(chest: (u16, u16, i8), ordinal: u8, item: u16) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "qcx47_pickup", false, false).await?;
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
        let world = retained_world(chest, native)?;
        let start = world.start();
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
        let result: TestResult = async {
            let key = oteryn_world_bundle::bundle::placement_key(chest.2, chest.0, chest.1, ordinal)
                .ok_or("pickup placement")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let target = WorldObjectTarget { placement, expected_revision: 0 };
            let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?;
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.quest_catalogue = Some(std::sync::Arc::new(catalogue.catalogue().clone()));
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            owner.admit_quest_session(&admitted).await;
            deps.initialize(&owner).await;
            let results = super::quest_callbacks::wire_uses(&owner, admitted, &[1, 2], &target).await?;
            if results[0] != UseDisposition::Committed {
                return Err(format!("source pickup USE returned {:?}", results[0]).into());
            }
            assert_eq!(results[1], UseDisposition::Rejected, "a second USE must not mint again");
            let bag = harness.root.read_character_backpack(&authority, fence.character_id)
                .await.map_err(debug)?.ok_or("backpack")?;
            assert_eq!(bag.entries.len(), 1);
            assert_eq!(bag.entries[0].item.quantity, 1);
            assert_eq!(bag.entries[0].item.definition.production_key, format!("oteryn:item.tibia.i{item}"));
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE transition_key LIKE 'oteryn:quest-callback/dream-courts/%'")
                .fetch_one(&harness.pool).await?;
            assert_eq!(receipts, 1);
            Ok(())
        }.await;
        drop(authority);
        drop(seal);
        harness.cleanup().await?;
        result
    })
}
