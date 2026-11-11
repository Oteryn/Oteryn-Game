//! Real Book of Wisdom command-2 acquisition, durable repeat and replay.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::wire_uses;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::RewardClaimMintRequest;
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};

use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn viola_book_postgres_real_use_repeat_replay_refusal_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("viola-book", scenario)?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED viola_book_postgres_real_use_repeat_replay_refusal_and_fences duration={:?}",
        started.elapsed()
    );
    Ok(())
}
fn definition(id: u16) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    }
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
            super::death_ml_loss_composed::production_generation_with_pin(
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
pub(super) async fn free_one(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    command_id: u64,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("bag")?;
    let filler = bag
        .entries
        .iter()
        .find(|entry| entry.item.definition.production_key == "oteryn:item.tibia.i3031")
        .ok_or("filler")?;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let permit = owner.spell_lane_permit().await.ok_or("lane permit")?;
    let moved = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            CommandRef::new(
                fence.game_session_id,
                CommandId::new(command_id).map_err(debug)?,
            ),
            &NativeItemWeights(native),
            crate::durability::item_loop_move::LoopMoveRequest {
                item: filler.item.item_instance_id,
                count: None,
                pickup_cell: None,
                destination: crate::durability::item_loop_move::LoopDestination::Ground(
                    crate::durability::item_loop_move::LoopGround {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                32613, 32419, -7,
                            ),
                        map_revision: crate::map::overlay::map_revision(
                            owner.bundle_map().ok_or("bundle map")?.overlay.base(),
                        ),
                        content_revision: revision,
                        placement_context: frame.to_vec(),
                    },
                ),
            },
        )
        .await
        .map_err(debug)?;
    assert!(moved.is_ok(), "free capacity: {moved:?}");
    Ok(())
}

/// Exact committed source stack; the marker points one tile south of the chest.
pub(super) fn source_world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.clear();
    for (id, appearance) in [16485, 26996, 2472].into_iter().enumerate() {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(id)?,
            terrain: Some(bundle::Terrain {
                blocks_projectile: None,
                kind: if id == 0 {
                    bundle::TerrainKind::Ground
                } else {
                    bundle::TerrainKind::Common
                },
                walkable: (id == 0).then_some(true),
                ground_speed: (id == 0).then_some(150),
            }),
        });
    }
    data.manifest.world = bundle::Extent {
        min_x: 32570,
        min_y: 32419,
        max_x: 32615,
        max_y: 32509,
        floors: vec![-7],
    };
    let mut tiles = Vec::new();
    for y in 32419..=32420 {
        for x in 32612..=32614 {
            let palettes = if (x, y) == (32614, 32419) {
                vec![0, 1, 2]
            } else {
                vec![0]
            };
            tiles.push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items: palettes
                    .into_iter()
                    .map(|palette| sector::Item {
                        palette,
                        depth: 0,
                        attrs: sector::Attrs::default(),
                    })
                    .collect(),
            });
        }
    }
    // Retain Viola's real source placement, with a walkable approach in her sector.
    for y in 32506..=32508 {
        for x in 32570..=32572 {
            tiles.push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items: vec![sector::Item {
                    palette: 0,
                    depth: 0,
                    attrs: sector::Attrs::default(),
                }],
            });
        }
    }
    let mut grouped = std::collections::BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for tile in tiles {
        grouped
            .entry((tile.y / 32, tile.x / 32))
            .or_default()
            .push(tile);
    }
    let sectors = grouped
        .into_iter()
        .map(|((sy, sx), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor: -7,
                sx,
                sy,
                tiles,
            }
        })
        .collect::<Vec<_>>();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 32613,
                y: 32419,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

pub(super) fn target(world: &crate::map::boot::BundleWorld) -> TestResult<WorldObjectTarget> {
    let key =
        oteryn_world_bundle::bundle::placement_key(-7, 32614, 32419, 2).ok_or("book placement")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
}
