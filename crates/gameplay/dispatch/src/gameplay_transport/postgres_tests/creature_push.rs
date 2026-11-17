//! PUSH-ROUTE-1: command 98 through the real serve router, composed owner and PostgreSQL session.
use super::appearance_change_postgres::observer_visibility_tests as observer;
use super::autowalk::{at, place, read, send};
use super::fixture::{Harness, debug, id, join_two, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::map::boot::{BootPins, boot};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::creature_push::{CreaturePushIntent, encode_creature_push_intent};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_protocol_oteryn::{MessageType, decode_command_result, decode_wire_envelope};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, DuplexStream};

const MOVED: u8 = 1;
const NOT_MOVABLE: u8 = 2;
const REJECTED: u8 = 3;
const NOT_POSSIBLE: u8 = 4;
const NOT_ENOUGH_ROOM: u8 = 5;
const OUT_OF_REACH: u8 = 6;
const EXHAUSTED: u8 = 7;

/// Grass 0..=12 x 0..=8 on floor 7 (Tibia-keyed, so every tile has spell facts); a wall at (5, 2); every tile with x >= 8 is a protection zone.
pub(super) fn bundle(trap: Option<u16>) -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self as writer, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
        Terrain, TerrainKind,
    };
    let terrain = |key: &str, id, kind, walkable: Option<bool>| PaletteEntry {
        key: key.into(),
        family: Family::Terrain,
        id,
        terrain: Some(Terrain {
            kind,
            walkable,
            ground_speed: walkable.map(|_| 150),
            blocks_projectile: None,
        }),
    };
    let manifest = Manifest {
        subareas: Default::default(),
        format: writer::FORMAT.into(),
        min_reader_version: writer::VERSION,
        projection_class: "server".into(),
        compiler_version: "engineering".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
        },
        palette: vec![
            terrain(
                "oteryn:terrain.tibia.i4526",
                0,
                TerrainKind::Ground,
                Some(true),
            ),
            terrain("oteryn:terrain.tibia.i1026", 1, TerrainKind::Wall, None),
            terrain(
                "oteryn:terrain.tibia.i5405",
                2,
                TerrainKind::Ground,
                Some(true),
            ),
            PaletteEntry {
                key: "oteryn:item.tibia.i3944".into(),
                family: Family::Item,
                id: 3,
                terrain: None,
            },
            PaletteEntry {
                key: "oteryn:item.tibia.i3945".into(),
                family: Family::Item,
                id: 4,
                terrain: None,
            },
            PaletteEntry {
                key: "oteryn:item.tibia.i12368".into(),
                family: Family::Item,
                id: 5,
                terrain: None,
            },
            PaletteEntry {
                key: "oteryn:item.tibia.i12369".into(),
                family: Family::Item,
                id: 6,
                terrain: None,
            },
        ],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let mut tiles = Vec::new();
    for y in 0..=8 {
        for x in 0..=12 {
            let mut palettes = vec![if (x, y) == (3, 5) { 2 } else { 0 }];
            if let Some(trap) = trap
                && (x, y) == (4, 2)
            {
                palettes.push(if trap == 12368 { 5 } else { 3 });
            }
            if (x, y) == (5, 2) {
                palettes.push(1);
            }
            tiles.push(Tile {
                x,
                y,
                // Tile flag bit 0 is the protection zone (map/spell_tiles.rs).
                flags: u32::from(x >= 8),
                house: 0,
                zones: vec![],
                items: palettes
                    .into_iter()
                    .map(|palette| Item {
                        palette,
                        depth: 0,
                        attrs: Attrs::default(),
                    })
                    .collect(),
            });
        }
    }
    let bytes = writer::write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles,
        }],
        &Default::default(),
    )
    .map_err(debug)?;
    let digest = writer::read(&bytes).map_err(debug)?.digest;
    Ok((
        bytes,
        crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: false,
        },
    ))
}

#[test]
fn creature_push_real_dispatch_moves_and_refuses_on_postgres() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            // One qualification spawn per runtime: the unpushable profile gets its own run.
            scenario("creature_push_np", true, None)?;
            scenario("creature_push", false, None)
        })?
        .join()
        .map_err(|_| "creature push e2e panicked")?
        .map_err(Into::into)
}

fn scenario(seed: &'static str, unpushable: bool, trap: Option<u16>) -> Result<(), String> {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, seed, true, false).await?;
        prepare_current_session(&h).await?;
        // The pushed player is a second durable session (character 101, session 110).
        h.seed_pvp_target().await?;
        observer::admit(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let active = controller.active().ok_or("active")?;
        deps.room = pin.qualified_room().clone();
        let (bytes, pins) = bundle(trap)?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: TilePos {
                    x: 2,
                    y: 2,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(match key {
                        "oteryn:item.tibia.i3944" => 4,
                        "oteryn:item.tibia.i3945" => 5,
                        "oteryn:item.tibia.i12368" => 6,
                        "oteryn:item.tibia.i12369" => 7,
                        _ => return None,
                    })?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let combat_room = deps.room.bundle_view(
            cells.clone(),
            pin.frame_binding().digest(),
            world.map_revision_digest(),
            crate::content::NativeEntryStart { x: 2, y: 2, floor: 7 },
        ).ok_or("push combat room")?;
        let channel_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            world.map_revision_digest(),
            (2, 2, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            // The pusher, one creature and one pushed player.
            3,
            channel_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id).await?.0;
        runtime.bind_attacker_lease(
            deps.actor, deps.admitted.game_session_id, current.current_character_lease(),
        ).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, at(2, 2))
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&combat_room));
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 13, 82])
                .ok_or("selection")?;
        assert_eq!(
            admitted.continuity.selected_capabilities.as_slice(),
            &[4, 6, 13, 82]
        );
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let client_flow = async {
            let result = if let Some(trap) = trap {
                relocation_tiles_scenario(&mut client, &owner, admitted, trap).await
            } else if unpushable {
                unpushable_scenario(&mut client, &owner).await
            } else {
                client_scenario(&mut client, &deps, &owner, admitted).await
            };
            client.shutdown().await?;
            result
        };
        let (end, result) = join_two(
            serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS),
            client_flow,
        )
        .await;
        result?;
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
    .map_err(|error| error.to_string())
}

async fn drowning(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<bool> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .get(&runtime, actor, session)
        .ok_or("pushed player spell state")?;
    Ok(state
        .owned_conditions()
        .instances()
        .iter()
        .any(|condition| {
            crate::gameplay_transport::paced_movement::underwater_drown::is_environment_drown(
                condition.provenance(),
            )
        }))
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .borrow_movement_position()
        .read(actor)
        .map_err(debug)?
        .position())
}

async fn reference(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult<EntityRef> {
    let runtime = owner.runtime.lock().await;
    let entities = runtime.visible_entities();
    let generation = entities
        .players
        .iter()
        .chain(&entities.creatures)
        .find(|entry| entry.actor == actor)
        .ok_or("visible")?
        .generation;
    Ok(EntityRef {
        identity: actor.placement_identity(),
        generation,
    })
}

async fn spawn(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    key: &str,
    at: MovementLocalPosition,
) -> TestResult<ExactActorRef> {
    let mut runtime = owner.runtime.lock().await;
    let pin = runtime.content_pin().clone();
    Ok(runtime
        .realize_native_qualification_spawn(pin, key, at)
        .map_err(debug)?
        .actor)
}

/// Sends one push and returns its disposition, reading past spatial updates.
async fn push(
    client: &mut DuplexStream,
    id: u64,
    target: EntityRef,
    to: MovementLocalPosition,
) -> TestResult<u8> {
    let payload = encode_creature_push_intent(&CreaturePushIntent {
        target,
        destination: ActorPosition {
            x: to.x,
            y: to.y,
            floor: to.floor,
        },
    })
    .map_err(debug)?;
    raw(client, id, &payload).await
}

async fn raw(client: &mut DuplexStream, id: u64, payload: &[u8]) -> TestResult<u8> {
    send(client, id, 98, payload).await?;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, id);
            let [8, disposition] = result.payload[..] else {
                return Err(format!("result payload {:?}", result.payload).into());
            };
            return Ok(disposition);
        }
    }
}

/// Waits for the first snapshot and installs the content's companion policies.
async fn prepare(
    client: &mut DuplexStream,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    loop {
        let frame = read(client).await?;
        if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    let mut runtime = owner.runtime.lock().await;
    owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    Ok(())
}

async fn unpushable_scenario(
    client: &mut DuplexStream,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    prepare(client, owner).await?;
    let marauder = spawn(owner, "oteryn:creature.pirate_marauder", at(3, 2)).await?;
    let marauder_ref = reference(owner, marauder).await?;
    // Canary monster.cpp:275: a profile with `pushable: false` is never moved.
    assert_eq!(push(client, 1, marauder_ref, at(4, 2)).await?, NOT_MOVABLE);
    assert_eq!(position(owner, marauder).await?, at(3, 2));
    Ok(())
}

async fn client_scenario(
    client: &mut DuplexStream,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    prepare(client, owner).await?;
    let troll = spawn(owner, "oteryn:creature.troll", at(3, 2)).await?;
    let troll_ref = reference(owner, troll).await?;
    // A pushable adjacent creature moves exactly one cell.
    assert_eq!(push(client, 1, troll_ref, at(4, 2)).await?, MOVED);
    assert_eq!(position(owner, troll).await?, at(4, 2));
    // PUSH-RL-03: the next push inside 1000 ms is refused before the owner reads anything.
    assert_eq!(push(client, 2, troll_ref, at(4, 3)).await?, EXHAUSTED);
    assert_eq!(position(owner, troll).await?, at(4, 2));
    tokio::time::sleep(Duration::from_millis(1100)).await;
    // The troll is now two cells away from the pusher.
    assert_eq!(push(client, 3, troll_ref, at(4, 3)).await?, NOT_POSSIBLE);
    let player_session = admitted;
    place(owner, player_session, at(3, 3)).await?;
    // The destination must neighbour the pushed creature.
    assert_eq!(push(client, 4, troll_ref, at(6, 2)).await?, OUT_OF_REACH);
    // A wall and an occupied cell (the pusher's own) refuse with no room.
    assert_eq!(push(client, 5, troll_ref, at(5, 2)).await?, NOT_ENOUGH_ROOM);
    assert_eq!(push(client, 6, troll_ref, at(3, 3)).await?, NOT_ENOUGH_ROOM);
    // Self, stale generations and malformed bytes never move anything.
    assert_eq!(
        push(client, 7, reference(owner, actor).await?, at(3, 4)).await?,
        NOT_POSSIBLE
    );
    let stale = EntityRef {
        generation: troll_ref.generation + 1,
        ..troll_ref
    };
    assert_eq!(push(client, 8, stale, at(4, 3)).await?, NOT_POSSIBLE);
    assert_eq!(raw(client, 9, &[0x0a, 0x00]).await?, REJECTED);
    assert_eq!(position(owner, troll).await?, at(4, 2));
    assert_eq!(position(owner, actor).await?, at(3, 3));

    // A second real player is pushed by its own ordinary paced step.
    let target = observer::initialize(deps, owner).await?;
    let mut target_admitted = AdmittedSession {
        runtime_actor: Some(target),
        game_session_id: GameSessionId::decode(&id(110)).map_err(debug)?,
        ..admitted
    };
    owner.admit_progression_session(&mut target_admitted).await;
    place(owner, target_admitted, at(3, 4)).await?;
    let target_ref = reference(owner, target).await?;
    assert_eq!(push(client, 10, target_ref, at(3, 5)).await?, MOVED);
    assert_eq!(position(owner, target).await?, at(3, 5));
    assert!(drowning(owner, target, target_admitted.game_session_id).await?);
    tokio::time::sleep(Duration::from_millis(1100)).await;
    place(owner, player_session, at(3, 4)).await?;
    assert_eq!(push(client, 11, target_ref, at(3, 6)).await?, MOVED);
    assert_eq!(position(owner, target).await?, at(3, 6));
    assert!(!drowning(owner, target, target_admitted.game_session_id).await?);
    tokio::time::sleep(Duration::from_millis(1100)).await;
    // Canary game.cpp:1865: out of a protection zone only onto another protection zone.
    place(owner, target_admitted, at(8, 5)).await?;
    place(owner, player_session, at(7, 4)).await?;
    assert_eq!(push(client, 12, target_ref, at(7, 5)).await?, NOT_POSSIBLE);
    assert_eq!(position(owner, target).await?, at(8, 5));
    assert_eq!(push(client, 13, target_ref, at(8, 6)).await?, MOVED);
    assert_eq!(position(owner, target).await?, at(8, 6));
    Ok(())
}

// Crystal 201189f8 data/scripts/movements/trap.lua:12-44,59-69: forced movement
// has the same creature StepIn damage/transform and StepOut reopening as walking.
#[test]
fn reaudit_relocation_tiles_monster_push_trap_entry_exit_postgres() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("srv_b3_relocation_tiles", false, Some(3944)))?
        .join()
        .map_err(|_| "relocation tiles e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn reaudit_relocation_tiles_monster_push_wolf_capture_postgres() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("srv_b3_relocation_wolf", false, Some(12368)))?
        .join()
        .map_err(|_| "relocation wolf e2e panicked")?
        .map_err(Into::into)
}

async fn relocation_tiles_scenario(
    client: &mut DuplexStream,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    trap: u16,
) -> TestResult {
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::creature_push::CreaturePushDisposition;
    prepare(client, owner).await?;
    let actor = admitted.runtime_actor.ok_or("pusher")?;
    let key = if trap == 12368 {
        "oteryn:creature.starving_wolf"
    } else {
        "oteryn:creature.troll"
    };
    let troll = spawn(owner, key, at(3, 2)).await?;
    let target = reference(owner, troll).await?;
    let health = owner
        .runtime
        .lock()
        .await
        .live_creature_health(troll)
        .ok_or("health")?;
    let map = owner.bundle_map().ok_or("bundle")?;
    let trap_pos = TilePos {
        x: 4,
        y: 2,
        floor: -7,
    };
    let appearance = || -> TestResult<u16> {
        Ok(map
            .facts
            .base_entry(trap_pos, 1, if trap == 12368 { 5 } else { 3 })
            .ok_or("trap")?
            .appearance_id)
    };
    let intent = |destination: MovementLocalPosition| CreaturePushIntent {
        target,
        destination: ActorPosition {
            x: destination.x,
            y: destination.y,
            floor: destination.floor,
        },
    };
    assert_eq!(appearance()?, trap);
    assert_eq!(
        owner
            .push_creature_now(actor, admitted.game_session_id, intent(at(4, 2)))
            .await,
        CreaturePushDisposition::Moved
    );
    if trap == 12368 {
        // Crystal trap.lua:39-42: capture removes the wolf and creates item 12369.
        assert!(
            !owner.runtime.lock().await.contains_live_creature(troll),
            "a pushed Starving Wolf must be captured"
        );
        let stack = crate::map::view::compose_tile(&map.overlay, &map.facts, trap_pos)
            .map_err(debug)?
            .ok_or("wolf trap tile")?;
        assert!(
            stack
                .entries
                .iter()
                .any(|entry| entry.facts.appearance_id == 12369)
        );
        assert_eq!(appearance()?, 12368);
        return Ok(());
    }
    assert_eq!(position(owner, troll).await?, at(4, 2));
    assert_eq!(
        appearance()?,
        3945,
        "a pushed monster must close the trap on entry"
    );
    let after = owner
        .runtime
        .lock()
        .await
        .live_creature_health(troll)
        .ok_or("health after entry")?;
    let policy = owner
        .runtime
        .lock()
        .await
        .companion_snapshot(troll)
        .map_err(debug)?
        .state
        .policy;
    assert_eq!(policy.definition_key, "oteryn:creature.troll");
    let mitigation = policy.mitigation.ok_or("Troll mitigation")?;
    assert_eq!((mitigation.numerator, mitigation.denominator), (8, 25));
    let earth = policy
        .resistances
        .iter()
        .find(|resistance| resistance.damage_type == "earth")
        .ok_or("Troll Earth resistance")?;
    assert_eq!(
        (earth.percent.numerator, earth.percent.denominator),
        (-10, 1)
    );
    // Crystal trap 3944 deals 15..30 Earth damage. The pinned Troll's 0.32%
    // mitigation truncates each positive raw hit to raw - 1; its 10% Earth
    // weakness then rounds that result. Check the exact possible one-hit set.
    let expected_hits: Vec<u64> = (15..=30).map(|raw| ((raw - 1) * 11 + 5) / 10).collect();
    assert!(
        expected_hits.contains(&(health - after)),
        "one trap hit: {health} -> {after}"
    );
    place(owner, admitted, at(3, 3)).await?;
    assert_eq!(
        owner
            .push_creature_now(actor, admitted.game_session_id, intent(at(5, 2)))
            .await,
        CreaturePushDisposition::NotEnoughRoom
    );
    assert_eq!(
        appearance()?,
        3945,
        "refused movement cannot reopen the trap"
    );
    assert_eq!(
        owner.runtime.lock().await.live_creature_health(troll),
        Some(after)
    );
    assert_eq!(
        owner
            .push_creature_now(actor, admitted.game_session_id, intent(at(4, 3)))
            .await,
        CreaturePushDisposition::Moved
    );
    assert_eq!(position(owner, troll).await?, at(4, 3));
    assert_eq!(
        appearance()?,
        3944,
        "a pushed monster must reopen the trap on exit"
    );
    assert_eq!(
        owner.runtime.lock().await.live_creature_health(troll),
        Some(after)
    );
    Ok(())
}
