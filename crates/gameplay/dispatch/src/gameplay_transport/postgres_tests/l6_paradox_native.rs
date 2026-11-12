//! Selected source-native tower puzzles; each probe grows into its physical route proof.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;

fn scenario(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED {name}");
    super::qengine_rewards::scenario(name)
}
#[test]
fn l6_paradox_native_first_grass_stair() -> TestResult {
    scenario("qengine-rewards-questpc4-native-tower-first")
}
#[test]
fn l6_paradox_native_second_exact_levers_ladder() -> TestResult {
    scenario("qengine-rewards-questpc4-native-tower-second")
}
#[test]
fn l6_paradox_native_third_ghoul_switch_and_plate() -> TestResult {
    scenario("qengine-rewards-questpc4-native-tower-third")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY {name} version={version}");
    let (z, position, ids, key) = if name.ends_with("-first") {
        (
            7,
            (32478, 31902),
            vec![409, 1791],
            "l6/paradox-first-grass-stair",
        )
    } else if name.ends_with("-second") {
        (
            6,
            (32479, 31905),
            vec![409, 2772],
            "l6/paradox-second-main-pattern",
        )
    } else {
        (
            5,
            (32481, 31904),
            vec![409, 9125],
            "l6/paradox-ghoul-switch",
        )
    };
    source_tile(z, position, &ids)?;
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    callbacks
        .iter()
        .find(|callback| callback.key.ends_with(key))
        .ok_or_else(|| format!("missing selected native tower binding {key}"))?;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("native map capabilities")?;
    if name.ends_with("-first") {
        first(owner, admitted, h).await
    } else if name.ends_with("-second") {
        second(owner, admitted, h).await
    } else {
        third(owner, admitted, h).await
    }
}

fn source_tile(z: u8, position: (u16, u16), expected: &[u64]) -> TestResult {
    use oteryn_world_bundle::{bundle, sector};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let path = root.join(format!(
        "content/world/placements/region-z{z:02}-x126-y124.b3"
    ));
    let bytes = std::fs::read(path)?;
    let region = oteryn_world_bundle_compiler::b3::decode_region(
        &bytes,
        bundle::TILE_LIMITS,
        &mut sector::Budget {
            tiles: 65536,
            entries: 1_000_000,
        },
    )?;
    assert_eq!((region.z, region.rx, region.ry), (z, 126, 124));
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/index.json"),
    )?)?;
    let tile = region
        .sectors
        .iter()
        .flat_map(|(_, _, tiles)| tiles)
        .find(|tile| (tile.x, tile.y) == position)
        .ok_or("native source tile")?;
    assert_eq!(tile.house, 0);
    assert!(
        tile.items
            .iter()
            .all(|item| item.depth == 0 && item.attrs == Default::default()),
        "do not invent source UID or action attributes"
    );
    let actual = tile
        .items
        .iter()
        .map(|item| {
            index["palette"][item.palette as usize]["source_item_id"]
                .as_u64()
                .ok_or("source ID")
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(actual, expected);
    Ok(())
}

pub(super) fn trigger(name: &str) -> oteryn_gameplay_quests::quest::world_callbacks::Position {
    if name.ends_with("-first") {
        position(32481, 31905, 7)
    } else if name.ends_with("-second") {
        position(32479, 31905, 6)
    } else {
        position(32481, 31904, 5)
    }
}
pub(super) fn start(name: &str) -> oteryn_gameplay_quests::quest::world_callbacks::Position {
    if name.ends_with("-first") {
        position(32478, 31908, 7)
    } else if name.ends_with("-second") {
        position(32478, 31905, 6)
    } else {
        position(32480, 31904, 5)
    }
}
fn position(x: u16, y: u16, z: i8) -> oteryn_gameplay_quests::quest::world_callbacks::Position {
    oteryn_gameplay_quests::quest::world_callbacks::Position { x, y, floor: -z }
}
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    let initial = start(name);
    let z = u8::try_from(-initial.floor)?;
    let bytes = |z| match z {
        7 => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z07-x126-y124.b3"
        ))
        .as_slice(),
        6 => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z06-x126-y124.b3"
        ))
        .as_slice(),
        5 => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z05-x126-y124.b3"
        ))
        .as_slice(),
        4 => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z04-x126-y124.b3"
        ))
        .as_slice(),
        _ => unreachable!("selected native pair"),
    };
    let tile = |x, y, ids| ExpectedTile {
        position: (x, y),
        ids,
        counted_entry: None,
    };
    let expected = if z == 7 {
        let mut tiles = vec![
            tile(32478, 31902, vec![409, 1791]),
            tile(32478, 31907, vec![409, 1632]),
            tile(32481, 31905, vec![431, 3696]),
        ];
        for (x, y, ground) in [
            (32478, 31920, 4521),
            (32479, 31920, 4516),
            (32478, 31921, 4522),
            (32479, 31921, 4518),
            (32478, 31922, 4518),
            (32479, 31922, 4515),
        ] {
            tiles.push(tile(x, y, vec![ground, 3696]));
        }
        tiles
    } else if z == 6 {
        let mut tiles = vec![tile(32479, 31905, vec![409, 2772])];
        for x in 32476..=32481 {
            tiles.push(tile(x, 31900, vec![409, 2773]));
        }
        tiles
    } else {
        vec![
            tile(32481, 31904, vec![409, 9125]),
            tile(32479, 31900, vec![431]),
        ]
    };
    let bounds = (32475, 31899, 32484, if z == 7 { 31924 } else { 31907 });
    native_scene_with_appearances(
        native,
        vec![
            NativeCrop {
                bytes: bytes(z),
                region: (z, 126, 124),
                bounds,
                start: (initial.x, initial.y),
                expected,
            },
            NativeCrop {
                bytes: bytes(z - 1),
                region: (z - 1, 126, 124),
                bounds,
                start: (initial.x, initial.y),
                expected: vec![],
            },
        ],
        &[],
        &[
            430, 431, 1632, 1633, 1791, 1947, 1948, 2772, 2773, 3695, 9125, 9126,
        ],
    )
}

fn appearance(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pos: Position,
    ordinal: u8,
) -> TestResult<u16> {
    let map = owner.bundle_map().ok_or("native map")?;
    let tile = map
        .overlay
        .base()
        .tile(pos.x, pos.y, pos.floor)
        .ok_or("source tile")?;
    let id = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))
        .map(|(id, _)| *id)
        .ok_or("native source ordinal")?;
    Ok(map
        .facts
        .base_entry(
            crate::map::overlay::TilePos {
                x: pos.x,
                y: pos.y,
                floor: pos.floor,
            },
            ordinal,
            id,
        )
        .ok_or("native source facts")?
        .appearance_id)
}
pub(super) fn ladder(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pos: Position,
) -> TestResult<Option<u64>> {
    let entries = owner
        .bundle_map()
        .ok_or("native map")?
        .facts
        .movement_items(crate::map::overlay::TilePos {
            x: pos.x,
            y: pos.y,
            floor: pos.floor,
        });
    assert!(entries.len() <= 1);
    if let Some(entry) = entries.first() {
        assert_eq!(entry.facts.appearance_id, 1948);
        assert!(!entry.facts.pickupable);
        if let crate::map::view::EntrySource::Added(id) = entry.source {
            return Ok(Some(id.get()));
        }
        return Err("owned native ladder identity".into());
    }
    Ok(None)
}
pub(super) async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<Position> {
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    Ok(Position {
        x: at.x.try_into()?,
        y: at.y.try_into()?,
        floor: -i8::try_from(at.floor)?,
    })
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    dx: i32,
    dy: i32,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::*;
    let direction = match (dx, dy) {
        (0, -1) => StepDirection::North,
        (1, 0) => StepDirection::East,
        (0, 1) => StepDirection::South,
        (-1, 0) => StepDirection::West,
        _ => return Err("one source step".into()),
    };
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_millis(500))
        .ok_or("clock")?;
    let before_step = at(owner, admitted).await?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved,
        "native step from {before_step:?} by ({dx},{dy})"
    );
    Ok(())
}
pub(super) async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: Position,
) -> TestResult {
    for _ in 0..64 {
        let here = at(owner, admitted).await?;
        assert_eq!(
            here.floor, target.floor,
            "ordinary source walk stays on its floor"
        );
        if here == target {
            return Ok(());
        }
        step(
            owner,
            admitted,
            (i32::from(target.x) - i32::from(here.x)).signum(),
            if here.x == target.x {
                (i32::from(target.y) - i32::from(here.y)).signum()
            } else {
                0
            },
        )
        .await?;
    }
    Err("bounded native walk".into())
}
async fn sql_state(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(r)) FROM game_character_roots r),'states',(SELECT jsonb_agg(to_jsonb(s) ORDER BY quest_key) FROM game_character_quest_states s),'tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY track_key) FROM game_character_quest_tracks t),'receipts',(SELECT count(*) FROM game_character_quest_receipts),'sources',(SELECT count(*) FROM game_quest_reward_sources),'mints',(SELECT count(*) FROM game_reward_claim_mint_receipts),'consumptions',(SELECT count(*) FROM game_quest_callback_consumptions),'items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i))").fetch_one(&h.pool).await?)
}
async fn second(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::l6_world_admission::use_source;
    let main = position(32479, 31905, 6);
    let stairs = position(32476, 31904, 6);
    let before = sql_state(h).await?;
    assert_eq!(
        use_source(owner, admitted, main, 0).await?,
        UseDisposition::Rejected
    );
    assert!(ladder(owner, stairs)?.is_none());
    assert_eq!(sql_state(h).await?, before);
    walk(owner, admitted, position(32476, 31905, 6)).await?;
    walk(owner, admitted, position(32476, 31901, 6)).await?;
    walk(owner, admitted, position(32478, 31901, 6)).await?;
    for x in [32478, 32479] {
        assert_eq!(
            use_source(owner, admitted, position(x, 31900, 6), 0).await?,
            UseDisposition::Committed
        );
    }
    walk(owner, admitted, position(32480, 31901, 6)).await?;
    assert_eq!(
        use_source(owner, admitted, position(32481, 31900, 6), 0).await?,
        UseDisposition::Committed
    );
    walk(owner, admitted, position(32481, 31901, 6)).await?;
    walk(owner, admitted, position(32481, 31905, 6)).await?;
    walk(owner, admitted, position(32480, 31905, 6)).await?;
    assert_eq!(
        use_source(owner, admitted, main, 0).await?,
        UseDisposition::Committed
    );
    let first = ladder(owner, stairs)?.ok_or("native second ladder")?;
    walk(owner, admitted, position(32481, 31905, 6)).await?;
    walk(owner, admitted, position(32481, 31901, 6)).await?;
    walk(owner, admitted, position(32480, 31901, 6)).await?;
    assert_eq!(
        use_source(owner, admitted, position(32481, 31900, 6), 1).await?,
        UseDisposition::Committed
    );
    walk(owner, admitted, position(32481, 31901, 6)).await?;
    walk(owner, admitted, position(32481, 31905, 6)).await?;
    walk(owner, admitted, position(32480, 31905, 6)).await?;
    assert_eq!(
        use_source(owner, admitted, main, 1).await?,
        UseDisposition::Committed,
        "closing ignores a changed pattern"
    );
    assert!(ladder(owner, stairs)?.is_none());
    assert_eq!(
        use_source(owner, admitted, main, 2).await?,
        UseDisposition::Rejected,
        "reopening rechecks all six source switches"
    );
    walk(owner, admitted, position(32481, 31905, 6)).await?;
    walk(owner, admitted, position(32481, 31901, 6)).await?;
    walk(owner, admitted, position(32480, 31901, 6)).await?;
    assert_eq!(
        use_source(owner, admitted, position(32481, 31900, 6), 2).await?,
        UseDisposition::Committed
    );
    walk(owner, admitted, position(32481, 31901, 6)).await?;
    walk(owner, admitted, position(32481, 31905, 6)).await?;
    walk(owner, admitted, position(32480, 31905, 6)).await?;
    assert_eq!(
        use_source(owner, admitted, main, 2).await?,
        UseDisposition::Committed
    );
    assert_ne!(ladder(owner, stairs)?.ok_or("reopened ladder")?, first);
    walk(owner, admitted, position(32481, 31905, 6)).await?;
    walk(owner, admitted, position(32481, 31901, 6)).await?;
    walk(owner, admitted, position(32476, 31901, 6)).await?;
    walk(owner, admitted, position(32476, 31903, 6)).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    super::l6_world_puzzles::use_added_ladder(
        owner,
        admitted,
        crate::map::overlay::TilePos {
            x: stairs.x,
            y: stairs.y,
            floor: stairs.floor,
        },
    )
    .await?;
    assert_eq!(at(owner, admitted).await?, position(32476, 31905, 5));
    assert_eq!(
        sql_state(h).await?,
        before,
        "source pattern changes and climb grant no personal value/progress"
    );
    Ok(())
}
async fn first(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: start("-first"),
            item: "oteryn:item.tibia.i3308".into(),
            count: 1,
        },
        80,
        true,
    )
    .await?;
    let tool:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key='oteryn:item.tibia.i3308'").bind(fence.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(appearance(owner, position(32478, 31907, 7), 1)?, 1632);
    assert_eq!(
        super::l6_world_admission::use_source(owner, admitted, position(32478, 31907, 7), 0)
            .await?,
        UseDisposition::Committed,
        "open the actual native corridor door"
    );
    assert_eq!(appearance(owner, position(32478, 31907, 7), 1)?, 1633);
    walk(owner, admitted, position(32478, 31907, 7)).await?;
    let entry = [
        (32478, 31906),
        (32478, 31905),
        (32477, 31905),
        (32476, 31905),
        (32476, 31904),
        (32476, 31903),
        (32476, 31902),
        (32476, 31901),
        (32476, 31900),
        (32477, 31900),
        (32478, 31900),
        (32479, 31900),
        (32480, 31900),
        (32481, 31900),
        (32482, 31900),
        (32482, 31901),
        (32482, 31902),
        (32482, 31903),
        (32482, 31904),
        (32482, 31905),
    ];
    for (x, y) in entry {
        let target = position(x, y, 7);
        cut_grass(owner, admitted, target).await?;
        walk(owner, admitted, target).await?;
    }
    cut_grass(owner, admitted, position(32481, 31905, 7)).await?;
    walk(owner, admitted, position(32481, 31905, 7)).await?;
    assert_eq!(
        appearance(owner, position(32478, 31902, 7), 1)?,
        1947,
        "all six untouched native grass predicates create the source stair"
    );
    // Native maze route avoids the actual solid amphorae. Every obstructing grass
    // entry is cut using the observed carried Machete and its real map ordinal.
    let path = [
        (32482, 31905),
        (32482, 31904),
        (32482, 31903),
        (32482, 31902),
        (32482, 31901),
        (32482, 31900),
        (32481, 31900),
        (32480, 31900),
        (32480, 31901),
        (32480, 31902),
        (32480, 31903),
        (32479, 31903),
        (32478, 31903),
    ];
    for (x, y) in path {
        let target = position(x, y, 7);
        cut_grass(owner, admitted, target).await?;
        walk(owner, admitted, target).await?;
    }
    for y in [31920, 31921, 31922] {
        for x in [32478, 32479] {
            assert_eq!(
                appearance(owner, position(x, y, 7), 1)?,
                3696,
                "do not cut the six required outside grass tiles"
            );
        }
    }
    step(owner, admitted, 0, -1).await?;
    assert_eq!(
        at(owner, admitted).await?,
        position(32478, 31901, 6),
        "actual native1947 upstairs-north traversal"
    );
    let after:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key='oteryn:item.tibia.i3308'").bind(fence.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after, tool,
        "retained Machete identity/quantity/custody are unchanged"
    );
    let personal:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_reward_sources),(SELECT count(*) FROM game_quest_callback_consumptions)").fetch_one(&h.pool).await?;
    assert_eq!(
        personal,
        (0, 0, 0),
        "physical grass/stair puzzle neither charges quest inputs nor grants personal value"
    );
    Ok(())
}
async fn cut_grass(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: Position,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
    };
    let map = owner.bundle_map().ok_or("native map")?;
    let tile = map
        .overlay
        .base()
        .tile(target.x, target.y, target.floor)
        .ok_or("grass tile")?;
    if tile.depths().iter().filter(|depth| **depth == 0).count() < 2
        || appearance(owner, target, 1)? != 3696
    {
        return Ok(());
    }
    let key = oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, 1)
        .ok_or("grass source key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("placeholder")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let reference = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i3308", "definition-r1"))
        .ok_or("native Machete")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let (next, outcome, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
    )
    .await?;
    *admitted = next;
    assert_eq!(
        outcome,
        UseDisposition::Committed,
        "ordinary Machete on exact native grass {target:?}"
    );
    assert_eq!(appearance(owner, target, 1)?, 3695);
    Ok(())
}
async fn third(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::l6_world_admission::use_source;
    let lever = position(32481, 31904, 5);
    let plate = position(32479, 31900, 5);
    let stairs = position(32478, 31904, 5);
    let before = sql_state(h).await?;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let key = oteryn_world_bundle::bundle::placement_key(lever.floor, lever.x, lever.y, 1)
            .ok_or("lever key")?;
        let facts = &owner.bundle_map().ok_or("native map")?.facts;
        eprintln!(
            "NATIVE_TIMED_CONTEXT level={:?} restore={} busy={}",
            states
                .get(
                    &runtime,
                    admitted.runtime_actor.ok_or("actor")?,
                    admitted.game_session_id
                )
                .map(|state| state.character_facts().level),
            facts.quest_callback_replacement(key, 9125).is_some(),
            facts.quest_callback_replacement(key, 9126).is_some()
        );
    }
    assert!(ladder(owner, stairs)?.is_none());
    owner.drain_quest_encounter_deadlines().await;
    assert!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_candidates(4096)
            .map_err(debug)?
            .is_empty()
    );
    assert_eq!(
        use_source(owner, admitted, lever, 0).await?,
        UseDisposition::Committed
    );
    let spawn_at = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("queued source deadline")?;
    let scheduled = spawn_at.checked_sub(2_000_000).ok_or("source2s delay")?;
    let reset_at = scheduled
        .checked_add(100_000_000)
        .ok_or("source100s reset")?;
    assert_eq!(appearance(owner, lever, 1)?, 9126);
    assert_eq!(
        sql_state(h).await?,
        before,
        "queuing a source monster cannot grant value or progress"
    );
    walk(owner, admitted, position(32479, 31904, 5)).await?;
    set_clock(owner, spawn_at - 100_000)?;
    owner.drain_quest_encounter_deadlines().await;
    assert!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_candidates(4096)
            .map_err(debug)?
            .is_empty(),
        "no early native ghoul"
    );
    assert!(ladder(owner, stairs)?.is_none());
    set_clock(owner, spawn_at + 100_000)?;
    owner.drain_quest_encounter_deadlines().await;
    let ghoul = {
        let runtime = owner.runtime.lock().await;
        let monsters = runtime
            .current_live_creature_candidates(4096)
            .map_err(debug)?;
        assert_eq!(monsters.len(), 1);
        let ghoul = monsters[0];
        let native = runtime.companion_snapshot(ghoul).map_err(debug)?;
        assert_eq!(native.state.policy.definition_key, "oteryn:creature.ghoul");
        assert_eq!(
            native.health,
            runtime
                .companion_policy("oteryn:creature.ghoul")
                .map_err(debug)?
                .maximum_health
        );
        ghoul
    };
    let original = ladder(owner, stairs)?.ok_or("native ghoul presses its actual source plate")?;
    assert_eq!(appearance(owner, plate, 0)?, 430);
    assert_eq!(
        sql_state(h).await?,
        before,
        "native spawn and plate carry no personal progress/value"
    );
    super::l6_world_puzzles::use_added_ladder(
        owner,
        admitted,
        crate::map::overlay::TilePos {
            x: stairs.x,
            y: stairs.y,
            floor: stairs.floor,
        },
    )
    .await?;
    assert_eq!(at(owner, admitted).await?, position(32478, 31905, 4));
    // The independently bound fourth-floor checkpoint settles from the real ladder ingress.
    let checkpoint:i64=sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/l6/paradox/floor-four-arrived'),-1)").fetch_one(&h.pool).await?;
    assert_eq!(checkpoint, 1);
    let after_climb = sql_state(h).await?;
    let (inside, outside, guest) = {
        let mut runtime = owner.runtime.lock().await;
        let pin = runtime.content_pin().clone();
        let inside = runtime
            .realize_native_qualification_spawn_for_source(
                u16::MAX - 9,
                pin.clone(),
                "oteryn:creature.rat",
                crate::foundation::MovementLocalPosition {
                    x: 32478,
                    y: 31900,
                    floor: 5,
                },
            )
            .map_err(debug)?
            .actor;
        let outside = runtime
            .realize_native_qualification_spawn_for_source(
                u16::MAX - 10,
                pin,
                "oteryn:creature.rat",
                crate::foundation::MovementLocalPosition {
                    x: 32480,
                    y: 31904,
                    floor: 5,
                },
            )
            .map_err(debug)?
            .actor;
        let reservation = runtime
            .reserve_fresh_session(
                crate::foundation::GameSessionId::decode(&super::fixture::id(34)).map_err(debug)?,
            )
            .map_err(debug)?;
        let guest = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                guest,
                crate::foundation::MovementLocalPosition {
                    x: 32479,
                    y: 31901,
                    floor: 5,
                },
            )
            .map_err(debug)?;
        (inside, outside, guest)
    };
    set_clock(owner, reset_at - 100_000)?;
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(ladder(owner, stairs)?, Some(original));
    assert!(
        owner.runtime.lock().await.contains_live_creature(ghoul),
        "no early cleanup"
    );
    set_clock(owner, reset_at + 100_000)?;
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(!runtime.contains_live_creature(ghoul));
        assert!(!runtime.contains_live_creature(inside));
        assert!(runtime.contains_live_creature(outside));
        assert!(
            runtime.read_actor_position(guest).is_ok(),
            "source cleanup leaves players inside its rectangle"
        );
        assert!(
            runtime
                .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                .is_ok()
        );
    }
    assert_eq!(appearance(owner, lever, 1)?, 9125);
    assert_eq!(appearance(owner, plate, 0)?, 431);
    assert!(ladder(owner, stairs)?.is_none());
    assert_eq!(
        sql_state(h).await?,
        after_climb,
        "cleanup/reset adds no reward, kill or quest write"
    );
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        sql_state(h).await?,
        after_climb,
        "source timer reconciliation is idempotent"
    );
    Ok(())
}
fn set_clock(owner: &mut ComposedFreshAdmission<'_, '_, '_>, micros: u64) -> TestResult {
    assert!(
        micros >= owner.owner_now().get(),
        "fixture semantic clock never rewinds"
    );
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_micros(micros))
        .ok_or("semantic clock")?;
    Ok(())
}
