//! Real-world journeys use the same pinned bundle and native content as `serve`.
use super::*;
use crate::map::boot::BundleWorld;
use crate::map::overlay::TilePos;
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

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
include!("full_row_g39_mount.rs");
include!("full_row_g40_mount.rs");
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
#[path = "full_e2ef1.rs"]
mod provisioning;
include!("quest_journal.rs");

pub(crate) const TOWN: TilePos = TilePos {
    x: 32369,
    y: 32238,
    floor: -7,
};

/// One production world per test process: every journey shares it, so the
/// ~110 s bundle boot runs once instead of once per row.
pub(crate) fn load_world(
    data: Vec<u8>,
    pin: &serde_json::Value,
    world: WorldId,
    channel: ChannelId,
) -> TestResult<&'static BundleWorld> {
    static LOADED: std::sync::OnceLock<(WorldId, ChannelId, BundleWorld)> =
        std::sync::OnceLock::new();
    if LOADED.get().is_none() {
        let booted = boot_world(data, pin, world, channel)?;
        let _ = LOADED.set((world, channel, booted));
    }
    match LOADED.get() {
        Some((loaded_world, loaded_channel, booted))
            if *loaded_world == world && *loaded_channel == channel =>
        {
            Ok(booted)
        }
        Some(_) => Err("world loader asked for a different world or channel".into()),
        None => Err("world loader cache empty".into()),
    }
}

fn boot_world(
    data: Vec<u8>,
    pin: &serde_json::Value,
    world: WorldId,
    channel: ChannelId,
) -> TestResult<BundleWorld> {
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
    crate::node::serve::boot_bundle_world(
        checked,
        world,
        channel,
        controller
            .active()
            .and_then(|active| active.native_gameplay()),
    )
    .map_err(|error| format!("production world loader: {error:?}").into())
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
            return Some(path);
        }
        for direction in [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ] {
            let Some(next) = destination(world, at, direction) else {
                continue;
            };
            if next.x.abs_diff(from.x) > 160
                || next.y.abs_diff(from.y) > 160
                || next.floor.abs_diff(from.floor) > 2
                || parents.contains_key(&next)
            {
                continue;
            }
            parents.insert(next, Some((at, direction)));
            queue.push_back(next);
        }
    }
    None
}

/// Use the production floor resolver over the production bundle's collision facts.
fn destination(world: &BundleWorld, from: TilePos, direction: StepDirection) -> Option<TilePos> {
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
        |_| true,
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

async fn walk(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = client.world_spatial().actor_position;
        let outcome = client.step_retrying(direction).await?;
        if outcome.disposition != StepDisposition::Moved || outcome.world_spatial_delta.is_none() {
            return Err(format!("town route step {direction:?}: {outcome:?}").into());
        }
        let after = client.world_spatial().actor_position;
        if Some(tile(after)?) != destination(world, tile(before)?, direction) {
            return Err(format!(
                "town step changed position unexpectedly: {before:?} -> {after:?}"
            )
            .into());
        }
        client.take_events();
    }
    Ok(())
}

pub(super) async fn journeys(clients: &SeamClients<'_>, path: &Path) -> TestResult {
    let started = std::time::Instant::now();
    let bytes = std::fs::read(path)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    evidence(&format!(
        "world_loader state=booted duration_ms={} start={TOWN:?}",
        started.elapsed().as_millis()
    ));
    // Keep the high-level fixture on the other Character: the original
    // progression/death journey must retain its ordinary starter build.
    if e2e_row_selected(0) {
        full_e2ef7::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(1) {
        full_row_c03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(2) {
        full_row_c12::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(3) {
        full_row_c09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(4) {
        full_row_c06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(5) {
        full_row_c02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(6) {
        full_row_c01::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(7) {
        full_row_w22b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(8) {
        full_row_c05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(9) {
        full_row_c04::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(10) {
        full_row_w18::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(11) {
        full_row_c11::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(12) {
        full_row_w19a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(13) {
        full_row_w01b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(14) {
        full_row_w23::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(15) {
        full_row_w15d::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(16) {
        full_row_w15a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(17) {
        full_row_w10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(18) {
        full_row_w08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(19) {
        full_row_w24::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(20) {
        full_row_w22a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(21) {
        full_row_w15b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(22) {
        full_row_w11::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(23) {
        full_row_w05a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(24) {
        full_row_w03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(25) {
        full_row_w01a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(26) {
        full_row_s16b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(27) {
        full_row_s15b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(28) {
        full_row_s10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(29) {
        full_row_s07::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(30) {
        full_row_s06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(31) {
        full_row_s05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(32) {
        full_row_p07::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(33) {
        full_row_w17::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(34) {
        full_row_w16::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(35) {
        full_row_w13::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(36) {
        full_row_w12::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(37) {
        full_row_w05c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(38) {
        full_row_w04::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(39) {
        full_row_w02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(40) {
        full_row_w01c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(41) {
        full_row_s13c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(42) {
        full_row_s04c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(43) {
        full_row_s04a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(44) {
        full_row_s02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(45) {
        full_row_p23c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(46) {
        full_row_p20::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(47) {
        full_row_p18::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(48) {
        full_row_p17::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(49) {
        full_row_p15::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(50) {
        full_row_p14::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(51) {
        full_row_p13::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(52) {
        full_row_p12c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(53) {
        full_row_p12a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(54) {
        full_row_p11::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(55) {
        full_row_p09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(56) {
        full_row_p08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(57) {
        full_row_p06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(58) {
        full_row_p05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(59) {
        full_row_p04::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(60) {
        full_row_p03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(61) {
        full_row_p02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(62) {
        full_row_p01::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(63) {
        full_row_e10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(64) {
        full_row_e09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(65) {
        full_row_e08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(66) {
        full_row_e06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(67) {
        full_row_i11::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(68) {
        full_row_i10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(69) {
        full_row_i05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(70) {
        full_row_m11b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(71) {
        full_row_m11a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(72) {
        full_row_m08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(73) {
        full_row_c18b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(74) {
        full_row_c17b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(75) {
        full_row_c10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(76) {
        full_row_c07::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(77) {
        full_row_e12::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(78) {
        full_row_i14::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(79) {
        full_row_i03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(80) {
        full_row_m10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(81) {
        full_row_m09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(82) {
        full_row_m06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(83) {
        full_row_c15::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(84) {
        full_row_c08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(85) {
        full_e2ef19::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(86) {
        full_e2ef18::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(87) {
        full_e2ef20::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(88) {
        client_row_c01::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(89) {
        client_row_c02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(90) {
        client_row_c03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(91) {
        client_row_c04::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(92) {
        client_row_c05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(93) {
        client_row_c06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(227) {
        client_row_c07::run_client_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(94) {
        client_row_c09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(95) {
        client_row_c10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(96) {
        client_row_c11::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(97) {
        client_row_c12::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(98) {
        client_row_c13::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(99) {
        client_row_c14::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(100) {
        client_row_c15::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(101) {
        client_row_c16a::run_packet(clients, world).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(102) {
        client_row_c16b::run_packet(clients, world).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(231) {
        full_e2ef5::run(clients, world).await;
    }
    if e2e_row_selected(103) {
        client_row_c16c::run_packet(clients, world).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(104) {
        client_row_c16d::run_packet(clients, world).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(105) {
        client_row_c17a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(106) {
        client_row_c17b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(107) {
        client_row_c17d::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(108) {
        client_row_c18a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(109) {
        client_row_c18b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(228) {
        client_row_c20a::run_packet(clients, world, &metadata.spawns).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(230) {
        include!("full_e2ef6_mount.rs");
        release_row_characters(clients).await;
    }
    if e2e_row_selected(110) {
        client_row_c20b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(111) {
        client_row_c20c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(229) {
        client_row_c20d::run_packet(clients, world, &metadata.spawns).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(112) {
        client_row_e01::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(113) {
        client_row_i01::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(114) {
        client_row_i03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(115) {
        client_row_i04::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(116) {
        client_row_i05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(117) {
        client_row_i06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(118) {
        client_row_i07::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(119) {
        client_row_i08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(120) {
        client_row_i09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(121) {
        client_row_i10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(122) {
        client_row_i11::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(123) {
        client_row_i12::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(124) {
        client_row_i13::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(125) {
        client_row_i14::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(126) {
        client_row_i15::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(127) {
        client_row_i16::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(128) {
        client_row_m01::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(129) {
        client_row_m02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(130) {
        client_row_m03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(131) {
        client_row_m04::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(132) {
        client_row_m05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(133) {
        client_row_m06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(134) {
        client_row_m07::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(135) {
        client_row_m08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(136) {
        client_row_m09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(137) {
        client_row_m10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(138) {
        client_row_m11a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(139) {
        client_row_m11b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(140) {
        client_row_n01::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(141) {
        client_row_n02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(142) {
        client_row_n03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(143) {
        client_row_n04::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(144) {
        client_row_c08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(145) {
        client_row_c17c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(146) {
        client_row_c19a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(147) {
        client_row_e02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(148) {
        client_row_e03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(149) {
        client_row_e05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(150) {
        client_row_e06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(151) {
        client_row_e07::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(152) {
        client_row_e08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(153) {
        client_row_e09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(154) {
        client_row_e10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(155) {
        client_row_e12::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(156) {
        client_row_p01::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(157) {
        client_row_p02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(158) {
        client_row_p03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(159) {
        client_row_p04::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(160) {
        client_row_p05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(161) {
        client_row_p06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(162) {
        client_row_p07::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(163) {
        client_row_p08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(164) {
        client_row_p09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(165) {
        client_row_p10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(166) {
        client_row_p11::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(167) {
        client_row_p12a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(168) {
        client_row_p12c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(169) {
        client_row_p13::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(170) {
        client_row_p14::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(171) {
        client_row_p15::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(172) {
        client_row_p16::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(173) {
        client_row_p17::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(174) {
        client_row_p18::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(175) {
        client_row_p19::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(176) {
        client_row_p20::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(177) {
        client_row_p21::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(178) {
        client_row_p23c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(179) {
        client_row_s01::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(180) {
        client_row_s02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(181) {
        client_row_s03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(182) {
        client_row_s04a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(183) {
        client_row_s04b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(184) {
        client_row_s18::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(185) {
        client_row_w01a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(186) {
        client_row_w01b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(187) {
        client_row_w01c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(188) {
        client_row_w02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(189) {
        client_row_w03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(190) {
        client_row_w04::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(191) {
        client_row_w05a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(192) {
        client_row_w05c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(193) {
        client_row_w10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(194) {
        client_row_w11::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(195) {
        client_row_w12::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(196) {
        client_row_w13::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(197) {
        client_row_s06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(198) {
        client_row_s08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(199) {
        client_row_s13c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(200) {
        client_row_s13b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(201) {
        client_row_s13a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(202) {
        client_row_s15b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(203) {
        client_row_s04c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(204) {
        client_row_s07::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(205) {
        client_row_s16b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(206) {
        client_row_s09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(207) {
        client_row_s05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(208) {
        client_row_s10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(209) {
        client_row_s16a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(210) {
        client_row_s11::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(211) {
        client_row_w15a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(212) {
        client_row_w15b::run_packet(clients, world).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(213) {
        client_row_w15d::run_packet(clients, world).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(214) {
        client_row_w16::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(215) {
        client_row_w17::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(216) {
        client_row_w19a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(217) {
        client_row_w20a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(218) {
        client_row_w20b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(219) {
        client_row_w22a::run_packet(clients, world).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(220) {
        client_row_w22b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(221) {
        client_row_w23::run_packet(clients, world).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(222) {
        client_row_w24::run_packet(clients, world).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(223) {
        client_row_n07a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(224) {
        client_row_n07b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(225) {
        client_row_n07c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(226) {
        client_row_n07d::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    for index in 0..clients.accounts.len() {
        wait_character_released(clients, index).await?;
    }
    if e2e_row_selected(232) {
        include!("full_e2ef4_mount.rs");
        release_row_characters(clients).await;
    }
    if e2e_row_selected(233) {
        full_row_g01::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(234) {
        full_row_g02::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(235) {
        full_row_g03::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(236) {
        full_row_g04::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(237) {
        full_row_g05::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(238) {
        full_row_g06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(239) {
        full_row_g07::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(240) {
        full_row_g08::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(241) {
        full_row_g09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(242) {
        full_row_g10::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(243) {
        full_row_g11::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(244) {
        full_row_g12::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(245) {
        full_row_g13::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(246) {
        full_row_g14::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(247) {
        full_row_g15::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(248) {
        full_row_g16::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(249) {
        full_row_g17::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(250) {
        full_row_g18::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(251) {
        full_row_g19::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(252) {
        full_row_g20::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(253) {
        full_row_g21::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(254) {
        full_row_g22::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(255) {
        full_row_g23::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(256) {
        full_row_g24::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(257) {
        full_row_g25::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(258) {
        full_row_g26::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(259) {
        full_row_g27::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(260) {
        full_row_g28::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(261) {
        full_row_g29::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(262) {
        full_row_g30::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(263) {
        full_row_g31::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(264) {
        full_row_g32::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(265) {
        full_row_g33::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(266) {
        full_row_g34::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(267) {
        full_row_g35::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(268) {
        full_row_g36::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(269) {
        full_row_c19b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(270) {
        full_row_i17a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(271) {
        full_row_i17b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(272) {
        full_row_i17c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(273) {
        full_row_i17d::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(274) {
        full_row_i18a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(275) {
        full_row_i18b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(276) {
        full_row_i18c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(277) {
        full_row_i19::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(278) {
        full_row_p12b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(279) {
        full_row_p22a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(280) {
        full_row_p22b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(281) {
        full_row_p22c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(282) {
        full_row_p22d::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(283) {
        full_row_s14a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(284) {
        full_row_s14b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(285) {
        full_row_s14c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(286) {
        full_row_s15a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(287) {
        full_row_s16c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(288) {
        full_row_s16d::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(289) {
        full_row_s17a::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(290) {
        full_row_s17b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(291) {
        full_row_s17c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(292) {
        full_row_s19::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(293) {
        full_row_w05b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(294) {
        full_row_w05d::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(295) {
        full_row_w06::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(296) {
        full_row_w07::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(297) {
        full_row_w09::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(298) {
        full_row_w15c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(299) {
        full_row_w19b::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(300) {
        full_row_w19c::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(301) {
        full_row_w19d::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(302) {
        full_row_w19e::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(303) {
        full_row_w21::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(304) {
        full_row_g37::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(305) {
        full_row_g38::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(306) {
        full_row_g39::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    if e2e_row_selected(307) {
        full_row_g40::run_packet(clients).await;
        release_row_characters(clients).await;
    }
    // The unindexed baseline (including character_core and full_e2ef3) belongs
    // to the first batch; its assertions and verdicts run once per full sweep.
    if !e2e_row_selected(0) {
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
    let entered_at = tile(client.world_spatial().actor_position)?;
    if entered_at != TOWN {
        return Err(format!(
            "Platform-created character did not enter the configured town: expected={TOWN:?} actual={entered_at:?}"
        )
        .into());
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
        if client.session_mut().logout().await?.result
            == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("death relog logout was not accepted".into());
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
        if client.session_mut().logout().await?.result
            == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("profile combat lock did not settle before logout".into());
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
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("profile journey logout refused".into());
    }
    map?;
    combat?;
    spells
}

async fn connect_world(
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    connect_character(clients, tag, 1).await
}

fn fresh_grant_material(tag: u8) -> TestResult<([u8; 32], [u8; 16])> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_CONNECTION: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT_CONNECTION
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| "qualification connection counter exhausted")?;
    let mut nonce = [tag; 32];
    nonce[24..].copy_from_slice(&sequence.to_be_bytes());
    let mut attempt = v7(tag, 0x05);
    attempt[8..].copy_from_slice(&sequence.to_be_bytes());
    // Preserve all 64 counter bits around the UUIDv7 version and RFC variant.
    attempt[6] |= (sequence >> 62) as u8;
    attempt[8] = (attempt[8] & 0x3f) | 0x80;
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
    Ok(
        oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
            address: clients.address,
            server_name: "localhost",
            root_certificate: clients.certificate,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&clients.characters[index])?,
            admission_material: token.as_bytes(),
            client_build_id: "oteryn-dev-client/world-qualification",
            deadline: Duration::from_secs(20),
        })
        .await?,
    )
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
                            && entity.position.floor == -i16::from(placement.floor)
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

// Every row uses the decoded logout result: InFight and Busy keep the session alive.
async fn row_logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let reply =
        tokio::time::timeout(Duration::from_secs(10), client.session_mut().logout()).await??;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("row logout refused: {:?}", reply.result).into());
    }
    Ok(())
}

// Keep cancelled/refused journeys' connections alive until the verdict has awaited cleanup.
tokio::task_local! {
    static ROW_PENDING_LOGOUT: std::cell::RefCell<Vec<oteryn_dev_client::DevClientSession>>;
}

struct RowClient {
    // Exactly one while active; consuming recovery and Drop remove it.
    inner: Vec<oteryn_dev_client::DevClientSession>,
    ended: bool,
}
impl std::ops::Deref for RowClient {
    type Target = oteryn_dev_client::DevClientSession;
    fn deref(&self) -> &Self::Target {
        &self.inner[0]
    }
}
impl std::ops::DerefMut for RowClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner[0]
    }
}
#[derive(Debug)]
struct RowLogout {
    status: CommandStatus,
    result: Option<oteryn_protocol_oteryn::logout::LogoutResult>,
}
impl RowClient {
    fn new(client: oteryn_dev_client::DevClientSession) -> Self {
        Self {
            inner: vec![client],
            ended: false,
        }
    }
    fn into_inner(mut self) -> TestResult<oteryn_dev_client::DevClientSession> {
        self.inner
            .pop()
            .ok_or_else(|| "row session already consumed".into())
    }
    async fn row_logout(&mut self) -> TestResult<RowLogout> {
        let reply = self.session_mut().logout().await?;
        self.ended = reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted);
        Ok(RowLogout {
            status: reply.status,
            result: reply.result,
        })
    }
}
impl Drop for RowClient {
    fn drop(&mut self) {
        if !self.ended
            && let Some(client) = self.inner.pop()
        {
            let _ = ROW_PENDING_LOGOUT.try_with(|pending| pending.borrow_mut().push(client));
        }
    }
}
async fn row_connect_world(clients: &SeamClients<'_>, tag: u8) -> TestResult<RowClient> {
    ROW_PENDING_LOGOUT
        .try_with(|_| ())
        .map_err(|_| "row admission requires its cleanup scope")?;
    Ok(RowClient::new(connect_world(clients, tag).await?))
}
async fn row_connect_character(
    clients: &SeamClients<'_>,
    tag: u8,
    index: usize,
) -> TestResult<RowClient> {
    ROW_PENDING_LOGOUT
        .try_with(|_| ())
        .map_err(|_| "row admission requires its cleanup scope")?;
    Ok(RowClient::new(
        connect_character(clients, tag, index).await?,
    ))
}
async fn row_finish(client: &mut RowClient) -> TestResult {
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("row logout refused".into());
    }
    Ok(())
}
async fn row_complete_with_timeout<F: std::future::Future>(
    duration: Duration,
    future: F,
) -> TestResult<F::Output> {
    Ok(row_complete(tokio::time::timeout(duration, future)).await??)
}
// Capture journeys already bound their operation and retain their client outside it.
// Full journeys use row_complete_with_timeout with their declared whole-row budget.
async fn row_complete<F: std::future::Future>(future: F) -> TestResult<F::Output> {
    ROW_PENDING_LOGOUT
        .scope(std::cell::RefCell::new(Vec::new()), async {
            let result = future.await;
            let pending =
                ROW_PENDING_LOGOUT.with(|pending| std::mem::take(&mut *pending.borrow_mut()));
            let mut cleanup = Ok(());
            for mut client in pending {
                if let Err(error) = row_logout(&mut client).await {
                    cleanup = Err(error);
                }
            }
            cleanup?;
            Ok(result)
        })
        .await
}

#[cfg(test)]
fn row_helper_world() -> TestResult<BundleWorld> {
    use crate::map::boot::{
        boot,
        tests::{floor_bundle, floor_items, floor_pins},
    };
    let (bytes, pins) = floor_bundle();
    Ok(boot(
        &bytes,
        &floor_pins(pins),
        WorldId::decode(&v7(1, 2))?,
        ChannelId::decode(&v7(1, 3))?,
        floor_items,
    )?)
}

// Handles name the current admission/custody only. Compare durable item values and slots.
fn row_inventory_signature(
    inventory: &oteryn_dev_client::CharacterInventory,
) -> Vec<(u32, u32, u32, u32)> {
    let item = |slot, entry: &oteryn_dev_client::ItemEntry| {
        (
            slot,
            entry.item_definition_ref.get(),
            entry.count.get(),
            entry.sub_type,
        )
    };
    let mut values: Vec<_> = inventory
        .entries
        .iter()
        .map(|entry| item(0, entry))
        .chain(inventory.main_backpack.iter().map(|entry| item(1, entry)))
        .chain(
            inventory
                .equipment
                .iter()
                .map(|entry| item(2 + entry.slot as u32, &entry.item)),
        )
        .collect();
    values.sort_unstable();
    values
}

#[cfg(test)]
#[test]
fn full_row_inventory_signature_preserves_slots_and_values_but_not_handles() -> TestResult {
    use oteryn_protocol_oteryn::item_view::*;
    let entry = ItemEntry {
        handle: 1_u64.try_into()?,
        item_definition_ref: 2_u32.try_into()?,
        count: 3_u32.try_into()?,
        sub_type: 4,
    };
    let before = CharacterInventory {
        entries: vec![entry],
        ..Default::default()
    };
    let mut after = before.clone();
    after.entries[0].handle = 5_u64.try_into()?;
    assert_eq!(
        row_inventory_signature(&before),
        row_inventory_signature(&after)
    );
    after.entries[0].sub_type = 0;
    assert_ne!(
        row_inventory_signature(&before),
        row_inventory_signature(&after)
    );
    after.entries.clear();
    after.main_backpack = Some(entry);
    assert_ne!(
        row_inventory_signature(&before),
        row_inventory_signature(&after)
    );
    Ok(())
}

async fn row_poison_active(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<bool> {
    use oteryn_protocol_oteryn::condition_status as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CONDITION_STATUS_V1,
            wire::COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err("condition-status query refused".into());
    }
    row_has_poison(&reply.payload)
}
fn row_has_poison(payload: &[u8]) -> TestResult<bool> {
    let status = oteryn_protocol_oteryn::condition_status::ConditionStatus::decode(payload)
        .ok_or("invalid condition-status reply")?;
    Ok(status
        .conditions
        .iter()
        .any(|condition| condition.icon == 1))
}
async fn row_isolate_poison(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while !row_poison_active(client).await? {
        if tokio::time::Instant::now() >= deadline {
            return Err("needed:server-reported-poison-condition".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
    })
    .ok_or("needed:protection-zone-to-isolate-poison-from-melee")?;
    walk(client, world, &path).await?;
    if !row_poison_active(client).await? {
        return Err("poison expired before isolated tick observation".into());
    }
    client.take_events();
    Ok(())
}
#[cfg(test)]
#[test]
fn full_row_poison_requires_the_poison_catalogue_entry() -> TestResult {
    use oteryn_protocol_oteryn::condition_status::*;
    assert!(row_has_poison(&[]).is_err());
    assert!(!row_has_poison(&[0])?);
    for (icon, expected) in [(1, true), (2, false), (14, false)] {
        let payload = ConditionStatus {
            conditions: vec![ActiveCondition {
                icon,
                sub_id: 0,
                remaining_ms: 1000,
            }],
        }
        .encode()
        .ok_or("condition encode")?;
        assert_eq!(row_has_poison(&payload)?, expected);
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn full_row_timeout_drops_the_journey_inside_its_cleanup_scope() -> TestResult {
    struct Probe(std::rc::Rc<std::cell::Cell<bool>>);
    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.set(ROW_PENDING_LOGOUT.try_with(|_| ()).is_ok());
        }
    }
    let dropped_in_scope = std::rc::Rc::new(std::cell::Cell::new(false));
    let probe = Probe(dropped_in_scope.clone());
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?
        .block_on(async {
            let result = row_complete_with_timeout(Duration::ZERO, async move {
                let _probe = probe;
                std::future::pending::<()>().await;
            })
            .await;
            assert!(result.is_err());
            assert!(dropped_in_scope.get());
        });
    Ok(())
}
