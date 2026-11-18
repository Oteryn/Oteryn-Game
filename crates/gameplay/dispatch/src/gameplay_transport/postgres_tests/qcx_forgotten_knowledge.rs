//! Forgotten Knowledge compatibility checks against TibiaWiki spoiler rev. 1200256.
use super::fixture::{Harness, MigratedDatabaseTemplate, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

use super::l6_world_pickups::NativeCrop;
use super::quest_catalogue_chests::CatalogueActivation;

#[test]
fn albinius_ice_offering_requires_contactman_and_fifty_fish() -> TestResult {
    super::qengine_rewards::scenario("questpc2-albinius-portals-exact")
}

#[test]
fn retained_world_claims_grant_each_quest_item_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }

    super::kill_reward_e2e::scenario_thread::run("qcx44-claims", || {
        let started = std::time::Instant::now();
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let activation =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let mut template = None;
        run(async |admin| {
            template = Some(MigratedDatabaseTemplate::create(admin).await?);
            Ok(())
        })?;
        let template = template.ok_or("migration template")?;
        let result = (|| -> TestResult {
            macro_rules! region {
                ($coordinates:literal) => {
                    include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-",
                        $coordinates,
                        ".b3"
                    ))
                    .as_slice()
                };
            }

            let mut failures = Vec::new();
            for (suffix, bytes, x, y, z, start, ordinal, item) in [
                (
                    "halls_of_hope.reward1",
                    region!("z09-x126-y125"),
                    32349,
                    32194,
                    9,
                    (32348, 32194),
                    1,
                    23986,
                ),
                (
                    "halls_of_hope.reward2",
                    region!("z09-x126-y126"),
                    32382,
                    32368,
                    9,
                    (32382, 32369),
                    2,
                    23986,
                ),
                (
                    "halls_of_hope.reward3",
                    region!("z07-x126-y125"),
                    32287,
                    32119,
                    7,
                    (32287, 32120),
                    1,
                    23986,
                ),
                (
                    "halls_of_hope.reward4",
                    region!("z06-x126-y125"),
                    32389,
                    32001,
                    6,
                    (32390, 32001),
                    1,
                    23986,
                ),
                (
                    "halls_of_hope.reward5",
                    region!("z08-x126-y125"),
                    32449,
                    32109,
                    8,
                    (32449, 32110),
                    1,
                    23986,
                ),
                (
                    "kv.ghostsilver_lantern_quest",
                    region!("z09-x128-y123"),
                    32825,
                    31664,
                    9,
                    (32826, 31664),
                    1,
                    23734,
                ),
                (
                    "kv.painting_of_agirl_quest",
                    region!("z10-x128-y123"),
                    32883,
                    31686,
                    10,
                    (32883, 31685),
                    1,
                    23732,
                ),
                (
                    "kv.imbuing_crystal_quest",
                    region!("z14-x129-y123"),
                    33026,
                    31662,
                    14,
                    (33026, 31663),
                    1,
                    24964,
                ),
            ] {
                let claim_started = std::time::Instant::now();
                let key =
                    oteryn_world_bundle::bundle::placement_key(-i8::try_from(z)?, x, y, ordinal)
                        .ok_or("quest reward placement")?;
                let outcome = claim_item(
                    &format!("oteryn:reward-claim.{suffix}"),
                    item,
                    key,
                    &activation,
                    &template,
                    NativeCrop {
                        bytes,
                        region: (z, x / 256, y / 256),
                        bounds: (
                            x.min(start.0),
                            y.min(start.1),
                            x.max(start.0) + 1,
                            y.max(start.1) + 1,
                        ),
                        start,
                        expected: Vec::new(),
                    },
                );
                eprintln!(
                    "QCX44_CLAIM {suffix} duration={:?} success={}",
                    claim_started.elapsed(),
                    outcome.is_ok()
                );
                if let Err(error) = outcome {
                    failures.push(format!("{suffix}: {error}"));
                }
            }
            if failures.is_empty() {
                Ok(())
            } else {
                Err(failures.join("\n").into())
            }
        })();
        eprintln!(
            "POSTGRES_CASE_COMPLETED qcx44-claims duration={:?} success={}",
            started.elapsed(),
            result.is_ok()
        );
        let cleanup = run(async |_| template.cleanup().await);
        result.and(cleanup)
    })
}

fn claim_item(
    claim: &str,
    item: u16,
    key: u64,
    activation: &CatalogueActivation,
    template: &MigratedDatabaseTemplate,
    crop: NativeCrop,
) -> TestResult {
    run(async |admin| {
        let harness = Harness::create_from_template(admin, "qcx44_claims", template).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) = activation;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let world = super::l6_world_pickups::native_crop(native, crop)?;
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
        owner.admit_quest_session(&admitted).await;
        deps.initialize(&owner).await;
        let outcome: TestResult = async {
            let actual =
                super::quest_callbacks::wire_uses(&owner, admitted, &[1, 2], &target).await?;
            if actual != [UseDisposition::Committed, UseDisposition::NothingToUse] {
                return Err(format!(
                    "{claim}: expected one commit and one refusal, got {actual:?}"
                )
                .into());
            }
            let bag = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("backpack")?;
            if bag.entries.len() != 1
                || bag.entries[0].item.quantity != 1
                || bag.entries[0].item.definition.production_key
                    != format!("oteryn:item.tibia.i{item}")
            {
                return Err(format!("{claim}: expected exactly one item {item}").into());
            }
            Ok(())
        }
        .await;
        drop(owner);
        drop(authority);
        drop(seal);
        let cleanup = harness.cleanup().await;
        outcome.and(cleanup)
    })
}
