//! Real-world journeys use the same pinned bundle and native content as `serve`.
use super::*;
use crate::map::boot::BundleWorld;
use crate::map::overlay::TilePos;
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

#[path = "full_e2ef3.rs"]
mod full_e2ef3;

#[path = "general_stats.rs"]
mod general_stats;

#[path = "creature_sweep.rs"]
mod creature_sweep;
#[path = "mp/mount.rs"]
mod mp;
pub(in crate::qualification) use mp::legacy_d150;

include!("character_core.rs");
include!("character_release.rs");
include!("e2e_row_batch.rs");
include!("floor_journey.rs");
include!("full_e2ef7_mount.rs");
include!("full_row_c03_mount.rs");
include!("full_row_c12_mount.rs");
include!("client_row_c01_mount.rs");
include!("client_row_c02_mount.rs");
include!("client_row_c03_mount.rs");
include!("client_row_c04_mount.rs");
include!("client_row_c05_mount.rs");
include!("client_row_c06_mount.rs");
include!("client_row_c07_mount.rs");
include!("client_row_c09_mount.rs");
include!("client_row_c10_mount.rs");
include!("client_row_c11_mount.rs");
include!("client_row_c12_mount.rs");
include!("client_row_c13_mount.rs");
include!("client_row_c14_mount.rs");
include!("client_row_c15_mount.rs");
include!("client_row_c16a_mount.rs");
include!("client_row_c16b_mount.rs");
include!("client_row_c16c_mount.rs");
include!("client_row_c16d_mount.rs");
include!("client_row_c17a_mount.rs");
include!("client_row_c17b_mount.rs");
include!("client_row_c17d_mount.rs");
include!("client_row_c18a_mount.rs");
include!("client_row_c18b_mount.rs");
include!("client_row_c20a_mount.rs");
include!("client_row_c20b_mount.rs");
include!("client_row_c20c_mount.rs");
include!("client_row_c20d_mount.rs");
include!("client_row_e01_mount.rs");
include!("client_row_i01_mount.rs");
include!("client_row_i03_mount.rs");
include!("client_row_i04_mount.rs");
include!("client_row_i05_mount.rs");
include!("client_row_i06_mount.rs");
include!("client_row_i07_mount.rs");
include!("client_row_i08_mount.rs");
include!("client_row_i09_mount.rs");
include!("client_row_i10_mount.rs");
include!("client_row_i11_mount.rs");
include!("client_row_i12_mount.rs");
include!("client_row_i13_mount.rs");
include!("client_row_i14_mount.rs");
include!("client_row_i15_mount.rs");
include!("client_row_i16_mount.rs");
include!("client_row_m01_mount.rs");
include!("client_row_m02_mount.rs");
include!("client_row_m03_mount.rs");
include!("client_row_m04_mount.rs");
include!("client_row_m05_mount.rs");
include!("client_row_m06_mount.rs");
include!("client_row_m07_mount.rs");
include!("client_row_m08_mount.rs");
include!("client_row_m09_mount.rs");
include!("client_row_m10_mount.rs");
include!("client_row_m11a_mount.rs");
include!("client_row_m11b_mount.rs");
include!("client_row_n01_mount.rs");
include!("client_row_n02_mount.rs");
include!("client_row_n03_mount.rs");
include!("client_row_n04_mount.rs");
include!("client_row_c08_mount.rs");
include!("client_row_c17c_mount.rs");
include!("client_row_c19a_mount.rs");
include!("client_row_e02_mount.rs");
include!("client_row_e03_mount.rs");
include!("client_row_e05_mount.rs");
include!("client_row_e06_mount.rs");
include!("client_row_e07_mount.rs");
include!("client_row_e08_mount.rs");
include!("client_row_e09_mount.rs");
include!("client_row_e10_mount.rs");
include!("client_row_e12_mount.rs");
include!("client_row_p01_mount.rs");
include!("client_row_p02_mount.rs");
include!("client_row_p03_mount.rs");
include!("client_row_p04_mount.rs");
include!("client_row_p05_mount.rs");
include!("client_row_p06_mount.rs");
include!("client_row_p07_mount.rs");
include!("client_row_p08_mount.rs");
include!("client_row_p09_mount.rs");
include!("client_row_p10_mount.rs");
include!("client_row_p11_mount.rs");
include!("client_row_p12a_mount.rs");
include!("client_row_p12c_mount.rs");
include!("client_row_p13_mount.rs");
include!("client_row_p14_mount.rs");
include!("client_row_p15_mount.rs");
include!("client_row_p16_mount.rs");
include!("client_row_p17_mount.rs");
include!("client_row_p18_mount.rs");
include!("client_row_p19_mount.rs");
include!("client_row_p20_mount.rs");
include!("client_row_p21_mount.rs");
include!("client_row_p23c_mount.rs");
include!("client_row_s01_mount.rs");
include!("client_row_s02_mount.rs");
include!("client_row_s03_mount.rs");
include!("client_row_s04a_mount.rs");
include!("client_row_s04b_mount.rs");
include!("client_row_s18_mount.rs");
include!("client_row_w01a_mount.rs");
include!("client_row_w01b_mount.rs");
include!("client_row_w01c_mount.rs");
include!("client_row_w02_mount.rs");
include!("client_row_w03_mount.rs");
include!("client_row_w04_mount.rs");
include!("client_row_w05a_mount.rs");
include!("client_row_w05c_mount.rs");
include!("client_row_w10_mount.rs");
include!("client_row_w11_mount.rs");
include!("client_row_w12_mount.rs");
include!("client_row_w13_mount.rs");
include!("client_row_s06_mount.rs");
include!("client_row_s08_mount.rs");
include!("client_row_s13c_mount.rs");
include!("client_row_s13b_mount.rs");
include!("client_row_s13a_mount.rs");
include!("client_row_s15b_mount.rs");
include!("client_row_s04c_mount.rs");
include!("client_row_s07_mount.rs");
include!("client_row_s16b_mount.rs");
include!("client_row_s09_mount.rs");
include!("client_row_s05_mount.rs");
include!("client_row_s10_mount.rs");
include!("client_row_s16a_mount.rs");
include!("client_row_s11_mount.rs");
include!("client_row_w15a_mount.rs");
include!("client_row_w15b_mount.rs");
include!("client_row_w15d_mount.rs");
include!("client_row_w16_mount.rs");
include!("client_row_w17_mount.rs");
include!("client_row_w19a_mount.rs");
include!("client_row_w20a_mount.rs");
include!("client_row_w20b_mount.rs");
include!("client_row_w22a_mount.rs");
include!("client_row_w22b_mount.rs");
include!("client_row_w23_mount.rs");
include!("client_row_w24_mount.rs");
include!("client_row_n07a_mount.rs");
include!("client_row_n07b_mount.rs");
include!("client_row_n07c_mount.rs");
include!("client_row_n07d_mount.rs");
include!("full_row_c09_mount.rs");
include!("full_row_c06_mount.rs");
include!("full_row_c02_mount.rs");
include!("full_row_c01_mount.rs");
include!("full_row_w22b_mount.rs");
include!("full_row_c05_mount.rs");
include!("full_row_c04_mount.rs");
include!("full_row_w18_mount.rs");
include!("full_row_c11_mount.rs");
include!("full_row_w19a_mount.rs");
include!("full_row_w01b_mount.rs");
include!("full_row_w23_mount.rs");
include!("full_row_w15d_mount.rs");
include!("full_row_w15a_mount.rs");
include!("full_row_w10_mount.rs");
include!("full_row_w08_mount.rs");
include!("full_row_w24_mount.rs");
include!("full_row_w22a_mount.rs");
include!("full_row_w15b_mount.rs");
include!("full_row_w11_mount.rs");
include!("full_row_w05a_mount.rs");
include!("full_row_w03_mount.rs");
include!("full_row_w01a_mount.rs");
include!("full_row_s16b_mount.rs");
include!("full_row_s15b_mount.rs");
include!("full_row_s10_mount.rs");
include!("full_row_s07_mount.rs");
include!("full_row_s06_mount.rs");
include!("full_row_s05_mount.rs");
include!("full_row_p07_mount.rs");
include!("full_row_w17_mount.rs");
include!("full_row_w16_mount.rs");
include!("full_row_w13_mount.rs");
include!("full_row_w12_mount.rs");
include!("full_row_w05c_mount.rs");
include!("full_row_w04_mount.rs");
include!("full_row_w02_mount.rs");
include!("full_row_w01c_mount.rs");
include!("full_row_s13c_mount.rs");
include!("full_row_s04c_mount.rs");
include!("full_row_s04a_mount.rs");
include!("full_row_s02_mount.rs");
include!("full_row_p23c_mount.rs");
include!("full_row_p20_mount.rs");
include!("full_row_p18_mount.rs");
include!("full_row_p17_mount.rs");
include!("full_row_p15_mount.rs");
include!("full_row_p14_mount.rs");
include!("full_row_p13_mount.rs");
include!("full_row_p12c_mount.rs");
include!("full_row_p12a_mount.rs");
include!("full_row_p11_mount.rs");
include!("full_row_p09_mount.rs");
include!("full_row_p08_mount.rs");
include!("full_row_p06_mount.rs");
include!("full_row_p05_mount.rs");
include!("full_row_p04_mount.rs");
include!("full_row_p03_mount.rs");
include!("full_row_p02_mount.rs");
include!("full_row_p01_mount.rs");
include!("full_row_e10_mount.rs");
include!("full_row_e09_mount.rs");
include!("full_row_e08_mount.rs");
include!("full_row_e06_mount.rs");
include!("full_row_i11_mount.rs");
include!("full_row_i10_mount.rs");
include!("full_row_i05_mount.rs");
include!("full_row_m11b_mount.rs");
include!("full_row_m11a_mount.rs");
include!("full_row_m08_mount.rs");
include!("full_row_c18b_mount.rs");
include!("full_row_c17b_mount.rs");
include!("full_row_c10_mount.rs");
include!("full_row_c07_mount.rs");
include!("full_row_e12_mount.rs");
include!("full_row_i14_mount.rs");
include!("full_row_i03_mount.rs");
include!("full_row_m10_mount.rs");
include!("full_row_m09_mount.rs");
include!("full_row_m06_mount.rs");
include!("full_row_c15_mount.rs");
include!("full_row_c08_mount.rs");
include!("full_row_c19b_mount.rs");
include!("full_row_g01_mount.rs");
include!("full_row_g02_mount.rs");
include!("full_row_g03_mount.rs");
include!("full_row_g04_mount.rs");
include!("full_row_g05_mount.rs");
include!("full_row_g06_mount.rs");
include!("full_row_g07_mount.rs");
include!("full_row_g08_mount.rs");
include!("full_row_g09_mount.rs");
include!("full_row_g10_mount.rs");
include!("full_row_g11_mount.rs");
include!("full_row_g12_mount.rs");
include!("full_row_g13_mount.rs");
include!("full_row_g14_mount.rs");
include!("full_row_g15_mount.rs");
include!("full_row_g16_mount.rs");
include!("full_row_g17_mount.rs");
include!("full_row_g18_mount.rs");
include!("full_row_g19_mount.rs");
include!("full_row_g20_mount.rs");
include!("full_row_g21_mount.rs");
include!("full_row_g22_mount.rs");
include!("full_row_g23_mount.rs");
include!("full_row_g24_mount.rs");
include!("full_row_g25_mount.rs");
include!("full_row_g26_mount.rs");
include!("full_row_g27_mount.rs");
include!("full_row_g28_mount.rs");
include!("full_row_g29_mount.rs");
include!("full_row_g30_mount.rs");
include!("full_row_g31_mount.rs");
include!("full_row_g32_mount.rs");
include!("full_row_g33_mount.rs");
include!("full_row_g34_mount.rs");
include!("full_row_g35_mount.rs");
include!("full_row_g36_mount.rs");
include!("full_row_g37_mount.rs");
include!("full_row_g38_mount.rs");
include!("full_row_i17a_mount.rs");
include!("full_row_i17b_mount.rs");
include!("full_row_i17c_mount.rs");
include!("full_row_i17d_mount.rs");
include!("full_row_i18a_mount.rs");
include!("full_row_i18b_mount.rs");
include!("full_row_i18c_mount.rs");
include!("full_row_i19_mount.rs");
include!("full_row_p12b_mount.rs");
include!("full_row_p22a_mount.rs");
include!("full_row_p22b_mount.rs");
include!("full_row_p22c_mount.rs");
include!("full_row_p22d_mount.rs");
include!("full_row_s14a_mount.rs");
include!("full_row_s14b_mount.rs");
include!("full_row_s14c_mount.rs");
include!("full_row_s15a_mount.rs");
include!("full_row_s16c_mount.rs");
include!("full_row_s16d_mount.rs");
include!("full_row_s17a_mount.rs");
include!("full_row_s17b_mount.rs");
include!("full_row_s17c_mount.rs");
include!("full_row_s19_mount.rs");
include!("full_row_w05b_mount.rs");
include!("full_row_w05d_mount.rs");
include!("full_row_w06_mount.rs");
include!("full_row_w07_mount.rs");
include!("full_row_w09_mount.rs");
include!("full_row_w15c_mount.rs");
include!("full_row_w19b_mount.rs");
include!("full_row_w19c_mount.rs");
include!("full_row_w19d_mount.rs");
include!("full_row_w19e_mount.rs");
include!("full_row_w21_mount.rs");
include!("full_e2ef19_mount.rs");
include!("full_e2ef18_mount.rs");
include!("full_e2ef20_mount.rs");
include!("full_e2ef5_mount.rs");
#[path = "map_journey.rs"]
mod map_journey;
include!("native_content.rs");
#[path = "character_prep.rs"]
mod character_prep;
#[path = "full_e2ef1.rs"]
mod provisioning;
include!("quest_journal.rs");
#[path = "spatial_basis.rs"]
mod spatial_basis;
#[path = "starter_inventory.rs"]
mod starter_inventory;
#[path = "verdict.rs"]
pub(crate) mod verdict;

pub(crate) const TOWN: TilePos = TilePos {
    x: 32369,
    y: 32238,
    floor: -7,
};

struct QualificationWorld {
    world_id: WorldId,
    channel_id: ChannelId,
    bundle: BundleWorld,
    rewards: std::sync::Arc<crate::content::creature_reward::CreatureRewardTable>,
}

/// One production generation per test process. Keep its pinned reward table for
/// the sweep's seeded loot expectation as well as the observed World bundle.
fn load_world_generation(
    data: Vec<u8>,
    pin: &serde_json::Value,
    world: WorldId,
    channel: ChannelId,
) -> TestResult<&'static QualificationWorld> {
    static LOADED: std::sync::OnceLock<QualificationWorld> = std::sync::OnceLock::new();
    if LOADED.get().is_none() {
        let booted = boot_world(data, pin, world, channel)?;
        let _ = LOADED.set(booted);
    }
    match LOADED.get() {
        Some(loaded) if loaded.world_id == world && loaded.channel_id == channel => Ok(loaded),
        Some(_) => Err("world loader asked for a different world or channel".into()),
        None => Err("world loader cache empty".into()),
    }
}

pub(crate) fn load_world(
    data: Vec<u8>,
    pin: &serde_json::Value,
    world: WorldId,
    channel: ChannelId,
) -> TestResult<&'static BundleWorld> {
    Ok(&load_world_generation(data, pin, world, channel)?.bundle)
}

fn boot_world(
    data: Vec<u8>,
    pin: &serde_json::Value,
    world: WorldId,
    channel: ChannelId,
) -> TestResult<QualificationWorld> {
    let digest = hex32(pin["digest"].as_str().ok_or("World digest")?)?;
    let checked = crate::map::boot::check(
        data,
        crate::map::boot::BootPins {
            bundle: crate::map::BundlePins {
                digest,
                project_format_version: pin["project_format_version"]
                    .as_str()
                    .ok_or("World project")?
                    .into(),
                world_schema_version: pin["world_schema_version"]
                    .as_str()
                    .ok_or("World schema")?
                    .into(),
                content_revision: pin["content_revision"]
                    .as_str()
                    .ok_or("World content")?
                    .into(),
                production: false,
            },
            map_revision: format!("sha256:{}", pin["digest"].as_str().ok_or("World digest")?),
            start: TOWN,
        },
    )?;
    let input = native_input()?;
    // Prewarm before the world pass admits its first client.
    native_spell_book()?;
    let room = crate::content::qualify_selected_native_gameplay_room(world, input)?;
    let issuance = crate::content::NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = crate::content::ContentActivationController::new();
    crate::content::activate_native_entry_room_with_gameplay(
        &mut controller,
        &crate::content::NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        input,
    )?;
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("world has no native generation")?;
    let rat = native
        .reward_table()
        .row("oteryn:creature.rat")
        .map_err(|reason| format!("rat reward unavailable: {reason:?}"))?;
    if rat.loot_table.entries.is_empty() {
        return Err("town hunt has no production loot table".into());
    }
    let bundle = crate::node::serve::boot_bundle_world(checked, world, channel, Some(native))
        .map_err(|error| format!("production world loader: {error:?}"))?;
    Ok(QualificationWorld {
        world_id: world,
        channel_id: channel,
        bundle,
        rewards: native.reward_table().clone(),
    })
}

fn tile(position: ActorPosition) -> TestResult<TilePos> {
    Ok(TilePos {
        x: u16::try_from(position.x)?,
        y: u16::try_from(position.y)?,
        floor: -i8::try_from(position.floor)?,
    })
}

/// Plan from immutable production map facts; only replies to real STEP commands prove travel.
fn route(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> Option<Vec<StepDirection>> {
    route_avoiding(world, from, goal, |_| true)
}

fn route_avoiding(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
    available: impl Fn(TilePos) -> bool,
) -> Option<Vec<StepDirection>> {
    route_with_landings(from, goal, available, |at, direction| {
        destination(world, at, direction)
    })
}

fn route_with_landings(
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
    available: impl Fn(TilePos) -> bool,
    landing: impl Fn(TilePos, StepDirection) -> Option<TilePos>,
) -> Option<Vec<StepDirection>> {
    find_route(from, goal, |at, direction| {
        let next = landing(at, direction)?;
        (map_journey::in_town(next, from) && available(next)).then_some(next)
    })
    .ok()
}

fn route_around_occupant(
    from: TilePos,
    center: TilePos,
    occupied: TilePos,
    remaining: &VecDeque<StepDirection>,
    available: impl Fn(TilePos) -> bool,
    landing: impl Fn(TilePos, StepDirection) -> Option<TilePos>,
) -> Option<Vec<StepDirection>> {
    let goal = remaining
        .iter()
        .try_fold(occupied, |at, &direction| landing(at, direction))?;
    find_route(
        from,
        |at| at == goal,
        |at, direction| {
            let next = landing(at, direction)?;
            (map_journey::in_town(next, center) && next != occupied && available(next))
                .then_some(next)
        },
    )
    .ok()
}

fn find_route(
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
    landing: impl Fn(TilePos, StepDirection) -> Option<TilePos>,
) -> Result<Vec<StepDirection>, usize> {
    let mut queue = VecDeque::from([from]);
    let mut parents = BTreeMap::from([(from, None)]);
    while let Some(at) = queue.pop_front() {
        if goal(at) {
            let mut path = Vec::new();
            let mut cursor = at;
            while let Some(Some((previous, direction))) = parents.get(&cursor) {
                path.push(*direction);
                cursor = *previous;
            }
            path.reverse();
            return Ok(path);
        }
        for direction in [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ] {
            let Some(next) = landing(at, direction) else {
                continue;
            };
            if let std::collections::btree_map::Entry::Vacant(entry) = parents.entry(next) {
                entry.insert(Some((at, direction)));
                queue.push_back(next);
            }
        }
    }
    Err(parents.len())
}

/// Use the production floor resolver over the production bundle's collision facts.
fn destination(world: &BundleWorld, from: TilePos, direction: StepDirection) -> Option<TilePos> {
    destination_with_teleports(world, from, direction, true)
}

fn destination_with_teleports(
    world: &BundleWorld,
    from: TilePos,
    direction: StepDirection,
    allow_teleport: bool,
) -> Option<TilePos> {
    use crate::map::floor::{Ground, resolve_landing};
    use crate::movement::CardinalStep;
    let direction = match direction {
        StepDirection::North => CardinalStep::North,
        StepDirection::East => CardinalStep::East,
        StepDirection::South => CardinalStep::South,
        StepDirection::West => CardinalStep::West,
        StepDirection::NorthEast => CardinalStep::NorthEast,
        StepDirection::SouthEast => CardinalStep::SouthEast,
        StepDirection::SouthWest => CardinalStep::SouthWest,
        StepDirection::NorthWest => CardinalStep::NorthWest,
    };
    let position = |(x, y, z): (i32, i32, i32)| {
        Some(TilePos {
            x: u16::try_from(x).ok()?,
            y: u16::try_from(y).ok()?,
            floor: -i8::try_from(z).ok().filter(|z| (0..=15).contains(z))?,
        })
    };
    let ground = |cell| match position(cell) {
        Some(pos) => match world.base().tile(pos.x, pos.y, pos.floor) {
            None => Ground::Absent,
            Some(tile) if tile.walkable() && world.enterable(pos) => Ground::Walkable,
            Some(_) => Ground::Blocked,
        },
        None => Ground::Absent,
    };
    let next = resolve_landing(
        world.floors(),
        ground,
        |_| allow_teleport,
        (i32::from(from.x), i32::from(from.y), -i32::from(from.floor)),
        direction,
    )
    .ok()?;
    position(next)
}

fn near_locker(world: &BundleWorld, at: TilePos) -> bool {
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| {
            let Some((x, y)) = at.x.checked_add_signed(dx).zip(at.y.checked_add_signed(dy)) else {
                return false;
            };
            let pos = TilePos {
                x,
                y,
                floor: at.floor,
            };
            world.base().tile(x, y, at.floor).is_some_and(|tile| {
                (0..tile.depths().iter().filter(|depth| **depth == 0).count()).any(|ordinal| {
                    u8::try_from(ordinal)
                        .ok()
                        .is_some_and(|ordinal| world.facts().depot_at(pos, ordinal).is_some())
                })
            })
        })
    })
}

fn town_route_retry_delay(
    disposition: StepDisposition,
    stationary: bool,
    attempt: u8,
) -> Option<Duration> {
    (disposition == StepDisposition::Rejected && stationary && attempt <= 5)
        .then_some(Duration::from_millis(500))
}

struct HuntDeath {
    actor: crate::foundation::ExactActorRef,
    stamina: u16,
    reward_occurrence: Option<[u8; 16]>,
    shared: bool,
    committed_at_ms: i64,
    awards: BTreeMap<[u8; 16], u64>,
}

/// Match the durable Channel death key to the exact wire incarnation before deriving
/// its corpse identity. Creature type and a neighbouring floor tile are insufficient.
async fn committed_hunt_death(
    client: &mut oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    principal: Option<[u8; 16]>,
    key: &str,
    target: oteryn_dev_client::EntityRef,
    deadline: tokio::time::Instant,
) -> TestResult<HuntDeath> {
    use crate::foundation::{
        ActorLocalGeneration, ActorLocalId, ActorRef, ExactActorRef, ScopeOwnershipGeneration,
    };
    let world = WorldId::decode(&clients.world)?;
    let channel = ChannelId::decode(&clients.channel)?;
    let scope = ScopeOwnershipGeneration::new(clients.scope_generation)
        .map_err(|error| format!("invalid hunt scope generation: {error:?}"))?;
    let prefix = [
        clients.world.as_slice(),
        clients.channel.as_slice(),
        &clients.scope_generation.to_be_bytes(),
    ]
    .concat();
    type ReceiptRow = (Vec<u8>, i32, bool, Option<Vec<u8>>, i64);
    tokio::time::timeout_at(deadline, async {
        let mut database = sqlx::PgConnection::connect(clients.url).await?;
        loop {
            // The Channel death key is the exact byte sequence hashed by
            // ExactActorRef::placement_identity after its domain separator.
            let row: Option<ReceiptRow> = sqlx::query_as(
                "SELECT r.death_key, m.stamina_minutes, r.shared, uuid_send(m.reward_occurrence_id),
                        floor(EXTRACT(EPOCH FROM r.committed_at) * 1000)::bigint
                 FROM game_party_kill_xp_receipts r JOIN game_party_kill_xp_members m
                 ON m.death_key=r.death_key AND m.character_id=r.principal_id
                 WHERE ($1::bytea IS NULL OR r.principal_id=encode($1,'hex')::uuid)
                 AND r.creature_key=$2
                 AND octet_length(r.death_key)=52 AND substring(r.death_key FROM 1 FOR 40)=$3
                 AND substring(r.death_key FROM 45 FOR 8)=$4
                 AND substring(sha256(convert_to('oteryn:runtime-player-placement:v1','UTF8')
                     || r.death_key) FROM 1 FOR 16)=$5",
            )
            .bind(principal.as_ref().map(|id| id.as_slice()))
            .bind(key)
            .bind(&prefix)
            .bind(target.generation.to_be_bytes().as_slice())
            .bind(target.identity.as_slice())
            .fetch_optional(&mut database)
            .await?;
            if let Some((death_key, stamina, shared, reward_occurrence, committed_at_ms)) = row {
                let slot = death_key
                    .get(40..44)
                    .ok_or("missing death actor slot")?
                    .try_into()?;
                let actor = ExactActorRef(ActorRef::from_owner_parts(
                    world,
                    channel,
                    scope,
                    ActorLocalId(u32::from_be_bytes(slot)),
                    ActorLocalGeneration(target.generation),
                ));
                if actor.placement_identity() != target.identity {
                    return Err("durable death key did not match target".into());
                }
                let awards: Vec<(Vec<u8>, i64)> = sqlx::query_as(
                    "SELECT uuid_send(character_id), xp_awarded FROM game_party_kill_xp_members
                     WHERE death_key=$1 AND xp_awarded>0 LIMIT 51",
                )
                .bind(death_key)
                .fetch_all(&mut database)
                .await?;
                if awards.len() > 50 {
                    return Err("too many hunt award members".into());
                }
                return Ok(HuntDeath {
                    actor,
                    stamina: u16::try_from(stamina)?,
                    reward_occurrence: reward_occurrence
                        .map(|id| id.as_slice().try_into())
                        .transpose()?,
                    shared,
                    committed_at_ms,
                    awards: awards
                        .into_iter()
                        .map(|(id, xp)| {
                            TestResult::Ok((id.as_slice().try_into()?, u64::try_from(xp)?))
                        })
                        .collect::<TestResult<_>>()?,
                });
            }
            client.service_liveness(Duration::from_millis(100)).await?;
            client.take_events();
        }
    })
    .await
    .map_err(|_| "no committed death receipt for the spawned creature")?
}

pub(crate) fn parse_spawned_entity(answer: &str) -> TestResult<oteryn_dev_client::EntityRef> {
    let fields: Vec<_> = answer.split_ascii_whitespace().collect();
    let ["spawned", identity, generation] = fields.as_slice() else {
        return Err(format!("invalid hunt spawn response: {answer}").into());
    };
    let identity = crate::node::operator_files::hex_bytes::<16>(identity)
        .map_err(|_| "invalid hunt spawn identity")?;
    let generation = generation.parse::<std::num::NonZeroU64>()?.get();
    Ok(oteryn_dev_client::EntityRef {
        identity,
        generation,
    })
}

/// `spawn <session> <key>` through `provision.py --spawn`; Err carries the rejected token.
/// Shared by `creature_sweep` and MP09 (both reach it through `use super::*`).
async fn request_spawn(session: String, key: String) -> Result<String, String> {
    request_fixture_control(session, "--spawn", key).await
}

pub(crate) async fn request_fixture_control(
    session: String,
    operation: &'static str,
    argument: impl Into<Option<String>>,
) -> Result<String, String> {
    let argument = argument.into();
    let socket = required("NODE_BOOT_CONTROL_SOCKET").map_err(|error| error.to_string())?;
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let output = tokio::task::spawn_blocking(move || {
        crate::qualification::provision_python()
            .arg(script)
            .args(["--socket", &socket, "--session", &session, operation])
            .args(argument)
            .output()
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(match stderr.split_once("reason=") {
        Some((_, rest)) => rest
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect(),
        None => format!(
            "status={};stderr_tail={}",
            output.status,
            stderr
                .replace('\n', " | ")
                .chars()
                .rev()
                .take(400)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        ),
    })
}

async fn walk(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    walk_with_landings(client, path, |at, direction| {
        destination(world, at, direction)
    })
    .await
}

async fn walk_with_landings(
    client: &mut oteryn_dev_client::DevClientSession,
    path: &[StepDirection],
    landing: impl Fn(TilePos, StepDirection) -> Option<TilePos>,
) -> TestResult {
    walk_with_landing_budget(client, path, landing, None, None).await
}

async fn walk_with_landing_budget(
    client: &mut oteryn_dev_client::DevClientSession,
    path: &[StepDirection],
    landing: impl Fn(TilePos, StepDirection) -> Option<TilePos>,
    step_budget: Option<Duration>,
    detour_center: Option<TilePos>,
) -> TestResult {
    let mut path = VecDeque::from(path.to_vec());
    let mut blocked = std::collections::BTreeSet::new();
    while let Some(direction) = path.pop_front() {
        let before = client.world_spatial().actor_position;
        let movement = async {
            let mut outcome = client.step_retrying(direction).await?;
            for attempt in 1..=6 {
                if outcome.disposition != StepDisposition::Rejected
                    || client.world_spatial().actor_position != before
                {
                    break;
                }
                evidence(&format!(
                    "town_route step_rejected attempt={attempt} direction={direction:?} from={before:?} expected={:?} actual={:?}",
                    landing(tile(before)?, direction),
                    client.world_spatial().actor_position,
                ));
                let Some(delay) = town_route_retry_delay(
                    outcome.disposition,
                    client.world_spatial().actor_position == before,
                    attempt,
                ) else {
                    break;
                };
                tokio::time::sleep(delay).await;
                outcome = client.step_retrying(direction).await?;
            }
            TestResult::Ok(outcome)
        };
        let outcome = match step_budget {
            Some(budget) => tokio::time::timeout(budget, movement).await.map_err(|_| {
                format!("town route step deadline direction={direction:?} from={before:?}")
            })?,
            None => movement.await,
        }?;
        let from = tile(before)?;
        let expected = landing(from, direction);
        if outcome.disposition == StepDisposition::Blocked
            && client.world_spatial().actor_position == before
            && let Some(occupied) = expected
            && let Some(entities) = client.world_entities()
        {
            use oteryn_dev_client::EntityKind;
            let visible_blockers: std::collections::BTreeSet<_> = entities
                .iter()
                .filter(|entity| {
                    &entity.entity.identity != entities.own_identity()
                        && matches!(
                            entity.kind,
                            EntityKind::Creature | EntityKind::Npc | EntityKind::Player
                        )
                })
                .filter_map(|entity| tile(entity.position).ok())
                .collect();
            if visible_blockers.contains(&occupied)
                && blocked.len() < 8
                && blocked.insert(occupied)
                && let Some(detour) = route_around_occupant(
                    from,
                    detour_center.unwrap_or(from),
                    occupied,
                    &path,
                    |at| !blocked.contains(&at) && !visible_blockers.contains(&at),
                    &landing,
                )
            {
                path = detour.into();
                continue;
            }
        }
        if outcome.disposition != StepDisposition::Moved || outcome.world_spatial_delta.is_none() {
            return Err(format!(
                "town route step {direction:?} from={before:?} expected={:?} actual={:?}: {outcome:?}",
                expected,
                client.world_spatial().actor_position,
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        let after = client.world_spatial().actor_position;
        if Some(tile(after)?) != expected {
            return Err(format!(
                "town step changed position unexpectedly: {before:?} -> {after:?}"
            )
            .into());
        }
        client.take_events();
    }
    Ok(())
}

pub(super) fn is_logout_rest_tile(world: &BundleWorld, at: TilePos) -> bool {
    world
        .base()
        .tile(at.x, at.y, at.floor)
        .is_some_and(|tile| tile.protection_zone() && !tile.no_logout())
}

pub(super) async fn logout_retrying_busy(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<(
    Option<oteryn_protocol_oteryn::logout::LogoutResult>,
    String,
    usize,
    bool,
)> {
    logout_retrying_busy_observed(client, None).await
}

type LogoutObserver<'a> = dyn FnMut(usize, &str) + 'a;

pub(super) async fn logout_retrying_busy_observed(
    client: &mut oteryn_dev_client::DevClientSession,
    mut observe: Option<&mut LogoutObserver<'_>>,
) -> TestResult<(
    Option<oteryn_protocol_oteryn::logout::LogoutResult>,
    String,
    usize,
    bool,
)> {
    use oteryn_protocol_oteryn::logout::LogoutResult;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut attempts = 0;
    loop {
        attempts += 1;
        let outcome = client.session_mut().logout().await.map_err(|error| {
            if let Some(observe) = observe.as_mut() {
                observe(attempts, &format!("command_error={error}"));
            }
            format!("logout attempt {attempts} failed: {error}")
        })?;
        if let Some(observe) = observe.as_mut() {
            observe(attempts, &format!("reply={outcome:?}"));
        }
        if outcome.result != Some(LogoutResult::Busy) {
            return Ok((outcome.result, format!("{outcome:?}"), attempts, false));
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok((outcome.result, format!("{outcome:?}"), attempts, true));
        }
        client.service_liveness(Duration::from_millis(250)).await
            .map_err(|error| {
                if let Some(observe) = observe.as_mut() {
                    observe(attempts, &format!("busy_liveness_error={error}"));
                }
                format!("logout Busy retry liveness failed after attempt {attempts}: {outcome:?}; error={error}")
            })?;
    }
}

struct WorldFixturePair {
    accounts: [String; 2],
    characters: [[u8; 16]; 2],
}

impl WorldFixturePair {
    fn clients<'a>(&'a self, scope: &SeamClients<'a>) -> SeamClients<'a> {
        SeamClients {
            accounts: &self.accounts,
            characters: &self.characters,
            ..*scope
        }
    }
}

async fn extra_world_fixtures(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
) -> TestResult<[WorldFixturePair; 3]> {
    let configured = required("NODE_BOOT_EXTRA_WORLD_ACCOUNTS")?;
    let ids: [String; 6] = configured
        .split(',')
        .map(str::to_owned)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "expected six dedicated MP and sweep Accounts")?;
    let mut accounts = std::collections::BTreeSet::new();
    let mut characters = std::collections::BTreeSet::new();
    for pair in [core, packets] {
        for account in pair.accounts {
            accounts.insert(crate::node::uuid_bytes(account).ok_or("invalid fixture Account")?);
        }
        characters.extend(pair.characters.iter().copied());
    }
    let mut database = sqlx::PgConnection::connect(core.url).await?;
    let mut resolved = Vec::new();
    let [a, b, c, d, e, f] = ids;
    for pair_accounts in [[a, b], [c, d], [e, f]] {
        let mut pair = WorldFixturePair {
            accounts: pair_accounts,
            characters: [[0; 16]; 2],
        };
        for (account, character) in pair.accounts.iter().zip(&mut pair.characters) {
            if !accounts.insert(crate::node::uuid_bytes(account).ok_or("invalid fixture Account")?)
            {
                return Err("dedicated World fixture reused an Account".into());
            }
            let rows: Vec<Vec<u8>> = sqlx::query_scalar(
                "SELECT uuid_send(character_id) FROM game_character_roots
                 WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid",
            )
            .bind(account)
            .bind(core.world.as_slice())
            .fetch_all(&mut database)
            .await?;
            let [row] = rows.as_slice() else {
                return Err("dedicated World fixture must own one Character".into());
            };
            *character = row.as_slice().try_into()?;
            if !characters.insert(*character) {
                return Err("dedicated World fixture reused a Character".into());
            }
        }
        resolved.push(pair);
    }
    database.close().await?;
    resolved
        .try_into()
        .map_err(|_| "dedicated World fixture count".into())
}

pub(super) async fn journeys(
    clients: &SeamClients<'_>,
    packet_clients: &SeamClients<'_>,
    path: &Path,
) -> TestResult {
    let started = std::time::Instant::now();
    let bytes = std::fs::read(path)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let generation = load_world_generation(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let world = &generation.bundle;
    evidence(&format!(
        "world_loader state=booted duration_ms={} start={TOWN:?}",
        started.elapsed().as_millis()
    ));
    let groups = verdict::Groups::from_env()?;
    // The required core keeps its ordinary pair; packet journeys own a separate pair.
    let mut client = connect_world(clients, 0x70).await?;
    let entered_at = tile(client.world_spatial().actor_position)?;
    evidence(&format!(
        "world_fixture role=core phase=first_entry position={entered_at:?}"
    ));
    if entered_at != TOWN {
        return Err(format!(
            "Platform-created character did not enter the configured town: expected={TOWN:?} actual={entered_at:?}"
        )
        .into());
    }
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("first town entry logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    drop(client);
    {
        let clients = packet_clients;
        // Default runs retain the ordinary packets alongside the indexed rows.
        // Explicit groups can isolate those packets from the row batch.
        include!("full_e2ef6_mount.rs");
        if groups.runs("rows") {
            release_row_characters(clients).await?;
            if e2e_row_selected(1) {
                verdict::indexed("C03.full", async {
                    full_row_c03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(2) {
                // Requires current Premium.
                verdict::indexed("C12.full", async {
                    full_row_c12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(3) {
                verdict::indexed("C09.full", async {
                    full_row_c09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(4) {
                verdict::indexed("C06.full", async {
                    full_row_c06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(5) {
                verdict::indexed("C02.full", async {
                    full_row_c02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(6) {
                verdict::indexed("C01.full", async {
                    full_row_c01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(7) {
                verdict::indexed("W22b.full", async {
                    full_row_w22b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(8) {
                verdict::indexed("C05.full", async {
                    full_row_c05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(9) {
                verdict::indexed("C04.full", async {
                    full_row_c04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(10) {
                verdict::indexed("W18.full", async {
                    full_row_w18::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(11) {
                verdict::indexed("C11.full", async {
                    full_row_c11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(12) {
                verdict::indexed("W19a.full", async {
                    full_row_w19a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(13) {
                verdict::indexed("W01b.full", async {
                    full_row_w01b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(14) {
                verdict::indexed("W23.full", async {
                    full_row_w23::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(15) {
                verdict::indexed("W15d.full", async {
                    full_row_w15d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(16) {
                verdict::indexed("W15a.full", async {
                    full_row_w15a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(17) {
                verdict::indexed("W10.full", async {
                    full_row_w10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(18) {
                verdict::indexed("W08.full", async {
                    full_row_w08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(19) {
                verdict::indexed("W24.full", async {
                    full_row_w24::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(20) {
                verdict::indexed("W22a.full", async {
                    full_row_w22a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(21) {
                verdict::indexed("W15b.full", async {
                    full_row_w15b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(22) {
                verdict::indexed("W11.full", async {
                    full_row_w11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(23) {
                verdict::indexed("W05a.full", async {
                    full_row_w05a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(24) {
                verdict::indexed("W03.full", async {
                    full_row_w03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(25) {
                verdict::indexed("W01a.full", async {
                    full_row_w01a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(26) {
                verdict::indexed("S16b.full", async {
                    full_row_s16b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(27) {
                verdict::indexed("S15b.full", async {
                    full_row_s15b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(28) {
                verdict::indexed("S10.full", async {
                    full_row_s10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(29) {
                // Requires current Premium.
                verdict::indexed("S07.full", async {
                    full_row_s07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(30) {
                verdict::indexed("S06.full", async {
                    full_row_s06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(31) {
                // Requires current Premium.
                verdict::indexed("S05.full", async {
                    full_row_s05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(32) {
                verdict::indexed("P07.full", async {
                    full_row_p07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(33) {
                verdict::indexed("W17.full", async {
                    full_row_w17::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(34) {
                verdict::indexed("W16.full", async {
                    full_row_w16::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(35) {
                verdict::indexed("W13.full", async {
                    full_row_w13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(36) {
                // Requires current Premium.
                verdict::indexed("W12.full", async {
                    full_row_w12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(37) {
                verdict::indexed("W05c.full", async {
                    full_row_w05c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(38) {
                verdict::indexed("W04.full", async {
                    full_row_w04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(39) {
                verdict::indexed("W02.full", async {
                    full_row_w02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(40) {
                verdict::indexed("W01c.full", async {
                    full_row_w01c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(41) {
                // Requires current Premium.
                verdict::indexed("S13c.full", async {
                    full_row_s13c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(42) {
                verdict::indexed("S04c.full", async {
                    full_row_s04c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(43) {
                verdict::indexed("S04a.full", async {
                    full_row_s04a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(44) {
                verdict::indexed("S02.full", async {
                    full_row_s02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(45) {
                verdict::indexed("P23c.full", async {
                    full_row_p23c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(46) {
                // Requires current Premium.
                verdict::indexed("P20.full", async {
                    full_row_p20::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(47) {
                verdict::indexed("P18.full", async {
                    full_row_p18::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(48) {
                verdict::indexed("P17.full", async {
                    full_row_p17::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(49) {
                verdict::indexed("P15.full", async {
                    full_row_p15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(50) {
                // Requires current Premium.
                verdict::indexed("P14.full", async {
                    full_row_p14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(51) {
                verdict::indexed("P13.full", async {
                    full_row_p13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(52) {
                verdict::indexed("P12c.full", async {
                    full_row_p12c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(53) {
                verdict::indexed("P12a.full", async {
                    full_row_p12a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(54) {
                verdict::indexed("P11.full", async {
                    full_row_p11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(55) {
                verdict::indexed("P09.full", async {
                    full_row_p09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(56) {
                verdict::indexed("P08.full", async {
                    full_row_p08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(57) {
                // Requires current Premium.
                verdict::indexed("P06.full", async {
                    full_row_p06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(58) {
                verdict::indexed("P05.full", async {
                    full_row_p05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(59) {
                verdict::indexed("P04.full", async {
                    full_row_p04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(60) {
                verdict::indexed("P03.full", async {
                    full_row_p03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(61) {
                verdict::indexed("P02.full", async {
                    full_row_p02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(62) {
                verdict::indexed("P01.full", async {
                    full_row_p01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(63) {
                verdict::indexed("E10.full", async {
                    full_row_e10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(64) {
                verdict::indexed("E09.full", async {
                    full_row_e09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(65) {
                verdict::indexed("E08.full", async {
                    full_row_e08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(66) {
                // Requires current Premium.
                verdict::indexed("E06.full", async {
                    full_row_e06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(67) {
                verdict::indexed("I11.full", async {
                    full_row_i11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(68) {
                verdict::indexed("I10.full", async {
                    full_row_i10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(69) {
                verdict::indexed("I05.full", async {
                    full_row_i05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(70) {
                verdict::indexed("M11b.full", async {
                    full_row_m11b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(71) {
                verdict::indexed("M11a.full", async {
                    full_row_m11a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(72) {
                verdict::indexed("M08.full", async {
                    full_row_m08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(73) {
                verdict::indexed("C18b.full", async {
                    full_row_c18b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(74) {
                verdict::indexed("C17b.full", async {
                    full_row_c17b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(75) {
                verdict::indexed("C10.full", async {
                    full_row_c10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(76) {
                verdict::indexed("C07.full", async {
                    full_row_c07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(77) {
                verdict::indexed("E12.full", async {
                    full_row_e12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(78) {
                verdict::indexed("I14.full", async {
                    full_row_i14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(79) {
                verdict::indexed("I03.full", async {
                    full_row_i03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(80) {
                verdict::indexed("M10.full", async {
                    full_row_m10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(81) {
                verdict::indexed("M09.full", async {
                    full_row_m09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(82) {
                verdict::indexed("M06.full", async {
                    full_row_m06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(83) {
                verdict::indexed("C15.full", async {
                    full_row_c15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(84) {
                verdict::indexed("C08.full", async {
                    full_row_c08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(85) {
                full_e2ef19::run_packet(clients).await;
                release_row_characters(clients).await?;
            }
            if e2e_row_selected(86) {
                full_e2ef18::run_packet(clients).await;
                release_row_characters(clients).await?;
            }
            if e2e_row_selected(87) {
                full_e2ef20::run_packet(clients).await;
                release_row_characters(clients).await?;
            }
            if e2e_row_selected(88) {
                verdict::indexed("C01.client", async {
                    client_row_c01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(89) {
                verdict::indexed("C02.client", async {
                    client_row_c02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(90) {
                verdict::indexed("C03.client", async {
                    client_row_c03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(91) {
                verdict::indexed("C04.client", async {
                    client_row_c04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(92) {
                verdict::indexed("C05.client", async {
                    client_row_c05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(93) {
                verdict::indexed("C06.client", async {
                    client_row_c06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(227) {
                client_row_c07::run_client_packet(clients).await;
                release_row_characters(clients).await?;
            }
            if e2e_row_selected(94) {
                verdict::indexed("C09.client", async {
                    client_row_c09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(95) {
                verdict::indexed("C10.client", async {
                    client_row_c10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(96) {
                verdict::indexed("C11.client", async {
                    client_row_c11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(97) {
                verdict::indexed("C12.client", async {
                    client_row_c12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(98) {
                verdict::indexed("C13.client", async {
                    client_row_c13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(99) {
                verdict::indexed("C14.client", async {
                    client_row_c14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(100) {
                verdict::indexed("C15.client", async {
                    client_row_c15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(101) {
                verdict::indexed("C16a.client", async {
                    client_row_c16a::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(102) {
                verdict::indexed("C16b.client", async {
                    client_row_c16b::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(103) {
                verdict::indexed("C16c.client", async {
                    client_row_c16c::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(104) {
                verdict::indexed("C16d.client", async {
                    client_row_c16d::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(105) {
                verdict::indexed("C17a.client", async {
                    client_row_c17a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(106) {
                verdict::indexed("C17b.client", async {
                    client_row_c17b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(107) {
                verdict::indexed("C17d.client", async {
                    client_row_c17d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(108) {
                verdict::indexed("C18a.client", async {
                    client_row_c18a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(109) {
                verdict::indexed("C18b.client", async {
                    client_row_c18b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(228) {
                verdict::indexed("C20a.client", async {
                    client_row_c20a::run_packet(clients, world, &metadata.spawns).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(110) {
                verdict::indexed("C20b.client", async {
                    client_row_c20b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(111) {
                verdict::indexed("C20c.client", async {
                    client_row_c20c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(229) {
                verdict::indexed("C20d.client", async {
                    client_row_c20d::run_packet(clients, world, &metadata.spawns).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(112) {
                verdict::indexed("E01.client", async {
                    client_row_e01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(113) {
                verdict::indexed("I01.client", async {
                    client_row_i01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(114) {
                verdict::indexed("I03.client", async {
                    client_row_i03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(115) {
                verdict::indexed("I04.client", async {
                    client_row_i04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(116) {
                verdict::indexed("I05.client", async {
                    client_row_i05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(117) {
                verdict::indexed("I06.client", async {
                    client_row_i06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(118) {
                // Requires current Premium.
                verdict::indexed("I07.client", async {
                    client_row_i07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(119) {
                verdict::indexed("I08.client", async {
                    client_row_i08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(120) {
                verdict::indexed("I09.client", async {
                    client_row_i09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(121) {
                verdict::indexed("I10.client", async {
                    client_row_i10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(122) {
                verdict::indexed("I11.client", async {
                    client_row_i11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(123) {
                verdict::indexed("I12.client", async {
                    client_row_i12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(124) {
                verdict::indexed("I13.client", async {
                    client_row_i13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(125) {
                verdict::indexed("I14.client", async {
                    client_row_i14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(126) {
                verdict::indexed("I15.client", async {
                    client_row_i15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(127) {
                verdict::indexed("I16.client", async {
                    client_row_i16::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(128) {
                verdict::indexed("M01.client", async {
                    client_row_m01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(129) {
                verdict::indexed("M02.client", async {
                    client_row_m02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(130) {
                verdict::indexed("M03.client", async {
                    client_row_m03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(131) {
                verdict::indexed("M04.client", async {
                    client_row_m04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(132) {
                verdict::indexed("M05.client", async {
                    client_row_m05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(133) {
                verdict::indexed("M06.client", async {
                    client_row_m06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(134) {
                verdict::indexed("M07.client", async {
                    client_row_m07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(135) {
                verdict::indexed("M08.client", async {
                    client_row_m08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(136) {
                verdict::indexed("M09.client", async {
                    client_row_m09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(137) {
                verdict::indexed("M10.client", async {
                    client_row_m10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(138) {
                verdict::indexed("M11a.client", async {
                    client_row_m11a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(139) {
                verdict::indexed("M11b.client", async {
                    client_row_m11b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(140) {
                verdict::indexed("N01.client", async {
                    client_row_n01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(141) {
                verdict::indexed("N02.client", async {
                    client_row_n02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(142) {
                verdict::indexed("N03.client", async {
                    client_row_n03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(143) {
                verdict::indexed("N04.client", async {
                    client_row_n04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(144) {
                verdict::indexed("C08.client", async {
                    client_row_c08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(145) {
                verdict::indexed("C17c.client", async {
                    client_row_c17c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(146) {
                verdict::indexed("C19a.client", async {
                    client_row_c19a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(147) {
                verdict::indexed("E02.client", async {
                    client_row_e02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(148) {
                verdict::indexed("E03.client", async {
                    client_row_e03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(149) {
                verdict::indexed("E05.client", async {
                    client_row_e05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(150) {
                verdict::indexed("E06.client", async {
                    client_row_e06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(151) {
                verdict::indexed("E07.client", async {
                    client_row_e07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(152) {
                verdict::indexed("E08.client", async {
                    client_row_e08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(153) {
                verdict::indexed("E09.client", async {
                    client_row_e09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(154) {
                verdict::indexed("E10.client", async {
                    client_row_e10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(155) {
                verdict::indexed("E12.client", async {
                    client_row_e12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(156) {
                verdict::indexed("P01.client", async {
                    client_row_p01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(157) {
                verdict::indexed("P02.client", async {
                    client_row_p02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(158) {
                verdict::indexed("P03.client", async {
                    client_row_p03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(159) {
                verdict::indexed("P04.client", async {
                    client_row_p04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(160) {
                verdict::indexed("P05.client", async {
                    client_row_p05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(161) {
                verdict::indexed("P06.client", async {
                    client_row_p06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(162) {
                verdict::indexed("P07.client", async {
                    client_row_p07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(163) {
                verdict::indexed("P08.client", async {
                    client_row_p08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(164) {
                verdict::indexed("P09.client", async {
                    client_row_p09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(165) {
                verdict::indexed("P10.client", async {
                    client_row_p10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(166) {
                verdict::indexed("P11.client", async {
                    client_row_p11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(167) {
                verdict::indexed("P12a.client", async {
                    client_row_p12a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(168) {
                verdict::indexed("P12c.client", async {
                    client_row_p12c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(169) {
                verdict::indexed("P13.client", async {
                    client_row_p13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(170) {
                verdict::indexed("P14.client", async {
                    client_row_p14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(171) {
                verdict::indexed("P15.client", async {
                    client_row_p15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(172) {
                verdict::indexed("P16.client", async {
                    client_row_p16::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(173) {
                verdict::indexed("P17.client", async {
                    client_row_p17::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(174) {
                verdict::indexed("P18.client", async {
                    client_row_p18::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(175) {
                verdict::indexed("P19.client", async {
                    client_row_p19::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(176) {
                verdict::indexed("P20.client", async {
                    client_row_p20::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(177) {
                verdict::indexed("P21.client", async {
                    client_row_p21::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(178) {
                verdict::indexed("P23c.client", async {
                    client_row_p23c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(179) {
                verdict::indexed("S01.client", async {
                    client_row_s01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(180) {
                verdict::indexed("S02.client", async {
                    client_row_s02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(181) {
                verdict::indexed("S03.client", async {
                    client_row_s03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(182) {
                verdict::indexed("S04a.client", async {
                    client_row_s04a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(183) {
                verdict::indexed("S04b.client", async {
                    client_row_s04b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(184) {
                verdict::indexed("S18.client", async {
                    client_row_s18::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(185) {
                verdict::indexed("W01a.client", async {
                    client_row_w01a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(186) {
                verdict::indexed("W01b.client", async {
                    client_row_w01b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(187) {
                verdict::indexed("W01c.client", async {
                    client_row_w01c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(188) {
                verdict::indexed("W02.client", async {
                    client_row_w02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(189) {
                verdict::indexed("W03.client", async {
                    client_row_w03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(190) {
                verdict::indexed("W04.client", async {
                    client_row_w04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(191) {
                verdict::indexed("W05a.client", async {
                    client_row_w05a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(192) {
                verdict::indexed("W05c.client", async {
                    client_row_w05c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(193) {
                verdict::indexed("W10.client", async {
                    client_row_w10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(194) {
                verdict::indexed("W11.client", async {
                    client_row_w11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(195) {
                verdict::indexed("W12.client", async {
                    client_row_w12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(196) {
                verdict::indexed("W13.client", async {
                    client_row_w13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(197) {
                verdict::indexed("S06.client", async {
                    client_row_s06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(198) {
                verdict::indexed("S08.client", async {
                    client_row_s08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(199) {
                // Requires current Premium.
                verdict::indexed("S13c.client", async {
                    client_row_s13c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(200) {
                // Requires current Premium.
                verdict::indexed("S13b.client", async {
                    client_row_s13b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(201) {
                // Requires current Premium.
                verdict::indexed("S13a.client", async {
                    client_row_s13a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(202) {
                verdict::indexed("S15b.client", async {
                    client_row_s15b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(203) {
                verdict::indexed("S04c.client", async {
                    client_row_s04c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(204) {
                verdict::indexed("S07.client", async {
                    client_row_s07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(205) {
                verdict::indexed("S16b.client", async {
                    client_row_s16b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(206) {
                verdict::indexed("S09.client", async {
                    client_row_s09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(207) {
                verdict::indexed("S05.client", async {
                    client_row_s05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(208) {
                verdict::indexed("S10.client", async {
                    client_row_s10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(209) {
                verdict::indexed("S16a.client", async {
                    client_row_s16a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(210) {
                verdict::indexed("S11.client", async {
                    client_row_s11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(211) {
                verdict::indexed("W15a.client", async {
                    client_row_w15a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(212) {
                verdict::indexed("W15b.client", async {
                    client_row_w15b::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(213) {
                verdict::indexed("W15d.client", async {
                    client_row_w15d::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(214) {
                verdict::indexed("W16.client", async {
                    client_row_w16::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(215) {
                verdict::indexed("W17.client", async {
                    client_row_w17::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(216) {
                verdict::indexed("W19a.client", async {
                    client_row_w19a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(217) {
                verdict::indexed("W20a.client", async {
                    client_row_w20a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(218) {
                verdict::indexed("W20b.client", async {
                    client_row_w20b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(219) {
                verdict::indexed("W22a.client", async {
                    client_row_w22a::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(220) {
                verdict::indexed("W22b.client", async {
                    client_row_w22b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(221) {
                verdict::indexed("W23.client", async {
                    client_row_w23::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(222) {
                verdict::indexed("W24.client", async {
                    client_row_w24::run_packet(clients, world).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(223) {
                verdict::indexed("N07a.client", async {
                    client_row_n07a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(224) {
                verdict::indexed("N07b.client", async {
                    client_row_n07b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(225) {
                verdict::indexed("N07c.client", async {
                    client_row_n07c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(226) {
                verdict::indexed("N07d.client", async {
                    client_row_n07d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
        }
        if groups.runs("f5") {
            full_e2ef5::run(clients, world).await;
        }
        for index in 0..clients.accounts.len() {
            wait_character_released(clients, index).await?;
        }
        if groups.runs("f4") {
            include!("full_e2ef4_mount.rs");
        }
        // F4 can drop its socket; require terminal release before the next journey.
        for index in 0..clients.accounts.len() {
            wait_character_released(clients, index).await?;
        }
        if groups.runs("rows") {
            if e2e_row_selected(233) {
                verdict::indexed("G01.full", async {
                    full_row_g01::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(234) {
                verdict::indexed("G02.full", async {
                    full_row_g02::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(235) {
                verdict::indexed("G03.full", async {
                    full_row_g03::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(236) {
                verdict::indexed("G04.full", async {
                    full_row_g04::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(237) {
                verdict::indexed("G05.full", async {
                    full_row_g05::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(238) {
                verdict::indexed("G06.full", async {
                    full_row_g06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(239) {
                verdict::indexed("G07.full", async {
                    full_row_g07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(240) {
                verdict::indexed("G08.full", async {
                    full_row_g08::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(241) {
                verdict::indexed("G09.full", async {
                    full_row_g09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(242) {
                verdict::indexed("G10.full", async {
                    full_row_g10::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(243) {
                verdict::indexed("G11.full", async {
                    full_row_g11::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(244) {
                verdict::indexed("G12.full", async {
                    full_row_g12::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(245) {
                verdict::indexed("G13.full", async {
                    full_row_g13::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(246) {
                verdict::indexed("G14.full", async {
                    full_row_g14::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(247) {
                verdict::indexed("G15.full", async {
                    full_row_g15::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(248) {
                verdict::indexed("G16.full", async {
                    full_row_g16::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(249) {
                verdict::indexed("G17.full", async {
                    full_row_g17::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(250) {
                verdict::indexed("G18.full", async {
                    full_row_g18::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(251) {
                verdict::indexed("G19.full", async {
                    full_row_g19::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(252) {
                verdict::indexed("G20.full", async {
                    full_row_g20::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(253) {
                verdict::indexed("G21.full", async {
                    full_row_g21::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(254) {
                verdict::indexed("G22.full", async {
                    full_row_g22::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(255) {
                verdict::indexed("G23.full", async {
                    full_row_g23::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(256) {
                verdict::indexed("G24.full", async {
                    full_row_g24::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(257) {
                verdict::indexed("G25.full", async {
                    full_row_g25::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(258) {
                verdict::indexed("G26.full", async {
                    full_row_g26::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(259) {
                verdict::indexed("G27.full", async {
                    full_row_g27::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(260) {
                verdict::indexed("G28.full", async {
                    full_row_g28::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(261) {
                verdict::indexed("G29.full", async {
                    full_row_g29::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(262) {
                verdict::indexed("G30.full", async {
                    full_row_g30::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(263) {
                verdict::indexed("G31.full", async {
                    full_row_g31::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(264) {
                verdict::indexed("G32.full", async {
                    full_row_g32::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(265) {
                verdict::indexed("G33.full", async {
                    full_row_g33::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(266) {
                verdict::indexed("G34.full", async {
                    full_row_g34::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(267) {
                verdict::indexed("G35.full", async {
                    full_row_g35::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(268) {
                verdict::indexed("G36.full", async {
                    full_row_g36::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(269) {
                verdict::indexed("C19b.full", async {
                    full_row_c19b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(270) {
                verdict::indexed("I17a.full", async {
                    full_row_i17a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(271) {
                verdict::indexed("I17b.full", async {
                    full_row_i17b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(272) {
                verdict::indexed("I17c.full", async {
                    full_row_i17c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(273) {
                verdict::indexed("I17d.full", async {
                    full_row_i17d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(274) {
                verdict::indexed("I18a.full", async {
                    full_row_i18a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(275) {
                verdict::indexed("I18b.full", async {
                    full_row_i18b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(276) {
                verdict::indexed("I18c.full", async {
                    full_row_i18c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(277) {
                verdict::indexed("I19.full", async {
                    full_row_i19::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(278) {
                verdict::indexed("P12b.full", async {
                    full_row_p12b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(279) {
                verdict::indexed("P22a.full", async {
                    full_row_p22a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(280) {
                verdict::indexed("P22b.full", async {
                    full_row_p22b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(281) {
                verdict::indexed("P22c.full", async {
                    full_row_p22c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(282) {
                verdict::indexed("P22d.full", async {
                    full_row_p22d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(283) {
                verdict::indexed("S14a.full", async {
                    full_row_s14a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(284) {
                verdict::indexed("S14b.full", async {
                    full_row_s14b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(285) {
                verdict::indexed("S14c.full", async {
                    full_row_s14c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(286) {
                verdict::indexed("S15a.full", async {
                    full_row_s15a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(287) {
                verdict::indexed("S16c.full", async {
                    full_row_s16c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(288) {
                verdict::indexed("S16d.full", async {
                    full_row_s16d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(289) {
                verdict::indexed("S17a.full", async {
                    full_row_s17a::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(290) {
                verdict::indexed("S17b.full", async {
                    full_row_s17b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(291) {
                verdict::indexed("S17c.full", async {
                    full_row_s17c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(292) {
                verdict::indexed("S19.full", async {
                    full_row_s19::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(293) {
                verdict::indexed("W05b.full", async {
                    full_row_w05b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(294) {
                verdict::indexed("W05d.full", async {
                    full_row_w05d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(295) {
                verdict::indexed("W06.full", async {
                    full_row_w06::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(296) {
                verdict::indexed("W07.full", async {
                    full_row_w07::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(297) {
                verdict::indexed("W09.full", async {
                    full_row_w09::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(298) {
                // Requires current Premium.
                verdict::indexed("W15c.full", async {
                    full_row_w15c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(299) {
                verdict::indexed("W19b.full", async {
                    full_row_w19b::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(300) {
                verdict::indexed("W19c.full", async {
                    full_row_w19c::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(301) {
                verdict::indexed("W19d.full", async {
                    full_row_w19d::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(302) {
                verdict::indexed("W19e.full", async {
                    full_row_w19e::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(303) {
                verdict::indexed("W21.full", async {
                    full_row_w21::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(304) {
                verdict::indexed("G37.full", async {
                    full_row_g37::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
            if e2e_row_selected(305) {
                verdict::indexed("G38.full", async {
                    full_row_g38::run_packet(clients).await;
                    release_row_characters(clients).await
                })
                .await;
            }
        }
        for index in 0..clients.accounts.len() {
            wait_character_released(clients, index).await?;
        }
        evidence(&format!(
            "world_fixture role=packets phase=terminal_release accounts={}",
            clients.accounts.len()
        ));
    }
    let mp_selected = groups.runs("mp");
    let sweep_selected = groups.runs("creature_sweep")
        && std::env::var("NODE_BOOT_E2E_ONLY").is_ok_and(|list| !list.trim().is_empty());
    if mp_selected || sweep_selected {
        let [mp_first, mp_second, sweep] = extra_world_fixtures(clients, packet_clients).await?;
        if sweep_selected {
            creature_sweep::run(&sweep.clients(clients), world, &generation.rewards).await?;
        }
        if mp_selected {
            mp::run(
                &mp_first.clients(clients),
                &mp_second.clients(clients),
                world,
                &metadata.manifest.subareas,
            )
            .await;
        }
    }
    // The core/profile pair has not inherited packet movement, loot or death.
    if !groups.runs("core") {
        evidence("e2e_groups core=skipped");
        return Ok(());
    }
    let profile_result = profile_journey(clients, world, &bytes, &metadata.spawns).await;
    if let Err(error) = &profile_result {
        evidence(&format!("world_profile state=failed error={error}"));
    }
    let mut client = connect_world(clients, 0x71).await?;
    for capability in [17, 18, 151, 152, 153, 179, 230] {
        if !client.selected_capabilities().contains(&capability) {
            return Err(
                format!("qualified World did not offer journey capability {capability}").into(),
            );
        }
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("town join has no production bundle map snapshot")?;
    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(|error| format!("town map snapshot: {error:?}"))?;
    if map.header.bundle_digest != world.base().digest() || map.tiles.is_empty() {
        return Err("town join served a different or empty World bundle".into());
    }
    // Resume from the profile journey's saved cell and travel normally to town.
    let from = tile(client.world_spatial().actor_position)?;
    evidence(&format!(
        "world_fixture role=core phase=return_to_town from={from:?} to={TOWN:?}"
    ));
    let back = route(world, from, |at| at == TOWN).ok_or_else(|| {
        format!("required core has no return route to town: from={from:?} to={TOWN:?}")
    })?;
    walk(&mut client, world, &back).await?;
    client = character_core(client, clients, world).await?;
    client = include!("full_e2ef3_registration.rs");
    client = floor_journey(client, clients, world).await?;
    client_guild::guild(&mut client).await?;
    client_party::party(&mut client).await?;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let buddy: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_one(&mut connection)
    .await?;
    client_vip::add(&mut client, &buddy).await?;
    client_player_trade::pair(&mut client, clients).await?;
    client_prey::prey(&mut client).await?;
    client_task_board::task_board(&mut client).await?;
    client_imbue::imbue(&mut client).await?;
    client_hireling::hireling(&mut client).await?;
    client_browse_field::browse_field(&mut client).await?;
    client_daily_boost::daily_boost(&mut client).await?;
    let path = route(world, TOWN, |at| near_locker(world, at))
        .ok_or("town has no reachable authored depot locker")?;
    walk(&mut client, world, &path).await?;
    client_depot_search::depot_search(&mut client).await?;
    client_stash::stash(&mut client).await?;
    client_market::market(&mut client).await?;
    shop_journey(&mut client, world, &metadata.npcs, clients).await?;
    client_hotkey_equip::hotkey_equip(&mut client).await?;
    client_item_forge::item_forge(&mut client).await?;
    client_store::store(&mut client).await?;
    let after = destination(
        world,
        tile(client.world_spatial().actor_position)?,
        StepDirection::West,
    )
    .map(|at| ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    });
    client = client_disconnect_resume::recover(client, clients, after).await?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    client_hunt::hunt(&mut client).await?;
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await?;
    client_death::death(&mut client, clients).await?;
    let respawn = client.world_spatial().actor_position;
    let old_session = client.join_snapshot().game_session_id;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let logout = client.session_mut().logout().await?;
        if logout.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("death relog logout was not accepted: {logout:?}").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    drop(client);
    client = connect_world(clients, 0x73).await?;
    if client.join_snapshot().game_session_id == old_session
        || client.world_spatial().actor_position != respawn
        || client
            .actor_vitals()
            .is_none_or(|vitals| vitals.health != vitals.max_health)
    {
        return Err("death relog did not retain the saved respawn position and full vitals".into());
    }
    evidence(&format!(
        "mechanic=death state=relogged position={respawn:?} health={}",
        client.actor_vitals().ok_or("death relog vitals")?.health
    ));

    drop(client);
    profile_result
}

async fn profile_journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    bytes: &[u8],
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    // Catalogue classification takes several seconds in a debug build. Prepare it
    // before admission so the peer can keep acknowledging probes during the journey.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let model = map_journey::MapModel::load(world, bytes, &root)?;
    let client = connect_character(clients, 0x6e, 0).await?;
    let mut client = provisioning::provision(client, clients, 0).await?;
    let pz = if world
        .base()
        .tile(TOWN.x, TOWN.y, TOWN.floor)
        .is_some_and(|tile| tile.protection_zone())
    {
        client_spell_journey::protection_zone_probe(&mut client, true).await
    } else {
        Err("configured town start is not a bundle PZ".into())
    };
    let outside: TestResult = async {
        let path = route(world, TOWN, |at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
        })
        .ok_or("no reachable outside-PZ comparison tile")?;
        walk(&mut client, world, &path).await?;
        client_spell_journey::protection_zone_probe(&mut client, false).await
    }
    .await;
    let pz = pz.and(outside);
    let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("PZ comparison has no return route")?;
    walk(&mut client, world, &back).await?;
    let map = map_journey::run(&mut client, world, &model).await;
    let combat = match travel_to_spawn(&mut client, world, spawns, "oteryn:creature.rat").await {
        Ok(()) => client_combat_journey::combat_journey(&mut client).await,
        Err(error) => Err(error),
    };
    let spells = client_spell_journey::spell_journey(&mut client, pz).await;
    // Combat may still hold the logout/PZ lock. Settle it outside the temple
    // before restoring the peer that main's trade/party phases use.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let logout = client.session_mut().logout().await?;
        if logout.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("profile combat lock did not settle before logout: {logout:?}").into(),
            );
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    drop(client);
    client = connect_character(clients, 0x6f, 0).await?;
    let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("profile journey has no return route")?;
    walk(&mut client, world, &back).await?;
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("profile journey logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    map?;
    combat?;
    spells
}

const C16C_CHARACTER: usize = 4;
const W10_CHARACTER: usize = 3;

async fn connect_world(
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    connect_character(clients, tag, 1).await
}

fn fresh_grant_material(tag: u8) -> TestResult<([u8; 32], [u8; 16])> {
    let mut nonce = crate::node::operator_files::random_bytes::<32>()?;
    nonce[0] = tag;
    let mut attempt = crate::node::operator_files::uuid_v7()?;
    attempt[7] = tag;
    Ok((nonce, attempt))
}

pub(super) async fn connect_character(
    clients: &SeamClients<'_>,
    tag: u8,
    index: usize,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    wait_character_released(clients, index).await?;
    let generation = platform_generation(clients.descriptor, &clients.accounts[index]).await?;
    let (nonce, attempt) = fresh_grant_material(tag)?;
    let grant = Grant {
        signing: clients.signing,
        key_id: clients.key_id,
        account_id: &clients.accounts[index],
        character_id: clients.characters[index],
        world_id: clients.world,
        channel_id: clients.channel,
        security_generation: generation,
        scope_generation: clients.scope_generation,
        nonce,
        attempt,
    };
    let token = sign_grant(&grant, now_seconds()?);
    let client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address: clients.address,
        server_name: "localhost",
        root_certificate: clients.certificate,
        schema_revision: 1,
        character_id: crate::foundation::CharacterId::decode(&clients.characters[index])?,
        admission_material: token.as_bytes(),
        client_build_id: "oteryn-dev-client/world-qualification",
        deadline: Duration::from_secs(20),
    })
    .await?;
    verdict::Harness::global().progress("admission");
    Ok(client)
}

async fn travel_to_spawn(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    creature: &str,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|entry| entry.key == creature)
        })
        .collect();
    let path = route(world, from, |at| {
        !world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_none_or(|tile| tile.protection_zone())
            && points.iter().any(|(floor, point)| {
                at.floor == *floor && at.x.abs_diff(point.x) <= 1 && at.y.abs_diff(point.y) <= 1
            })
    })
    .ok_or_else(|| format!("no reachable {creature} spawn within town route budget"))?;
    walk(client, world, &path).await?;
    for _ in 0..32 {
        let here = client.world_spatial().actor_position;
        let target = client
            .world_entities()
            .and_then(|entities| {
                entities
                    .iter()
                    .filter(|entry| {
                        entry.kind == oteryn_dev_client::EntityKind::Creature
                            && entry.position.floor == here.floor
                    })
                    .min_by_key(|entry| {
                        entry
                            .position
                            .x
                            .abs_diff(here.x)
                            .max(entry.position.y.abs_diff(here.y))
                    })
            })
            .copied()
            .ok_or("spawn route has no visible creature")?;
        let target = tile(target.position)?;
        let path = route(world, tile(here)?, |at| {
            at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 1
        })
        .ok_or("visible creature unreachable")?;
        let Some(direction) = path.first() else {
            return Ok(());
        };
        walk(client, world, &[*direction]).await?;
    }
    Err("could not approach a live creature within the route budget".into())
}

pub(crate) fn smoke() -> TestResult {
    use oteryn_world_bundle_compiler::{bundle::BuildClass, compile, source};
    let started = std::time::Instant::now();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(126, 125), (126, 126), (127, 127)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let input = compile::Input {
        regions: &project.regions,
        palette: &palette,
        identity,
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: oteryn_world_bundle_compiler::project::draft_areas(&project.index)?,
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    compile::equivalence(&input, &registry, &compiled.bytes)?;
    let compiled_ms = started.elapsed().as_millis();
    let boot_started = std::time::Instant::now();
    let mut pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
    pin["digest"] = compiled
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .into();
    let world = load_world(
        compiled.bytes,
        &pin,
        WorldId::decode(&v7(1, 2))?,
        ChannelId::decode(&v7(1, 3))?,
    )?;
    assert!(
        world
            .base()
            .tile(TOWN.x, TOWN.y, TOWN.floor)
            .is_some_and(|tile| tile.protection_zone()),
        "town temple must be a protection zone"
    );
    assert!(
        route(world, TOWN, |at| near_locker(world, at)).is_some(),
        "town depot must be reachable"
    );
    let floor_path = route(world, TOWN, |at| at.floor != TOWN.floor)
        .ok_or("town floor-change route must be reachable")?;
    let mut landing = TOWN;
    for direction in floor_path {
        landing = destination(world, landing, direction).ok_or("floor route landing")?;
    }
    assert_ne!(landing.floor, TOWN.floor);
    assert!(
        route(world, landing, |at| at == TOWN).is_some(),
        "town floor-change route must have a return path"
    );
    let rats: Vec<_> = project
        .families
        .spawns
        .iter()
        .flat_map(|source| &source.points)
        .filter(|point| point.creature == "oteryn:creature.rat")
        .map(|point| point.cell)
        .collect();
    assert!(
        route(world, TOWN, |at| rats.iter().any(|&(x, y, z)| i16::from(
            at.floor
        )
            == -i16::from(z)
            && at.x.abs_diff(x).max(at.y.abs_diff(y)) <= 1))
        .is_some(),
        "town hunt must reach an authored rat with an admitted corpse and loot table"
    );
    evidence(&format!(
        "world_loader_smoke state=booted compile_ms={compiled_ms} boot_ms={} duration_ms={}",
        boot_started.elapsed().as_millis(),
        started.elapsed().as_millis()
    ));
    Ok(())
}

async fn shop_journey(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    clients: &SeamClients<'_>,
) -> TestResult<()> {
    let basis = spatial_basis::current(client)?;
    let Some(npc) = npcs.npcs.iter().find(|npc| npc.key == "oteryn:npc.sam") else {
        evidence(
            "mechanic=npc_shop skipped=authored_shop_npc_missing content_id=oteryn:npc.sam source=content/npcs/definitions/index.json:1",
        );
        return Ok(());
    };
    let from = tile(client.world_spatial().actor_position)?;
    let Some(path) = route(world, from, |at| {
        npc.placements.iter().any(|placement| {
            placement.floor == at.floor
                && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
        })
    }) else {
        evidence(
            "mechanic=npc_shop skipped=no_route_to_authored_shop_within_town_budget content_id=oteryn:npc.sam source=apps/game-server/src/qualification/client_phases/world.rs:126",
        );
        return Ok(());
    };
    walk(client, world, &path).await?;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|placement| {
                        entity.position.x == i32::from(placement.x)
                            && entity.position.y == i32::from(placement.y)
                            && entity.position.floor == basis.floor(placement.floor)
                    })
            })
        })
        .copied();
    let Some(target) = target else {
        evidence(
            "mechanic=npc_shop skipped=authored_shop_has_no_visible_actor_at_placement content_id=oteryn:npc.sam source=crates/protocol-oteryn/src/world_spatial_entities.rs:105",
        );
        return Ok(());
    };
    client_npc_shop::transact(client, target.entity, clients).await
}

#[cfg(test)]
#[test]
fn qualification_nbtown_checks_first_entry_before_mutating_packets() -> TestResult {
    let source = include_str!("world.rs");
    let (_, journey) = source
        .split_once("pub(super) async fn journeys(")
        .ok_or("missing world journey")?;
    let (journey, _) = journey
        .split_once("async fn profile_journey(")
        .ok_or("missing profile journey boundary")?;
    let entry_check = journey
        .find("if entered_at != TOWN {")
        .ok_or("missing strict Platform-created town assertion")?;
    for packet in [
        "include!(\"full_e2ef6_mount.rs\")",
        "full_e2ef5::run(clients",
        "include!(\"full_e2ef4_mount.rs\")",
        "profile_journey(clients",
    ] {
        let packet_start = journey.find(packet).ok_or("missing mutating packet")?;
        assert!(
            entry_check < packet_start,
            "town entry checked after {packet}"
        );
    }
    let mount = include_str!("full_e2ef6_mount.rs");
    let starter = mount
        .find("full_e2ef6::run_starter(clients)")
        .ok_or("missing pristine starter row")?;
    let packet = mount
        .find("full_e2ef7::run_packet(clients,")
        .ok_or("missing F7 packet")?;
    let remaining = mount
        .find("full_e2ef6::run(clients")
        .ok_or("missing remaining F6 rows")?;
    assert!(starter < packet && packet < remaining);
    Ok(())
}

#[test]
fn qualification_fresh_grants_do_not_repeat_in_new_processes() -> TestResult {
    const CHILD: &str = "OTERYN_TEST_FRESH_GRANT_CHILD";
    if std::env::var_os(CHILD).is_some() {
        println!("GRANT_MATERIAL={:?}", fresh_grant_material(0xe0)?);
        return Ok(());
    }
    let material = || -> TestResult<String> {
        let output = std::process::Command::new(std::env::current_exe()?)
            .args([
                "qualification_fresh_grants_do_not_repeat_in_new_processes",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()?;
        assert!(output.status.success());
        String::from_utf8(output.stdout)?
            .lines()
            .find_map(|line| {
                line.split_once("GRANT_MATERIAL=")
                    .map(|(_, material)| material)
            })
            .map(str::to_owned)
            .ok_or_else(|| "child did not issue a grant".into())
    };
    let first = material()?;
    let second = material()?;
    let first = first.split_once("], [").ok_or("malformed first grant")?;
    let second = second.split_once("], [").ok_or("malformed second grant")?;
    assert_ne!(first.0, second.0, "grant nonce reused across processes");
    assert_ne!(first.1, second.1, "attempt ID reused across processes");
    Ok(())
}

#[cfg(test)]
#[test]
fn qualification_nbfix_fresh_grants_for_reconnects_and_colliding_tags() -> TestResult {
    let mut nonces = std::collections::BTreeSet::new();
    let mut attempts = std::collections::BTreeSet::new();
    for tag in [0xe0, 0xa1, 0xa0, 0xe0, 0xa1, 0xa0] {
        let (nonce, attempt) = fresh_grant_material(tag)?;
        assert!(nonces.insert(nonce), "reused grant nonce for tag {tag}");
        assert!(attempts.insert(attempt), "reused attempt for tag {tag}");
        assert_eq!(nonce[0], tag);
        assert_eq!(attempt[7], tag);
        assert_eq!(attempt[6] >> 4, 7);
        assert_eq!(attempt[8] >> 6, 2);
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn qualification_nbroute_retries_only_stationary_rejections_with_a_bound() {
    for attempt in 1..=6 {
        assert_eq!(
            town_route_retry_delay(StepDisposition::Rejected, true, attempt),
            (attempt <= 5).then_some(Duration::from_millis(500)),
        );
        for disposition in [StepDisposition::Moved, StepDisposition::Blocked] {
            assert_eq!(town_route_retry_delay(disposition, true, attempt), None);
        }
        for disposition in [
            StepDisposition::Moved,
            StepDisposition::Blocked,
            StepDisposition::Rejected,
        ] {
            assert_eq!(town_route_retry_delay(disposition, false, attempt), None);
        }
    }
}

#[cfg(test)]
#[test]
fn qualification_town_route_detours_around_live_wolf() -> TestResult {
    use StepDirection::*;
    let from = TilePos {
        x: 32427,
        y: 32158,
        floor: -7,
    };
    let wolf = TilePos { x: 32428, ..from };
    let goal = TilePos { x: 32429, ..from };
    let landing = |pos: TilePos, direction: StepDirection| {
        let (dx, dy) = direction.delta();
        let x = pos.x.checked_add_signed(dx as i16)?;
        let y = pos.y.checked_add_signed(dy as i16)?;
        ((from.x..=goal.x).contains(&x) && (32157..=32158).contains(&y)).then_some(TilePos {
            x,
            y,
            ..pos
        })
    };
    assert_eq!(
        route_with_landings(from, |at| at == goal, |_| true, landing),
        Some(vec![East, East])
    );
    let detour =
        route_around_occupant(from, from, wolf, &VecDeque::from([East]), |_| true, landing)
            .ok_or("missing wolf detour")?;
    assert_eq!(detour, [North, East, East, South]);
    assert!(route_around_occupant(from, from, wolf, &VecDeque::new(), |_| true, landing).is_none());
    Ok(())
}
