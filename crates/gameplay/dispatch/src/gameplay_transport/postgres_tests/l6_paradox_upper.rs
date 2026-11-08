//! Source-native upper food floors and selected protected climb arrival.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;
const CLIMB: &str = "oteryn:quest-progress/l6/paradox/climb-complete";
fn p(x: u16, y: u16, z: i8) -> Position {
    Position { x, y, floor: -z }
}
pub(super) fn start(name: &str) -> Position {
    if name.ends_with("-complete") {
        super::l6_paradox_story::start()
    } else if name.ends_with("-five") {
        p(32477, 31904, 3)
    } else {
        p(32478, 31905, 4)
    }
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::qengine_rewards::scenario(name)
}
#[test]
fn l6_paradox_upper_native_four_foods() -> TestResult {
    run("qengine-rewards-questpc4-paradox-upper-four")
}
#[test]
fn l6_paradox_upper_native_five_foods() -> TestResult {
    run("qengine-rewards-questpc4-paradox-upper-five")
}
#[test]
fn l6_paradox_upper_native_complete_checkpoint() -> TestResult {
    run("qengine-rewards-questpc4-paradox-upper-complete")
}
pub(super) fn world(
    native: &NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    let mut crops = if name.ends_with("-complete") {
        super::l6_paradox_story::crops()?
    } else {
        Vec::new()
    };
    let bytes: [&'static [u8]; 3] = [
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z04-x126-y124.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z03-x126-y124.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z02-x126-y124.b3"
        )),
    ];
    let initial = start(name);
    let floors = if name.ends_with("-five") {
        vec![3_u8, 2]
    } else {
        vec![4, 3, 2]
    };
    for z in floors {
        let expected = match z {
            4 => vec![ExpectedTile {
                position: (32479, 31905),
                ids: vec![409, 2772],
                counted_entry: None,
            }],
            3 => vec![ExpectedTile {
                position: (32478, 31904),
                ids: vec![409, 2772],
                counted_entry: None,
            }],
            _ => vec![ExpectedTile {
                position: (32479, 31905),
                ids: vec![409],
                counted_entry: None,
            }],
        };
        crops.push(NativeCrop {
            bytes: bytes[usize::from(4 - z)],
            region: (z, 126, 124),
            bounds: (32475, 31899, 32484, 31907),
            start: (initial.x, initial.y),
            expected,
        });
    }
    native_scene_with_appearances(native, crops, &[], &[1948, 2772, 2773])
}
fn callback(z: i8) -> TestResult<Callback> {
    let suffix = if z == 4 {
        "/paradox-food-four"
    } else {
        "/paradox-food-five"
    };
    oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with(suffix))
        .ok_or_else(|| "upper food callback".into())
}
async fn climb(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    z: i8,
) -> TestResult {
    let c = callback(z)?;
    let pos = c.selector.position.ok_or("lever position")?;
    let key = oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, 1)
        .ok_or("lever key")?;
    let rev = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
    assert_eq!(
        super::l6_world_admission::use_source(owner, admitted, pos, rev).await?,
        UseDisposition::Rejected,
        "missing exact native food inputs refuse"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    for (i, food) in c.offerings.iter().enumerate() {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            food,
            500 + u64::try_from(z)? * 10 + u64::try_from(i)?,
            false,
        )
        .await?;
    }
    assert_eq!(
        super::l6_world_admission::use_source(owner, admitted, pos, rev).await?,
        UseDisposition::Committed,
        "all source-native food cells open the ladder"
    );
    ascend(owner, admitted, &c, z).await
}
async fn ascend(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    c: &Callback,
    z: i8,
) -> TestResult {
    let ladder = c
        .effects
        .item_presence
        .as_ref()
        .ok_or("ladder effect")?
        .position;
    assert!(super::l6_paradox_native::ladder(owner, ladder)?.is_some());
    if z == 4 {
        super::l6_paradox_native::walk(owner, admitted, p(32476, 31905, 4)).await?;
    } else {
        super::l6_paradox_native::walk(owner, admitted, p(32477, 31905, 3)).await?;
        super::l6_paradox_native::walk(owner, admitted, p(32479, 31905, 3)).await?;
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("ladder clock")?;
    super::l6_world_puzzles::use_added_ladder(
        owner,
        admitted,
        crate::map::overlay::TilePos {
            x: ladder.x,
            y: ladder.y,
            floor: ladder.floor,
        },
    )
    .await?;
    assert_eq!(
        super::l6_paradox_native::at(owner, admitted).await?,
        p(ladder.x, ladder.y + 1, z - 1),
        "advertised native ladder reaches the next source floor"
    );
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_paradox_upper version={version}");
    let complete = name.ends_with("-complete");
    if complete {
        super::l6_paradox_story::exercise(
            owner,
            admitted,
            h,
            authority,
            "qengine-rewards-questpc4-paradox-story-ordered",
        )
        .await?;
        super::l6_native_signs::relocate(
            owner,
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            p(32477, 31905, 4),
        )
        .await?;
    } else {
        super::l6_paradox::raise_level(owner, admitted, authority, h, 30).await?;
    }
    let mut caps = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    caps.extend([4, 6, 11, 15, 18]);
    caps.sort_unstable();
    caps.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &caps,
        )
        .ok_or("native upper capabilities")?;
    if complete {
        super::l6_paradox_native::walk(owner, admitted, p(32478, 31905, 4)).await?;
        let arrived:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/l6/paradox/floor-four-arrived'").fetch_one(&h.pool).await?;
        assert_eq!(arrived, 1);
        climb(owner, admitted, 4).await?;
        super::l6_paradox_native::walk(owner, admitted, p(32477, 31905, 3)).await?;
        super::l6_paradox_native::walk(owner, admitted, p(32477, 31904, 3)).await?;
        climb(owner, admitted, 3).await?;
        let done:i64=sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1)").bind(CLIMB).fetch_one(&h.pool).await?;
        assert_eq!(
            done, 1,
            "native upper route records the selected protected climb checkpoint"
        );
    } else {
        let five = name.ends_with("-five");
        super::l6_world_admission::exercise(
            owner,
            admitted,
            h,
            authority,
            if five {
                "qengine-rewards-questpc4-admission-opening-five"
            } else {
                "qengine-rewards-questpc4-admission-opening"
            },
        )
        .await?;
        ascend(
            owner,
            admitted,
            &callback(if five { 3 } else { 4 })?,
            if five { 3 } else { 4 },
        )
        .await?;
    }
    Ok(())
}
