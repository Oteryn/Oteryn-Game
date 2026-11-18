//! Two admitted players drive the source action through ordinary movement.
use super::appearance_change_postgres::observer_visibility_tests;
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
    reset,
};
use crate::content::{NativeEntryStart, native_cell_lookup::NativeMovementCollisionIndex};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId,
    MovementLocalPosition, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use crate::gameplay_transport::activity_runtime::{ActivityRuntimeAssets, ActivityTemplateKey};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority,
};
use crate::gameplay_transport::{SeamRooms, item_fence_of};
use crate::map::boot::{BootPins, boot};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::threatened_dreams::{
    ACCESS, KROAZUR_BOSS, KROAZUR_CREATURE, QUEST,
};
use oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::writer::*;

const ORIGIN: MovementLocalPosition = MovementLocalPosition {
    x: 33618,
    y: 32307,
    floor: 9,
};
const PEER: MovementLocalPosition = MovementLocalPosition {
    x: 33620,
    y: 32307,
    floor: 9,
};
const SOURCE: MovementLocalPosition = MovementLocalPosition {
    x: 33619,
    y: 32307,
    floor: 9,
};
const BOSS: MovementLocalPosition = MovementLocalPosition {
    x: 33591,
    y: 32305,
    floor: 10,
};
const ENTRY: MovementLocalPosition = MovementLocalPosition {
    x: 33591,
    y: 32315,
    floor: 10,
};
const EXIT: MovementLocalPosition = MovementLocalPosition {
    x: 33619,
    y: 32306,
    floor: 9,
};

#[test]
fn kroazur_step_in_requires_access_and_uses_shared_boss_timer() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("kroazur-step-in", move || {
        run(async move |_| {
            reset();
            let h = Harness::create_appearance(admin, "kroazur_step_in").await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    h.seed_appearance_observer().await?;
    observer_visibility_tests::admit(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = super::fixture::fence(1)?.runtime_scope.world_id();
    let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
    let (activation, native_pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let room = native_pin.qualified_room().clone();
    let native = activation
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native gameplay")?;
    let (bytes, pins) = source_tiles()?;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let map = boot(
        &bytes,
        &BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 33618,
                y: 32307,
                floor: -9,
            },
        },
        world,
        channel,
        |_| None,
    )
    .map_err(debug)?;
    let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
    let pin = |start: MovementLocalPosition| {
        ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            (start.x, start.y, start.floor),
        )
    };
    let make_view = |start: MovementLocalPosition| {
        room.bundle_view(
            cells.clone(),
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            NativeEntryStart {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .ok_or("Kroazur source map view")
    };
    let view = make_view(ORIGIN)?;
    let deps = Dependencies::with_content(
        pin(ORIGIN),
        room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = activation.active();
    owner.movement_cells = (&cells).into();
    owner.rooms = SeamRooms::new(Some(&view), Some(&view));
    owner.quest_catalogue = Some(std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));

    let peer_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let peer = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(peer_session).map_err(debug)?;
        runtime.commit_fresh_session(reservation).map_err(debug)?
    };
    let mut players = [
        deps.admitted,
        AdmittedSession {
            game_session_id: peer_session,
            runtime_actor: Some(peer),
            ..deps.admitted
        },
    ];
    let store = FreshAdmissionStore::from_root(h.root.clone());
    for (player, at) in players.iter_mut().zip([ORIGIN, PEER]) {
        let current = store.current_session_at(player.game_session_id).await?.0;
        player.first_entry = FirstEntryOutcome::Positioned;
        player.item_fence = item_fence_of(current);
        let fence = player.item_fence.ok_or("player fence")?;
        let account: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid"
        ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        player.controller = Some(ControllerBinding {
            account_id: account.try_into().map_err(|_| "account id")?,
            transport: current.current_transport().ok_or("player transport")?,
        });
        player.continuity.connection_generation = fence.connection_generation.get();
        player.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[CAPABILITY_BOSS_ENCOUNTER_V1],
        )
        .ok_or("boss capability")?;
        let actor = player.runtime_actor.ok_or("player actor")?;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_pinned_test_position(actor, at)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    player.game_session_id,
                    CharacterLease::new(
                        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        deps.initialize_player(&owner, actor, player.game_session_id)
            .await;
        owner
            .refresh_quest_session(player.game_session_id, true)
            .await;
        owner
            .activity_runtimes
            .remember(*player)
            .map_err(|()| "remember player")?;
    }
    owner
        .register_current_runtime()
        .await
        .map_err(|()| "register Channel")?;

    super::boss_activity::issue_content_and_reset(h, &pin(ORIGIN)).await?;
    let template = "oteryn:room.kroazur_test";
    sqlx::query(
        "INSERT INTO game_boss_room_catalogue
         (boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,
          minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,
          reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds)
         VALUES ($1,'content-1',$2,1,1,1,7200,600,$3,$4,'oteryn:reward.kroazur',true,'test-r1',$5,$6,$7,0)"
    ).bind(KROAZUR_BOSS).bind(KROAZUR_CREATURE).bind(serde_json::json!([cell(SOURCE)]))
        .bind(template).bind(serde_json::json!([cell(ENTRY)])).bind(cell(EXIT)).bind(cell(BOSS))
        .execute(&h.pool).await?;
    // Two ready allocations prove the refusal is room occupancy rather than
    // mere exhaustion of the allocator's only Instance.
    for tag in [88, 89] {
        let scope = super::activity_runtime::allocate_ready_instance(h, tag).await?;
        let RuntimeScopeRefV1::Instance { instance_id, .. } = scope else {
            return Err("instance scope".into());
        };
        let assignment = h
            .root
            .read_runtime_scope_assignment(scope)
            .await
            .map_err(debug)?
            .ok_or("assignment")?;
        let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
            return Err("bundle cells".into());
        };
        let bounds = [33571, 32285, 33611, 32325, 10, 10];
        let index = index
            .activity_region(
                bounds,
                Some(instance_id),
                [ENTRY.x, ENTRY.y, i32::from(ENTRY.floor)],
            )
            .ok_or("bounded Instance cells")?;
        let room_view = make_view(ENTRY)?
            .with_activity_bundle_index(index, bounds)
            .ok_or("Instance room view")?;
        let instance_pin = pin(ENTRY);
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            assignment.ownership_generation,
            assignment.source_revision,
            &assignment.decision_identity,
            4,
            instance_pin.clone(),
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .realize_native_qualification_spawn(instance_pin, KROAZUR_CREATURE, BOSS)
            .map_err(debug)?;
        let generation =
            ScopeOwnershipGeneration::new(assignment.ownership_generation).map_err(debug)?;
        let door = crate::world_runtime::bind_native_entry_door(
            room.door(),
            &crate::world_runtime::ScopeContentGenerationFence::for_test(
                scope,
                generation,
                crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                    .map_err(debug)?,
            ),
            scope,
            generation,
        )
        .map_err(debug)?;
        // Retain the owned view in the fixture's registry.
        owner
            .register_activity_runtime(
                ActivityTemplateKey {
                    key: template.into(),
                    revision: "test-r1".into(),
                },
                &assignment,
                runtime,
                door,
                ActivityRuntimeAssets {
                    movement_cells: std::sync::Arc::new(room_view.movement_cells().clone()).into(),
                    rooms: SeamRooms::owned(std::sync::Arc::new(room_view), false),
                },
            )
            .await
            .map_err(debug)?;
    }

    assert_eq!(
        step(&mut owner, players[0], StepDirection::East).await?,
        ORIGIN,
        "missing access"
    );
    assert_eq!(rooms(h).await?, 0);

    // Seed the already-completed prerequisite as imported persisted state.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for player in players {
        let fence = player.item_fence.ok_or("player fence")?;
        sqlx::query(
            "INSERT INTO game_character_quest_tracks
             (character_id,track_key,quest_key,value,committed_character_revision)
             VALUES (encode($1,'hex')::uuid,$2,$3,1,1)",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(ACCESS)
        .bind(QUEST)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let [first, second] = players;
    let _ = step(&mut owner, first, StepDirection::East).await?;
    let inside = owner.current_activity_connection(first);
    assert_ne!(inside.game_session_id, first.game_session_id);
    let instance = owner
        .resolve_runtime_view(inside)
        .await
        .map_err(|()| "resolve Instance")?
        .ok_or("Instance owner")?;
    assert_eq!(
        instance
            .runtime
            .lock()
            .await
            .read_actor_position(inside.runtime_actor.ok_or("inside actor")?)
            .map_err(debug)?
            .position(),
        ENTRY
    );
    drop(instance);
    assert_eq!(rooms(h).await?, 1);
    let (room_ms, cooldown_ms): (i64, i64) = sqlx::query_as(
        "SELECT a.deadline-a.opened_at,c.available_at-a.opened_at
         FROM game_boss_room_activities a JOIN game_character_boss_cooldowns c ON c.admission=a.admission"
    ).fetch_one(&h.pool).await?;
    assert_eq!((room_ms, cooldown_ms), (600_000, 7_200_000));
    assert_eq!(
        step(&mut owner, second, StepDirection::West).await?,
        PEER,
        "other character finds room occupied"
    );
    assert_eq!(rooms(h).await?, 1);

    // Advance the persisted room clock while preserving the source duration.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_boss_room_activities SET opened_at=opened_at-600001,deadline=deadline-600001",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE game_activity_runtime_entries SET deadline=deadline-600001")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    owner
        .activity_lifecycle_pass()
        .await
        .map_err(|()| "shared activity expiry")?;
    let returned = owner.current_activity_connection(inside);
    assert_ne!(returned.game_session_id, inside.game_session_id);
    let returned_actor = returned.runtime_actor.ok_or("returned actor")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(returned_actor)
            .map_err(debug)?
            .position(),
        EXIT
    );
    assert_eq!(
        step(&mut owner, returned, StepDirection::South).await?,
        EXIT,
        "the timeout leaves personal cooldown active"
    );
    let _ = step(&mut owner, second, StepDirection::West).await?;
    assert_ne!(
        owner.current_activity_connection(second).game_session_id,
        second.game_session_id
    );
    assert_eq!(rooms(h).await?, 2);
    Ok(())
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    player: AdmittedSession,
    direction: StepDirection,
) -> TestResult<MovementLocalPosition> {
    let actor = player.runtime_actor.ok_or("player actor")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("pacing clock")?;
    let (stepped, _) = owner
        .paced_step(actor, player.game_session_id, direction)
        .await;
    assert_eq!(stepped.disposition, StepDisposition::Moved);
    let observation = stepped.moved_to.ok_or("moved observation")?.actor_position;
    Ok(MovementLocalPosition {
        x: observation.x,
        y: observation.y,
        floor: observation.floor,
    })
}

async fn rooms(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_boss_room_activities")
            .fetch_one(&h.pool)
            .await?,
    )
}

fn cell(at: MovementLocalPosition) -> serde_json::Value {
    serde_json::json!({ "x": at.x, "y": at.y, "floor": at.floor })
}

fn source_tiles() -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    let manifest = Manifest {
        build_class: BuildClass::Production,
        format: FORMAT.into(),
        min_reader_version: VERSION,
        projection_class: "server".into(),
        compiler_version: "kroazur-step-test".into(),
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "content-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 33571,
            min_y: 32285,
            max_x: 33620,
            max_y: 32325,
            floors: vec![-10, -9],
        },
        palette: vec![PaletteEntry {
            key: "oteryn:terrain.tibia.i100".into(),
            family: Family::Terrain,
            id: 0,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
                blocks_projectile: None,
            }),
        }],
        subareas: Default::default(),
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let tile = |x, y, action| Tile {
        x,
        y,
        flags: 0,
        house: 0,
        zones: vec![],
        items: vec![Item {
            palette: 0,
            depth: 0,
            attrs: Attrs {
                action,
                ..Attrs::default()
            },
        }],
    };
    let bytes = write(
        &manifest,
        &[
            Sector {
                floor: -10,
                sx: 33591 / 32,
                sy: 32305 / 32,
                tiles: vec![tile(33591, 32305, None), tile(33591, 32315, None)],
            },
            Sector {
                floor: -9,
                sx: 33619 / 32,
                sy: 32306 / 32,
                tiles: vec![
                    tile(33619, 32306, None),
                    tile(33618, 32307, None),
                    tile(33619, 32307, Some(25029)),
                    tile(33620, 32307, None),
                ],
            },
        ],
        &Default::default(),
    )
    .map_err(debug)?;
    let digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    Ok((
        bytes,
        crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "content-1".into(),
            production: false,
        },
    ))
}
