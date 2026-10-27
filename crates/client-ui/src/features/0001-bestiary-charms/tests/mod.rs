use super::*;
use std::num::NonZeroU32;

pub(super) fn index(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("non-zero index")
}

pub(super) fn race(race: u32, kill_count: u32) -> BestiaryRaceProgress {
    BestiaryRaceProgress {
        race: index(race),
        kill_count,
        kill_thresholds: [5, 10, 25],
    }
}

pub(super) fn charm(charm: u32, kind: CharmKind, stage: u8, assigned: Option<u32>) -> CharmState {
    CharmState {
        charm: index(charm),
        kind,
        unlocked_stage: stage,
        assigned_race: assigned.map(index),
        next_stage_cost: if stage == 3 { 0 } else { 100 },
        effect_active: charm != 2,
    }
}

/// Race 1 at stage 1, race 2 at stage 2, race 3 complete. Charm 1 major locked, 2 minor
/// unlocked, 3 major unlocked and assigned, 4 minor complete. 150 points, 50 echoes, 2 slots.
pub(super) fn cyclopedia() -> Cyclopedia {
    let mut view = Cyclopedia::new();
    view.apply_bestiary_snapshot(vec![race(3, 25), race(1, 5), race(2, 10)]);
    view.apply_charm_view(CharmView {
        charms: vec![
            charm(1, CharmKind::Major, 0, None),
            charm(2, CharmKind::Minor, 1, None),
            charm(3, CharmKind::Major, 1, Some(3)),
            charm(4, CharmKind::Minor, 3, None),
        ],
        charm_points_available: 150,
        minor_charm_echoes_available: 50,
        assignment_slot_limit: Some(index(2)),
    });
    view
}

mod protocol;
mod view;
