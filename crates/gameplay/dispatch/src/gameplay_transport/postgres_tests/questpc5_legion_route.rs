//! Whole selected Legion route: actual source NPCs, native descent, corpse and forcefield.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
fn p(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}
#[test]
fn questpc5_legion_complete_both_contacts_native_descent_corpse_and_forcefield_return() -> TestResult
{
    super::qengine_rewards::scenario("qengine-rewards-newhaven-legion-route")
}
async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    position: Position,
) -> TestResult {
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        position,
    )
    .await
}
async fn track(h: &Harness, name: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/legion-helmet/{name}")).fetch_optional(&h.pool).await?.unwrap_or(-1))
}
async fn completed(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.newhaven_legion_helmet_quest' AND completed_character_revision IS NOT NULL").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
    destination: Position,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(destination)
    );
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let started = std::time::Instant::now();
    for name in [
        "legion-route-entry",
        "legion-cave-entry",
        "legion-cave-return",
    ] {
        assert!(
            oteryn_gameplay_quests::quest::world_callbacks::load()?
                .iter()
                .any(|c| c.key == format!("oteryn:quest-callback/authored-{name}")),
            "missing whole-route binding {name}"
        );
    }
    let xp:i64=sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(track(h, "cave").await?, -1);
    assert_eq!(completed(h).await?, 0);
    super::questpc5_newhaven_route::legion_contacts(owner, admitted, h).await?;
    at(owner, admitted, p(32143, 31909, -8)).await?;
    assert_ne!(
        super::questpc5_legion_claim_gate::use_corpse_at_ordinal(owner, admitted, 2).await?,
        UseDisposition::Committed,
        "contacts alone cannot bypass the cave visit"
    );
    assert_eq!(super::questpc5_legion_claim_gate::count(h).await?, (0, 0));
    at(owner, admitted, p(32570, 32507, -7)).await?;
    step(owner, admitted, StepDirection::South, p(32111, 31904, -7)).await?;
    assert_eq!(
        track(h, "cave").await?,
        -1,
        "route bridge alone is not a descent"
    );
    step(owner, admitted, StepDirection::North, p(32143, 31909, -8)).await?;
    assert_eq!(
        track(h, "cave").await?,
        1,
        "actual native hole descent records the equipment room visit"
    );
    at(owner, admitted, p(32124, 31939, -8)).await?;
    step(owner, admitted, StepDirection::West, p(32110, 31937, -8)).await?;
    assert_eq!(
        completed(h).await?,
        0,
        "actual return forcefield cannot complete before the personal helmet claim"
    );
    assert_eq!(track(h, "finished").await?, -1);
    assert_eq!(super::questpc5_legion_claim_gate::count(h).await?, (0, 0));
    at(owner, admitted, p(32143, 31909, -8)).await?;
    assert_eq!(
        super::questpc5_legion_claim_gate::use_corpse_at_ordinal(owner, admitted, 2).await?,
        UseDisposition::Committed
    );
    assert_eq!(super::questpc5_legion_claim_gate::count(h).await?, (1, 1));
    assert_eq!(
        completed(h).await?,
        0,
        "corpse acquisition is not the final forcefield stage"
    );
    let _ = super::questpc5_legion_claim_gate::use_corpse_at_ordinal(owner, admitted, 2).await?;
    assert_eq!(super::questpc5_legion_claim_gate::count(h).await?, (1, 1));
    at(owner, admitted, p(32110, 31938, -8)).await?;
    step(owner, admitted, StepDirection::North, p(32110, 31937, -8)).await?;
    assert_eq!(
        completed(h).await?,
        0,
        "walking onto the portal destination cannot impersonate a forcefield traversal"
    );
    assert_eq!(track(h, "finished").await?, -1);
    at(owner, admitted, p(32143, 31909, -8)).await?; // Reset only the preceding negative fixture.
    step(owner, admitted, StepDirection::South, p(32143, 31910, -8)).await?;
    step(owner, admitted, StepDirection::North, p(32124, 31939, -8)).await?;
    // This clear approach is source4394+1056; western1840 is a blocking decoration.
    step(owner, admitted, StepDirection::North, p(32124, 31938, -8)).await?;
    assert_eq!(completed(h).await?, 0);
    assert_eq!(track(h, "finished").await?, -1);
    step(owner, admitted, StepDirection::South, p(32143, 31909, -8)).await?; // Reverse equipment portal.
    step(owner, admitted, StepDirection::South, p(32143, 31910, -8)).await?;
    step(owner, admitted, StepDirection::North, p(32124, 31939, -8)).await?;
    step(owner, admitted, StepDirection::West, p(32570, 32507, -7)).await?;
    assert_eq!(completed(h).await?, 1);
    assert_eq!(track(h, "finished").await?, 1);
    assert_eq!(super::questpc5_legion_claim_gate::count(h).await?, (1, 1));
    at(owner, admitted, p(32124, 31939, -8)).await?;
    step(owner, admitted, StepDirection::West, p(32110, 31937, -8)).await?;
    assert_eq!(
        completed(h).await?,
        1,
        "terminal forcefield replay cannot reopen or duplicate completion"
    );
    assert_eq!(track(h, "finished").await?, 1);
    let after:i64=sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after, xp,
        "selected recipe grants one helmet and no experience"
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED legion_whole_route duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.clear();
    let mut palette = std::collections::BTreeMap::new();
    for (appearance, ground, speed) in [
        (51914, true, 120),
        (4518, true, 150),
        (4515, true, 150),
        (4526, true, 150),
        (593, true, 160),
        (594, true, 160),
        (386, true, 160),
        (4402, true, 120),
        (4398, true, 120),
        (4394, true, 120),
        (4409, true, 120),
        (500, true, 160),
        (4399, true, 120),
        (6298, false, 0),
        (6293, false, 0),
        (4240, false, 0),
        (1949, false, 0),
        (6285, false, 0),
        (105, false, 0),
        (1840, false, 0),
        (1056, false, 0),
        (1058, false, 0),
    ] {
        let index = u32::try_from(data.manifest.palette.len())?;
        palette.insert(appearance, index);
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: index,
            terrain: Some(bundle::Terrain {
                kind: if ground {
                    bundle::TerrainKind::Ground
                } else {
                    bundle::TerrainKind::Common
                },
                walkable: ground.then_some(true),
                ground_speed: ground.then_some(speed),
            }),
        });
    }
    let mut tiles = std::collections::BTreeMap::new();
    let mut add = |x, y, floor, appearances: &[u16], flags| -> TestResult {
        let items = appearances
            .iter()
            .map(|a| {
                Ok(sector::Item {
                    palette: *palette.get(a).ok_or("palette")?,
                    depth: 0,
                    attrs: sector::Attrs::default(),
                })
            })
            .collect::<TestResult<Vec<_>>>()?;
        tiles.insert(
            (floor, y, x),
            sector::Tile {
                x,
                y,
                flags,
                house: 0,
                zones: Vec::new(),
                items,
            },
        );
        Ok(())
    };
    for y in 32487..=32489 {
        for x in 32558..=32561 {
            add(x, y, -7, &[51914], 0)?;
        }
    }
    for y in 32506..=32509 {
        for x in 32569..=32571 {
            add(x, y, -7, &[4518], 1)?;
        }
    }
    add(32570, 32508, -7, &[4515], 1)?;
    add(32110, 31938, -8, &[500, 6293], 0)?;
    add(32111, 31903, -7, &[593], 0)?;
    add(32111, 31904, -7, &[4526], 0)?;
    add(32111, 31903, -8, &[386], 0)?;
    add(32143, 31909, -8, &[4398, 6285, 105], 0)?;
    add(32143, 31910, -8, &[4402, 6298, 4240], 0)?;
    add(32121, 31939, -8, &[4409], 0)?;
    add(32122, 31939, -8, &[4409, 1840], 0)?;
    add(32123, 31939, -8, &[500, 1949], 0)?;
    add(32124, 31938, -8, &[4394, 1058], 0)?;
    add(32124, 31939, -8, &[4394, 1056], 0)?;
    add(32110, 31937, -8, &[4399, 1056], 0)?;
    let startup: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/quest-startup-bindings.json"
    )))?;
    let selector = startup["bindings"]
        .as_array()
        .ok_or("startup")?
        .iter()
        .find(|r| r["attribute"] == "unique" && r["value"] == 6006)
        .ok_or("production Legion corpse startup selector")?;
    assert_eq!(selector["position"], serde_json::json!([32143, 31910, 8]));
    assert_eq!(selector["item"], 4240);
    tiles.get_mut(&(-8, 31910, 32143)).ok_or("corpse")?.items[2]
        .attrs
        .unique = Some(u16::try_from(selector["value"].as_u64().ok_or("UID")?)?);
    tiles.get_mut(&(-8, 31939, 32123)).ok_or("portal")?.items[1]
        .attrs
        .teleport = Some((32110, 31937, (-8_i8).to_ne_bytes()[0]));
    let mut grouped = std::collections::BTreeMap::<(i8, u16, u16), Vec<sector::Tile>>::new();
    for ((floor, y, x), tile) in tiles {
        grouped
            .entry((floor, y / 32, x / 32))
            .or_default()
            .push(tile);
    }
    let sectors = grouped
        .into_iter()
        .map(|((floor, sy, sx), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    data.manifest.world = bundle::Extent {
        min_x: 32110,
        min_y: 31903,
        max_x: 32572,
        max_y: 32510,
        floors: vec![-8, -7],
    };
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 32559,
                y: 32488,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
