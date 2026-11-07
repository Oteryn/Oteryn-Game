//! Accepted World daily profile: committed current boost interval and past fixed-slot raids.
use serde::{Deserialize, Serialize};

/// PUBPROJ-WORLD-DAILY-REQUEST-BYTES (10077).
pub const REQUEST_BYTES: usize = 131_072;
/// PUBPROJ-WORLD-DAILY-HISTORY (10078).
pub const MAX_HISTORY: usize = 32;
/// PUBPROJ-WORLD-DAILY-RAIDS (10079).
pub const MAX_RAIDS: usize = 256;
/// PUBPROJ-WORLD-DAILY-HISTORY-WINDOW (10080); UTC elapsed coverage, not reset length.
pub const HISTORY_SECONDS: u64 = 30 * 86_400;
/// PUBPROJ-WORLD-DAILY-BOOST-KEY-BYTES (10081).
pub const BOOST_KEY_BYTES: usize = 255;
/// PUBPROJ-WORLD-DAILY-RAID-KEY-BYTES (10082).
pub const RAID_KEY_BYTES: usize = 128;
pub const MAX_SECONDS: u64 = 9_223_372_036_854_775;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct History {
    pub reset_epoch: String,
    pub effective_from: String,
    pub effective_until: String,
    pub creature_key: String,
    pub boss_key: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaidState {
    Running,
    Done,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Raid {
    pub raid_key: String,
    pub scheduled_at: String,
    pub state: RaidState,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Daily {
    pub reset_epoch: String,
    pub effective_from: String,
    pub effective_until: String,
    pub creature_key: String,
    pub boss_key: String,
    pub history_complete_from: String,
    pub history: Vec<History>,
    pub raids: Vec<Raid>,
}
fn decimal(value: &str, max: u64) -> Option<u64> {
    value
        .parse::<u64>()
        .ok()
        .filter(|n| *n <= max && n.to_string() == value)
}
fn seconds(value: &str) -> Option<u64> {
    decimal(value, MAX_SECONDS)
}
fn key(value: &str, bound: usize) -> bool {
    !value.is_empty()
        && value.len() <= bound
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b":._/-".contains(&b))
}
impl Daily {
    /// No current clock is inferred here: the durable revision's observation is
    /// immutable on retries. The SQL producer owns calendar/DST/source evidence.
    pub(super) fn valid(&self, observed: i64) -> bool {
        let (Some(epoch), Some(from), Some(until), Some(floor), Ok(observed)) = (
            decimal(&self.reset_epoch, u64::MAX),
            seconds(&self.effective_from),
            seconds(&self.effective_until),
            seconds(&self.history_complete_from),
            u64::try_from(observed),
        ) else {
            return false;
        };
        if from >= until
            || observed < from
            || observed >= until
            || floor > from
            || floor < from.saturating_sub(HISTORY_SECONDS)
            || !key(&self.creature_key, BOOST_KEY_BYTES)
            || !key(&self.boss_key, BOOST_KEY_BYTES)
            || self.history.len() > MAX_HISTORY
            || self.raids.len() > MAX_RAIDS
        {
            return false;
        }
        let mut newer_epoch = epoch;
        let mut newer_from = from;
        for h in &self.history {
            let (Some(e), Some(start), Some(end)) = (
                decimal(&h.reset_epoch, u64::MAX),
                seconds(&h.effective_from),
                seconds(&h.effective_until),
            ) else {
                return false;
            };
            if e >= newer_epoch
                || start < floor
                || start >= end
                || end > newer_from
                || !key(&h.creature_key, BOOST_KEY_BYTES)
                || !key(&h.boss_key, BOOST_KEY_BYTES)
            {
                return false;
            }
            newer_epoch = e;
            newer_from = start;
        }
        let mut preceding: Option<(u64, &str)> = None;
        for r in &self.raids {
            let Some(at) = seconds(&r.scheduled_at) else {
                return false;
            };
            if !key(&r.raid_key, RAID_KEY_BYTES)
                || at < from
                || at >= until
                || at > observed
                || preceding.is_some_and(|prior| prior >= (at, r.raid_key.as_str()))
            {
                return false;
            }
            preceding = Some((at, &r.raid_key));
        }
        true
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "bounded committed golden fixtures in codec tests"
)]
mod tests {
    use super::super::{Body, Snapshot, encode};
    use super::*;
    const GOLDENS: [&str; 4] = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/public-projections/world_daily_dst_spring_v2.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/public-projections/world_daily_dst_autumn_v2.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/public-projections/world_daily_empty_history_v2.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/public-projections/world_daily_withdrawn_v2.json"
        )),
    ];
    fn snapshot(raw: &str) -> Snapshot {
        let v: serde_json::Value = serde_json::from_str(raw).unwrap();
        Snapshot {
            subject: v["world_id"].as_str().unwrap().into(),
            world_id: String::new(),
            projection_epoch: 1,
            projection_revision: 3,
            source_observed_at: v["source_observed_at"].as_str().unwrap().parse().unwrap(),
            body: Body::WorldDaily(serde_json::from_value(v["daily"].clone()).unwrap()),
        }
    }
    #[test]
    fn world_daily_shared_goldens_cover_dst_history_states_and_withdrawal() {
        for raw in GOLDENS {
            assert_eq!(encode("oteryn:game:primary", &snapshot(raw)).unwrap(), raw);
        }
        for (raw, hours) in [(GOLDENS[0], 23), (GOLDENS[1], 25)] {
            let s = snapshot(raw);
            let Body::WorldDaily(rows) = s.body else {
                unreachable!()
            };
            let h = &rows[0].history[0];
            assert_eq!(
                seconds(&h.effective_until).unwrap() - seconds(&h.effective_from).unwrap(),
                hours * 3600
            );
            assert_eq!(
                rows[0].raids.iter().map(|r| r.state).collect::<Vec<_>>(),
                [RaidState::Running, RaidState::Done, RaidState::Cancelled]
            );
        }
    }
    #[test]
    fn world_daily_refuses_future_raids_overlap_unknown_keys_and_coverage_overclaims() {
        let baseline = snapshot(GOLDENS[0]);
        let Body::WorldDaily(rows) = &baseline.body else {
            unreachable!()
        };
        let d = &rows[0];
        let mut future = d.clone();
        future.raids[2].scheduled_at =
            (u64::try_from(baseline.source_observed_at).unwrap() + 1).to_string();
        assert!(!future.valid(baseline.source_observed_at));
        let mut overlap = d.clone();
        overlap.history[0].effective_until = (seconds(&d.effective_from).unwrap() + 1).to_string();
        assert!(!overlap.valid(baseline.source_observed_at));
        let mut key = d.clone();
        key.creature_key = "unknown cause%".into();
        assert!(!key.valid(baseline.source_observed_at));
        let mut coverage = d.clone();
        coverage.history_complete_from =
            (seconds(&d.effective_from).unwrap() - HISTORY_SECONDS - 1).to_string();
        assert!(!coverage.valid(baseline.source_observed_at));
        let mut subsecond = d.clone();
        subsecond.raids[0].scheduled_at.push_str(".5");
        assert!(!subsecond.valid(baseline.source_observed_at));
        let mut reordered = d.clone();
        reordered.raids.swap(0, 1);
        assert!(!reordered.valid(baseline.source_observed_at));
        let mut too_many = d.clone();
        too_many.history.clear();
        let start = seconds(&d.effective_from).unwrap();
        too_many.history_complete_from = (start - 33 * 60).to_string();
        for n in 1..=33_u64 {
            too_many.history.push(History {
                reset_epoch: (100 - n).to_string(),
                effective_from: (start - n * 60).to_string(),
                effective_until: (start - (n - 1) * 60).to_string(),
                creature_key: d.creature_key.clone(),
                boss_key: d.boss_key.clone(),
            });
        }
        assert!(!too_many.valid(baseline.source_observed_at));
    }
}
