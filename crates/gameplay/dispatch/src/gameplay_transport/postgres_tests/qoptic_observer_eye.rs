//! A committed STEP drives a second admitted actor on the west field.
use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

#[test]
fn qoptic_observer_eye_dispatch_moves_only_the_paired_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    observer_eye_scenario(admin, Scenario::Movement)
}

#[test]
fn qoptic_observer_eye_submerged_platform_ejects_the_west_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    observer_eye_scenario(admin, Scenario::Fall)
}

#[test]
fn qoptic_observer_eye_deadline_ejects_both_players_without_another_step() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    observer_eye_scenario(admin, Scenario::Timeout)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    Movement,
    Fall,
    Timeout,
}

fn observer_eye_scenario(admin: String, scenario: Scenario) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run_async(async move |_| {
        let h =
            Harness::create_with_admission_seed(admin, "qoptic_observer_eye", false, false).await?;
        prepare_current_session(&h).await?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        use crate::world_party_runtime_postgres_cases::{
            FreshPlayerOwners, committed_fresh_player_owned,
        };
        let partner_authority = committed_fresh_player_owned(
            &FreshPlayerOwners {
                root: h.root.clone(),
                node: h.node.clone(),
                recovery: &h.recovery,
            },
            60,
            62,
            70,
            now,
        )
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let fixture = Dependencies::new()?;
        let world = platform_world(
            fixture.admitted.world_id,
            fixture.admitted.channel_id,
            scenario,
        )?;
        let cells = world
            .movement_cells(fixture.room.movement_cells())
            .map_err(debug)?;
        let start = crate::content::NativeEntryStart {
            x: 32782,
            y: 32912,
            floor: 14,
        };
        let room = fixture
            .room
            .bundle_view(
                cells.clone(),
                world.frame_binding_digest(fixture.admitted.world_id),
                world.map_revision_digest(),
                start,
            )
            .ok_or("Observer Eye bundle room")?;
        let pin = ChannelContentPin::from_activation(
            fixture.admitted.world_id,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let mut deps = Dependencies::with_content(pin, room.clone(), fixture.spells)?;
        deps.combat_room = Some(room);
        let controller_authority =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let partner_session = partner_authority.current_game_session_id();
        let partner_start = MovementLocalPosition {
            x: 32777,
            y: 32907,
            floor: 14,
        };
        let partner = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    deps.admitted.game_session_id,
                    controller_authority.current_character_lease(),
                )
                .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(partner_session)
                .map_err(debug)?;
            let partner = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    partner,
                    partner_session,
                    partner_authority.current_character_lease(),
                )
                .map_err(debug)?;
            runtime
                .initialize_pinned_test_position(partner, partner_start)
                .map_err(debug)?;
            partner
        };
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        deps.initialize_player(&owner, partner, partner_session)
            .await;
        for (command, direction, controller_y, partner_y) in [
            (1, StepDirection::South, 32913, 32907),
            (2, StepDirection::South, 32914, 32908),
            (3, StepDirection::South, 32915, 32908),
            // Re-entering the topaz alone must not reuse the old partner.
            (4, StepDirection::North, 32914, 32908),
            (5, StepDirection::North, 32913, 32908),
            (6, StepDirection::South, 32914, 32908),
        ]
        .into_iter()
        .take(match scenario {
            Scenario::Movement => 6,
            Scenario::Fall => 2,
            Scenario::Timeout => 1,
        }) {
            let (result, _) = owner
                .paced_step(deps.actor, deps.admitted.game_session_id, direction)
                .await;
            assert_eq!(result.disposition, StepDisposition::Moved);
            if command == 1 && scenario == Scenario::Movement {
                let map = owner.bundle_map().ok_or("platform bundle")?;
                let position = crate::map::overlay::TilePos {
                    x: 32775,
                    y: 32908,
                    floor: -14,
                };
                let last_item = || -> TestResult<u16> {
                    crate::map::view::compose_tile(&map.overlay, &map.facts, position)
                        .map_err(debug)?
                        .and_then(|tile| tile.entries.last().map(|item| item.facts.appearance_id))
                        .ok_or_else(|| "fixture platform".into())
                };
                let now = owner.owner_now().get();
                let mut runtime = deps.runtime.lock().await;
                assert_eq!(last_item()?, 24912);
                owner.advance_observer_eye(&mut runtime, map, now).await;
                assert_eq!(last_item()?, 24912);
                owner
                    .advance_observer_eye(&mut runtime, map, now + 2_000_000)
                    .await;
                assert_eq!(last_item()?, 24889);
                owner
                    .advance_observer_eye(&mut runtime, map, now + 2_000_000)
                    .await;
                assert_eq!(last_item()?, 24889);
            }
            let runtime = deps.runtime.lock().await;
            assert_eq!(
                runtime
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    x: start.x,
                    y: controller_y,
                    floor: start.floor
                },
                "controller STEP {command}",
            );
            assert_eq!(
                runtime
                    .read_actor_position(partner)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    y: if scenario == Scenario::Fall && command == 2 {
                        32904
                    } else {
                        partner_y
                    },
                    ..partner_start
                },
                "partner after controller STEP {command}",
            );
        }
        if scenario == Scenario::Timeout {
            let map = owner.bundle_map().ok_or("timeout bundle")?;
            let now = owner.owner_now().get();
            let mut runtime = deps.runtime.lock().await;
            owner
                .advance_observer_eye(&mut runtime, map, now + 299_000_000)
                .await;
            assert_eq!(
                runtime
                    .read_actor_position(partner)
                    .map_err(debug)?
                    .position(),
                partner_start
            );
            owner
                .advance_observer_eye(&mut runtime, map, now + 300_000_000)
                .await;
            assert_eq!(
                runtime
                    .read_actor_position(partner)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    y: 32904,
                    ..partner_start
                }
            );
            assert_eq!(
                runtime
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    x: 32784,
                    y: 32904,
                    floor: 14
                }
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn platform_world(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    scenario: Scenario,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::{boot, overlay::TilePos};
    use oteryn_world_bundle::{
        bundle,
        sector::{Attrs, Item, Tile},
        writer,
    };
    let (bytes, mut pins) = boot::tests::bundle();
    let mut data = bundle::read(&bytes).map_err(debug)?;
    data.manifest.world = writer::Extent {
        min_x: 32775,
        max_x: 32786,
        min_y: 32904,
        max_y: 32919,
        floors: vec![-14],
    };
    data.manifest.palette.truncate(1);
    data.manifest.palette[0].key = "oteryn:item.tibia.i100".into();
    for (index, id) in [24912, 24889, 24898, 24899].into_iter().enumerate() {
        data.manifest.palette.push(writer::PaletteEntry {
            key: format!("oteryn:item.tibia.i{id}"),
            family: writer::Family::Item,
            id: u32::try_from(index)? + 1,
            terrain: None,
        });
    }
    let tiles = (32904..=32918)
        .flat_map(|y| {
            (32775..=32785).map(move |x| {
                let mut items = vec![Item {
                    palette: 0,
                    depth: 0,
                    attrs: Attrs::default(),
                }];
                if x == 32777 && y == 32908 && scenario == Scenario::Fall {
                    items.push(Item {
                        palette: 1,
                        depth: 0,
                        attrs: Attrs::default(),
                    });
                }
                if x == 32775 && (32908..=32911).contains(&y) {
                    items.push(Item {
                        palette: u32::from(y - 32907),
                        depth: 0,
                        attrs: Attrs::default(),
                    });
                }
                Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items,
                }
            })
        })
        .collect();
    data.sectors = vec![writer::Sector {
        floor: -14,
        sx: 32775 / 32,
        sy: 32904 / 32,
        tiles,
    }];
    let bytes = writer::write(&data.manifest, &data.sectors, &data.spawns).map_err(debug)?;
    pins.digest = bundle::read(&bytes).map_err(debug)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: TilePos {
                x: 32782,
                y: 32912,
                floor: -14,
            },
        },
        world,
        channel,
        |key| {
            let id = key.rsplit_once(".i")?.1.parse::<u32>().ok()?;
            Some(crate::map::facts::ItemDefinition {
                reference: std::num::NonZeroU32::new(id + 1)?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        },
    )
    .map_err(debug)
    .map_err(Into::into)
}
