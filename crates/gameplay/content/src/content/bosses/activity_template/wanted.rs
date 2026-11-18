//! Crystal 201189f8 actions_portal_minis_kilmaresh.lua:2–54,59–106.
use super::BossLeverCandidate;

pub const PATH: &str = "data-global/scripts/quests/kilmaresh/actions_portal_minis_kilmaresh.lua";
const DIGEST: &str = "ad41c21223a1ca498eba8727e1e9731302debcfc48c43a5b53714f6cf4164f46";

struct Room {
    line: u32,
    name: &'static str,
    creature: &'static str,
    boss: &'static str,
    portal: [i32; 3],
    entry: [i32; 3],
    spawn: [i32; 3],
    low: [i32; 3],
    high: [i32; 3],
    exit: [i32; 3],
    exit_portal: [i32; 3],
}

const ROOMS: [Room; 3] = [
    Room {
        line: 2,
        name: "Neferi the Spy",
        creature: "oteryn:creature.neferi_the_spy",
        boss: "oteryn:boss.neferi_the_spy",
        portal: [33886, 31477, 6],
        entry: [33871, 31547, 8],
        spawn: [33871, 31552, 8],
        low: [33866, 31545, 8],
        high: [33876, 31555, 8],
        exit: [33886, 31478, 6],
        exit_portal: [33871, 31546, 8],
    },
    Room {
        line: 16,
        name: "Sister Hetai",
        creature: "oteryn:creature.sister_hetai",
        boss: "oteryn:boss.sister_hetai",
        portal: [33883, 31467, 9],
        entry: [33833, 31490, 9],
        spawn: [33833, 31496, 9],
        low: [33827, 31488, 9],
        high: [33837, 31501, 9],
        exit: [33883, 31468, 9],
        exit_portal: [33833, 31489, 9],
    },
    Room {
        line: 30,
        name: "Amenef the Burning",
        creature: "oteryn:creature.amenef_the_burning",
        boss: "oteryn:boss.amenef_the_burning",
        portal: [33819, 31773, 10],
        entry: [33849, 31782, 10],
        spawn: [33849, 31787, 10],
        low: [33842, 31779, 10],
        high: [33855, 31791, 10],
        exit: [33819, 31774, 10],
        exit_portal: [33849, 31781, 10],
    },
];

pub fn boss_at(position: [i32; 3]) -> Option<&'static str> {
    ROOMS
        .iter()
        .find(|room| room.portal == position)
        .map(|room| room.boss)
}

pub fn is_portal_at(position: [i32; 3]) -> bool {
    ROOMS
        .iter()
        .any(|room| room.portal == position || room.exit_portal == position)
}

pub fn boss_keys() -> [&'static str; 3] {
    ROOMS.each_ref().map(|room| room.boss)
}

pub fn is_wanted_boss(boss: &str) -> bool {
    ROOMS.iter().any(|room| room.boss == boss)
}

pub(super) fn candidates() -> impl Iterator<Item = BossLeverCandidate> {
    ROOMS.iter().map(|room| BossLeverCandidate {
        path: PATH.into(),
        sha256: DIGEST.into(),
        line: room.line,
        boss_name: Some(room.name.into()),
        creature_key: Some(room.creature.into()),
        boss_position: Some(room.spawn),
        lever_positions: vec![room.portal],
        entry_positions: vec![room.entry],
        zone_from: Some(room.low),
        zone_to: Some(room.high),
        exit_position: Some(room.exit),
        source_exit_teleporter: Some(room.exit_portal),
        source_callback_closure: None,
        blockers: Vec::new(),
        required_callbacks: Vec::new(),
        callbacks_known: true,
        native_profile_sha256: None,
        complete_literal_layout: true,
        minimum_players: Some(1),
        minimum_level: Some(250),
        cooldown_seconds: Some(36_000),
        time_limit_seconds: Some(600),
        after_kill_seconds: Some(0),
    })
}
