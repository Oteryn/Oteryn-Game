//! Shared synthetic data construction only; no tests or current-authority grant.
#![allow(
    clippy::expect_used,
    reason = "Synthetic pure-state fixture construction"
)]
use super::*;

pub(super) fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}
pub(super) fn channel(seed: u8) -> ChannelId {
    ChannelId::decode(&id(seed)).expect("native channel")
}
pub(super) fn generation(value: u64) -> ScopeOwnershipGeneration {
    ScopeOwnershipGeneration::new(value).expect("native generation")
}
pub(super) fn key() -> RaidKey {
    RaidKey {
        world: WorldId::decode(&id(1)).expect("native world"),
        raid: "oteryn:raid.proposed".into(),
    }
}
pub(super) fn timing(persistent: bool) -> Timing {
    Timing {
        policy: Policy::Random {
            mean_interval_s: 60,
            min_gap_s: 0,
        },
        last_wave_delay_ms: 10_000,
        despawn_after_ms: (!persistent).then_some(20_000),
    }
}
pub(super) fn initial(persistent: bool) -> Change {
    Snapshot::initialize(key(), 1, [2; 32], timing(persistent), 0).expect("proposed initialization")
}
pub(super) fn facts(now: i64) -> CheckFacts {
    CheckFacts {
        now_ms: now,
        proposed_firing_id: id(10),
        proposed_targets: vec![channel(2), channel(3)],
        fixed_slot_ms: None,
    }
}
pub(super) fn fired(persistent: bool) -> Snapshot {
    initial(persistent)
        .after
        .check(&facts(60_000))
        .expect("check")
        .expect("due")
        .after
}
