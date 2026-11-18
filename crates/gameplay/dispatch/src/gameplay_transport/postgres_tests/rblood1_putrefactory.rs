use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::{FirstEntryOutcome, UseCommand};
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

// Quest definition hash from the fetched rblood1 base, with the original non-spending gate.
const BASE_QUEST_HASH: &str = "d3b144ae3f2a23843835844329a68e21cec90ebcee965cb131185c265d5c793d";
const COUNTER: &str =
    "oteryn:quest-progress/crystalserver/quest/u13_20/rotten_blood/putrefactory_task";
const RESERVOIR: TilePos = TilePos {
    x: 34081,
    y: 31704,
    floor: -14,
};
const START: TilePos = TilePos {
    x: 34080,
    ..RESERVOIR
};
const SECOND_RESERVOIR: TilePos = TilePos {
    x: 34013,
    y: 31744,
    floor: -14,
};
const GATE: TilePos = TilePos {
    x: 34065,
    y: 31718,
    floor: -15,
};
const DESTINATION: TilePos = TilePos {
    x: 32972,
    y: 32336,
    floor: -15,
};

#[test]
fn rblood1_putrefactory_use_awards_cools_down_and_spends() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("rblood1-putrefactory".into())
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Putrefactory PostgreSQL scenario panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String) -> TestResult {
    let h =
        Harness::create_with_admission_seed(admin, "rblood1_putrefactory", false, false).await?;
    let result = exercise(&h).await;
    let cleanup = h.cleanup().await;
    result.and(cleanup)
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activation) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("generation")?;
    let world = world(&deps)?;
    let cells = world
        .movement_cells(activation.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(deps.admitted.world_id);
    let start = MovementLocalPosition {
        x: i32::from(START.x),
        y: i32::from(START.y),
        floor: 14,
    };
    let room = activation
        .qualified_room()
        .bundle_view(
            cells.clone(),
            frame,
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .ok_or("bundle room")?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        frame,
        world.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
    deps = Dependencies::with_content(pin, deps.room, spells)?;
    deps.runtime
        .get_mut()
        .initialize_pinned_test_position(deps.actor, start)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
    owner.quest_catalogue = Some(std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            deps.actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let seeded = sqlx::query(
        "INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) \
         SELECT character_id,'oteryn:quest/rotten-blood','content-1',decode($2,'hex'),character_revision,character_revision \
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(BASE_QUEST_HASH)
    .execute(&mut *tx)
    .await?;
    assert_eq!(seeded.rows_affected(), 1);
    tx.commit().await?;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    owner.admit_quest_session(&admitted).await;

    let map = owner.bundle_map().ok_or("callback map")?;
    let use_at = |at: TilePos, command_id| {
        let owner = &owner;
        async move {
            // Position the test player beside the selected donor object, then let USE own all
            // reach, selector, counter and teleport behavior.
            let adjacent = local(adjacent(at));
            {
                let mut runtime = owner.runtime.lock().await;
                let expected = runtime.read_actor_position(deps.actor).map_err(debug)?;
                if expected.position() != adjacent {
                    let proof = crate::map::floor::prepare_quest_callback_teleport(
                        &runtime,
                        owner.movement_cells(),
                        deps.actor,
                        admitted.game_session_id,
                        expected,
                        adjacent,
                        false,
                        &map.facts,
                    )
                    .map_err(debug)?;
                    runtime.commit_source_step(proof).map_err(debug)?;
                }
            }
            let ordinal = if at == RESERVOIR { 2 } else { 1 };
            let key = crate::map::view::placement_key(at, ordinal).ok_or("fixture placement")?;
            let placement = map
                .overlay
                .base()
                .digest()
                .into_iter()
                .chain(key.to_be_bytes())
                .collect();
            let delay = owner
                .states
                .world_gate
                .lock()
                .map_err(|_| "world action gate")?
                .equip_delay(deps.actor, 0, owner.clock_origin.elapsed());
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            }
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "quest fence")?
                .ok_or("quest fence")?;
            Ok::<_, Box<dyn std::error::Error>>(
                owner
                    .use_world_object(
                        deps.actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id,
                            item_fence: Some(fence.item_fence()),
                        },
                        WorldObjectTarget {
                            placement,
                            expected_revision: map.facts.object_revision(key),
                        },
                    )
                    .await
                    .disposition,
            )
        }
    };
    let points =
        || {
            sqlx::query_scalar::<_, i64>(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    ).bind(COUNTER).fetch_one(&h.pool)
        };

    h.root.set_test_quest_clock(1_700_000_000)?;
    let first = use_at(RESERVOIR, 1).await?;
    if first != UseDisposition::Committed {
        return Err(format!("first reservoir USE returned {first:?}, expected Committed").into());
    }
    assert_eq!(points().await?, 20);
    assert_eq!(use_at(GATE, 2).await?, UseDisposition::Rejected);
    assert_eq!(
        use_at(SECOND_RESERVOIR, 3).await?,
        UseDisposition::Committed
    );
    assert_eq!(points().await?, 40);
    h.root.advance_test_quest_clock(19)?;
    assert_eq!(use_at(RESERVOIR, 4).await?, UseDisposition::Rejected);
    assert_eq!(points().await?, 40);
    h.root.advance_test_quest_clock(1)?;
    for (command, expected) in (5..8).zip((60..=100).step_by(20)) {
        assert_eq!(use_at(RESERVOIR, command).await?, UseDisposition::Committed);
        assert_eq!(points().await?, expected);
        h.root.advance_test_quest_clock(20)?;
    }
    assert_eq!(use_at(RESERVOIR, 8).await?, UseDisposition::Rejected);
    assert_eq!(points().await?, 100);
    assert_eq!(use_at(GATE, 9).await?, UseDisposition::Committed);
    assert_eq!(points().await?, 0);
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    assert_eq!(at, local(DESTINATION));
    assert_eq!(use_at(GATE, 10).await?, UseDisposition::Rejected);
    assert_eq!(points().await?, 0);
    Ok(())
}

const fn adjacent(at: TilePos) -> TilePos {
    TilePos { x: at.x - 1, ..at }
}

fn local(at: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

fn world(deps: &Dependencies) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{sector, writer};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = writer::read(&bytes)?;
    data.manifest.world = writer::Extent {
        min_x: 32972,
        min_y: 31704,
        max_x: 34082,
        max_y: 32337,
        floors: vec![-15, -14],
    };
    data.manifest.palette.truncate(1);
    // Mirror the published World stack ordinals and appearance IDs. Its imported items
    // carry no legacy action attributes.
    for (index, appearance) in [42991, 43789, 43790, 10831].into_iter().enumerate() {
        data.manifest.palette.push(writer::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: writer::Family::Terrain,
            id: u32::try_from(index + 1)?,
            terrain: Some(writer::Terrain {
                kind: writer::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
    }
    let mut by_sector = std::collections::BTreeMap::<_, Vec<_>>::new();
    for (at, palettes) in [
        (START, &[0][..]),
        (RESERVOIR, &[0, 1, 2]),
        (adjacent(SECOND_RESERVOIR), &[0]),
        (SECOND_RESERVOIR, &[0, 3]),
        (adjacent(GATE), &[0]),
        (GATE, &[0, 4]),
        (DESTINATION, &[0]),
    ] {
        by_sector
            .entry((at.floor, at.x / 32, at.y / 32))
            .or_default()
            .push(sector::Tile {
                x: at.x,
                y: at.y,
                flags: 0,
                house: 0,
                zones: vec![],
                items: palettes
                    .iter()
                    .map(|&palette| sector::Item {
                        palette,
                        depth: 0,
                        attrs: Default::default(),
                    })
                    .collect(),
            });
    }
    let mut sectors = by_sector
        .into_iter()
        .map(|((floor, sx, sy), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            writer::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect::<Vec<_>>();
    sectors.sort_by_key(writer::order);
    let bytes = writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = writer::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: START,
        },
        deps.admitted.world_id,
        deps.admitted.channel_id,
        |_| None,
    )
    .map_err(debug)
}
