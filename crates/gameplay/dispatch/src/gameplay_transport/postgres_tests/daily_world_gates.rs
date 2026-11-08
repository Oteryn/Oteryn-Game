//! W05b selected native attributes reach the production step event and sealed movement owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::map::{boot, overlay::TilePos};
use oteryn_world_bundle::{bundle, sector};
use std::collections::BTreeMap;

// Only the immutable admitted content artifact is shared. Every scene still creates its own
// SQL database, Character authority, Node holder, session and current eligibility reads.
type NativeArtifact = (
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
);
fn native_artifact(world: crate::foundation::WorldId) -> TestResult<&'static NativeArtifact> {
    static ARTIFACT: std::sync::OnceLock<Result<NativeArtifact, String>> =
        std::sync::OnceLock::new();
    let artifact = ARTIFACT
        .get_or_init(|| {
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| -> Box<dyn std::error::Error> { error.clone().into() })?;
    assert_eq!(
        artifact
            .1
            .qualified_room()
            .movement_cells()
            .scope()
            .world_id,
        world
    );
    Ok(artifact)
}

fn world(
    action: Option<u16>,
    unique: Option<u16>,
    layer: &str,
    active: bool,
) -> TestResult<boot::BundleWorld> {
    let (bytes, mut pins) = boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.palette.push(bundle::PaletteEntry {
        key: "oteryn:terrain.tibia.i100".into(),
        family: bundle::Family::Terrain,
        id: 1,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
            blocks_projectile: None,
        }),
    });
    let mut cells = BTreeMap::new();
    for p in [
        TilePos {
            x: 100,
            y: 100,
            floor: -7,
        },
        TilePos {
            x: 101,
            y: 100,
            floor: -7,
        },
        TilePos {
            x: 33497,
            y: 32616,
            floor: -8,
        },
        TilePos {
            x: 33290,
            y: 31786,
            floor: -13,
        },
        TilePos {
            x: 32265,
            y: 32164,
            floor: -7,
        },
        TilePos {
            x: 33215,
            y: 32273,
            floor: -7,
        },
        TilePos {
            x: 33475,
            y: 32641,
            floor: -10,
        },
    ] {
        cells.insert(
            (p.floor, p.y, p.x),
            vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }],
        );
    }
    cells
        .get_mut(&(-7, 100, 101))
        .ok_or("gate")?
        .push(sector::Item {
            palette: 1,
            depth: 0,
            attrs: sector::Attrs {
                action,
                unique,
                ..Default::default()
            },
        });
    let mut grouped: BTreeMap<(i8, u16, u16), Vec<sector::Tile>> = BTreeMap::new();
    for ((floor, y, x), items) in cells {
        grouped
            .entry((floor, y / 32, x / 32))
            .or_default()
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
    }
    let mut sectors = grouped
        .into_iter()
        .map(|((floor, sy, sx), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    sectors.sort_by_key(bundle::order);
    data.manifest.world = bundle::Extent {
        min_x: 100,
        min_y: 100,
        max_x: 33498,
        max_y: 32642,
        floors: vec![-13, -10, -8, -7],
    };
    let gate_sector = sectors
        .iter()
        .find(|s| s.floor == -7 && s.sx == 3 && s.sy == 3)
        .ok_or("sector")?
        .clone();
    data.layers = vec![oteryn_world_bundle::layers::Layer {
        key: layer.into(),
        palette: data.manifest.palette.clone(),
        sectors: vec![gate_sector],
    }];
    let bytes = oteryn_world_bundle::writer::write_with_layers(
        &data.manifest,
        &sectors,
        &Default::default(),
        &Default::default(),
        &data.layers,
    )?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let checked = boot::check(
        bytes,
        boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 100,
                y: 100,
                floor: -7,
            },
        },
    )
    .map_err(debug)?;
    let checked = if active {
        checked.compose_layers(&[layer]).map_err(debug)?
    } else if layer.starts_with("fury_gates/") {
        checked
            .compose_layers_with_exits(&[], &[layer])
            .map_err(debug)?
    } else {
        checked
    };
    let world = checked
        .boot(
            crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            |_| None,
        )
        .map_err(debug)?;
    if !active && layer.starts_with("fury_gates/") {
        assert!(world.base().active_layers().is_empty());
        assert_eq!(world.base().exit_layers(), &[layer.to_owned()]);
    }
    Ok(world)
}

fn scenario(
    action: Option<u16>,
    unique: Option<u16>,
    layer: &str,
    active: bool,
    expected: MovementLocalPosition,
    eligible: bool,
) -> TestResult {
    let world = world(action, unique, layer, active)?;
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "daily_world_gates", eligible, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) = native_artifact(deps.admitted.world_id)?;
        let generation = controller.active().ok_or("generation")?;
        if eligible {
            super::tutorial_vitals_fixture::publish_native_activation(&h, activated).await?;
        }
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        if eligible {
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
        }
        let premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (100, 100, 7),
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
        let session = deps.admitted.game_session_id;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        if eligible {
            owner.premium_coordinator = Some(&premium);
            owner.admit_premium(id(40), session);
            super::atelier::fixture::eligible(&h.pool).await?;
            let purchase = super::atelier::fixture::prepare(&h).await?;
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "promotion fence")?
                .ok_or("promotion fence")?;
            let promoted = owner
                .revision_sequencer
                .acquire(fence.character_id)
                .await
                .commit_npc_purchase(&h.root, &authority, &h.node, fence, purchase, &premium)
                .await
                .map_err(debug)?;
            assert!(matches!(
                promoted,
                crate::durability::npc_purchase::PurchaseOutcome::Committed(_)
            ));
            super::tutorial_vitals_fixture::native_asset_revisions(&h, generation).await?;
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "equipment fence")?
                .ok_or("equipment fence")?;
            let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
            h.root
                .initialize_admission_equipment(
                    &authority,
                    &h.node,
                    &fence.item_fence(),
                    generation.identity().server_artifact_digest(),
                    fresh.operation(),
                )
                .await
                .map_err(debug)?;
            let crate::gameplay_transport::CastFactsLoad::Ready { facts, .. } =
                crate::gameplay_transport::load_character_cast_facts(
                    &h.root,
                    &authority,
                    fence.character_id,
                )
                .await
            else {
                return Err("SQL cast facts".into());
            };
            {
                let runtime = deps.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                assert!(
                    states
                        .initialize(
                            &runtime,
                            deps.actor,
                            session,
                            facts,
                            (0, 0),
                            owner.owner_now()
                        )
                        .is_some()
                );
            }
            assert!(
                owner
                    .refresh_and_apply_spell_access(deps.actor, session)
                    .await
                    .premium_gate()
                    .premium_current()
            );
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states
                .get(&runtime, deps.actor, session)
                .ok_or("current state")?;
            assert_eq!(state.character_facts().level, 60);
            assert!(state.wheel_promoted());
        } else {
            deps.initialize(&owner).await;
        }
        let (outcome, duration) = owner
            .ordinary_paced_step(
                deps.actor,
                session,
                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
                Some(origin),
                None,
            )
            .await;
        assert_eq!(
            outcome.disposition,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
        );
        assert!(duration.is_some());
        let result = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?;
        assert_eq!(result.position(), expected);
        assert_eq!(
            outcome
                .moved_to
                .ok_or("moved observation")?
                .actor_position
                .x,
            expected.x
        );
        eprintln!(
            "W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED action={action:?} unique={unique:?} active={active} eligible={eligible} destination={expected:?}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn w05b_daily_gate_real_step_uses_selected_native_ids_and_refuses_unqualified_fury() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("daily-native-gates", || {
        scenario(
            None,
            Some(64001),
            "nightmare_isle/darashia-north",
            true,
            MovementLocalPosition {
                x: 33497,
                y: 32616,
                floor: 8,
            },
            false,
        )?;
        scenario(
            None,
            Some(64001),
            "nightmare_isle/darashia-north",
            false,
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            false,
        )?;
        scenario(
            None,
            Some(35020),
            "nightmare_isle/darashia-north",
            true,
            MovementLocalPosition {
                x: 33215,
                y: 32273,
                floor: 7,
            },
            false,
        )?;
        scenario(
            Some(64103),
            None,
            "nightmare_isle/darashia-north",
            true,
            MovementLocalPosition {
                x: 33475,
                y: 32641,
                floor: 10,
            },
            false,
        )?;
        scenario(
            Some(9715),
            None,
            "fury_gates/thais",
            true,
            MovementLocalPosition {
                x: 32265,
                y: 32164,
                floor: 7,
            },
            false,
        )?;
        scenario(
            Some(9710),
            None,
            "fury_gates/thais",
            true,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            false,
        )?;
        scenario(
            Some(9710),
            None,
            "fury_gates/thais",
            true,
            MovementLocalPosition {
                x: 33290,
                y: 31786,
                floor: 13,
            },
            true,
        )
    })
}

#[test]
fn w05b_fury_offday_exit_keeps_city_return_without_enabling_the_entrance() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("daily-offday-exit", || {
        scenario(
            Some(9715),
            None,
            "fury_gates/thais",
            false,
            MovementLocalPosition {
                x: 32265,
                y: 32164,
                floor: 7,
            },
            false,
        )?;
        scenario(
            Some(9710),
            None,
            "fury_gates/thais",
            false,
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            true,
        )
    })
}
