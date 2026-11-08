//! Chosen Life Ring exploration, reusable pick, native descent and exact cache.
use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes::{object_target, step, use_target};
use super::{TestResult, quest_world_callbacks};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn questpc8_life_ring_pick_descent_exact_rewards_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-life-ring", || {
        quest_world_callbacks::scenario(
            "authored-life-ring-entry",
            Position {
                x: 32437,
                y: 32238,
                floor: -10,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_life_ring_pick_descent_exact_rewards_and_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.clear();
    let appearances = [351, 352, 353, 354, 355, 386, 394, 2469];
    for (index, appearance) in appearances.into_iter().enumerate() {
        let ground = index < 7;
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(index)?,
            terrain: Some(bundle::Terrain {
                blocks_projectile: None,
                kind: if ground {
                    bundle::TerrainKind::Ground
                } else {
                    bundle::TerrainKind::Common
                },
                walkable: ground.then_some(true),
                ground_speed: ground.then_some(150),
            }),
        });
    }
    let item = |appearance| sector::Item {
        palette: u32::try_from(
            appearances
                .iter()
                .position(|id| *id == appearance)
                .expect("fixture appearance"),
        )
        .expect("bounded palette"),
        depth: 0,
        attrs: Default::default(),
    };
    let mut tiles10 = Vec::new();
    for (dy, row) in [[351, 351, 351], [351, 355, 351], [355, 353, 351]]
        .into_iter()
        .enumerate()
    {
        for (dx, ground) in row.into_iter().enumerate() {
            tiles10.push(sector::Tile {
                x: 32436 + u16::try_from(dx)?,
                y: 32238 + u16::try_from(dy)?,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items: vec![item(ground)],
            });
        }
    }
    let grounds11 = [
        [354, 355, 352, 355, 352, 351, 352, 353],
        [352, 386, 354, 355, 354, 351, 352, 354],
        [355, 353, 353, 354, 351, 354, 354, 352],
    ];
    let mut tiles11 = Vec::new();
    for (dy, row) in grounds11.into_iter().enumerate() {
        for (dx, ground) in row.into_iter().enumerate() {
            let x = 32436 + u16::try_from(dx)?;
            let y = 32238 + u16::try_from(dy)?;
            let cache = (x, y) == (32443, 32238);
            let mut items = vec![item(ground)];
            if cache {
                items.push(item(2469));
            }
            tiles11.push(sector::Tile {
                x,
                y,
                flags: if cache { 9 } else { 0 },
                house: 0,
                zones: Vec::new(),
                items,
            });
        }
    }
    let sectors = vec![
        bundle::Sector {
            floor: -11,
            sx: 32436 / 32,
            sy: 32238 / 32,
            tiles: tiles11,
        },
        bundle::Sector {
            floor: -10,
            sx: 32436 / 32,
            sy: 32238 / 32,
            tiles: tiles10,
        },
    ];
    data.manifest.world = bundle::Extent {
        min_x: 32436,
        min_y: 32238,
        max_x: 32444,
        max_y: 32241,
        floors: vec![-11, -10],
    };
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 32436,
                y: 32238,
                floor: -10,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner.premium_coordinator = None;
    let before = [quantity(h, 3052).await?, quantity(h, 3085).await?];
    step(owner, admitted, 1, StepDirection::East).await?;
    assert_eq!(progress(h).await?, 1);
    let hole = TilePos {
        x: 32437,
        y: 32239,
        floor: -10,
    };
    let target = object_target(owner, hole, 0)?;
    assert_eq!(
        use_target(owner, admitted, 2, &target).await?,
        UseDisposition::Rejected,
        "opening the passage must require a carried pick"
    );
    assert_eq!(progress(h).await?, 1);
    assert_eq!(ground_appearance(owner, hole)?, 355);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 32437,
                y: 32238,
                floor: -10,
            },
            item: "oteryn:item.tibia.i3456".into(),
            count: 1,
        },
        60,
        true,
    )
    .await?;
    let pick = pick_instance(h).await?;
    assert_eq!(
        use_target(owner, admitted, 3, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h).await?, 2);
    assert_eq!(ground_appearance(owner, hole)?, 394);
    assert_eq!(quantity(h, 3456).await?, 1);
    assert_eq!(
        pick_instance(h).await?,
        pick,
        "reusable tool keeps its identity"
    );
    let consumed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1",
    )
    .bind("oteryn:quest-callback/authored-life-ring-pick")
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(consumed, 0, "a reusable tool creates no BURN");
    assert_eq!(quantity(h, 3052).await?, before[0]);
    assert_eq!(quantity(h, 3085).await?, before[1]);
    step(owner, admitted, 4, StepDirection::South).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        MovementLocalPosition {
            x: 32437,
            y: 32239,
            floor: 11
        },
        "native394 floor-change must descend onto the source rope spot386"
    );
    for command in 5..=9 {
        step(owner, admitted, command, StepDirection::East).await?;
    }
    step(owner, admitted, 10, StepDirection::North).await?;
    let cache = TilePos {
        x: 32443,
        y: 32238,
        floor: -11,
    };
    let target = object_target(owner, cache, 1)?;
    assert_eq!(
        use_target(owner, admitted, 11, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h).await?, 4);
    assert_rewards(owner, h, before).await?;
    assert_eq!(
        use_target(owner, admitted, 11, &target).await?,
        UseDisposition::Committed
    );
    let target = object_target(owner, cache, 1)?;
    assert_eq!(
        use_target(owner, admitted, 12, &target).await?,
        UseDisposition::Rejected
    );
    assert_rewards(owner, h, before).await?;
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.life_ring_quest'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    let target = object_target(owner, cache, 1)?;
    assert_eq!(
        use_target(owner, admitted, 13, &target).await?,
        UseDisposition::Rejected
    );
    assert_rewards(owner, h, before).await?;
    assert_eq!(pick_instance(h).await?, pick);
    Ok(())
}
async fn assert_rewards(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    before: [i64; 2],
) -> TestResult {
    assert_eq!(quantity(h, 3052).await? - before[0], 1);
    assert_eq!(quantity(h, 3085).await? - before[1], 1);
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    use crate::content::ReferenceItemField::Known;
    let policy = native
        .item_policy("oteryn:item.tibia.i3085", "definition-r1")
        .ok_or("dragon necklace policy")?;
    assert!(
        matches!(&policy.record().semantics.charges,Known(value) if value.count == Known(200)),
        "source subtype200 is granted only when full-definition charges match exactly"
    );
    let explicit:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_timed_states t JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3085'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        explicit, 0,
        "fresh necklace uses the proven full definition, not partial charges"
    );
    Ok(())
}
async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn progress(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/chosen/life-ring/stage'),-1)")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn pick_instance(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT i.item_instance_id::text FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3456' AND i.lifecycle=1 AND e.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

fn ground_appearance(owner: &ComposedFreshAdmission<'_, '_, '_>, at: TilePos) -> TestResult<u16> {
    let map = owner.bundle_map().ok_or("map")?;
    let tile = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("tile")?;
    let id = *tile.ids().first().ok_or("ground")?;
    Ok(map
        .facts
        .base_entry(at, 0, id)
        .ok_or("ground facts")?
        .appearance_id)
}
