//! G-8 World UTC-day physical death totals. Private creature dimensions never
//! cross this wire (owner acceptance 2026-11-07, statistics proposal revision2).
use super::super::{SourceError, canonical_uuid, envelope_valid, finish};
use serde::{Deserialize, Serialize};

pub const REQUEST_BYTES: usize = 2_048;
pub const DAY_SECONDS: u64 = 86_400;
pub const RETENTION_SECONDS: u64 = 400 * DAY_SECONDS;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Day {
    pub day_end: String,
    pub monsters_killed: String,
    pub bosses_killed: String,
    pub complete: bool,
}
fn decimal(value: &str) -> Option<u64> {
    let number = value.parse::<u64>().ok()?;
    (number.to_string() == value).then_some(number)
}
pub fn valid(day_start: &str, observed: i64, days: &[Day]) -> bool {
    let Some(start) = decimal(day_start).filter(|s| *s > 0 && s % DAY_SECONDS == 0) else {
        return false;
    };
    let Some(end) = start.checked_add(DAY_SECONDS) else {
        return false;
    };
    let Ok(observed) = u64::try_from(observed) else {
        return false;
    };
    start <= observed
        && days.len() <= 1
        && days.iter().all(|day| {
            decimal(&day.day_end) == Some(end)
                && decimal(&day.monsters_killed).is_some()
                && decimal(&day.bosses_killed).is_some()
                && (!day.complete || end <= observed)
        })
}
#[derive(Serialize)]
struct Wire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    world_id: &'a str,
    day_start: &'a str,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    days: &'a [Day],
}
pub fn encode(
    authority: &str,
    world: &str,
    start: &str,
    epoch: u64,
    revision: u64,
    observed: i64,
    days: &[Day],
) -> Result<String, SourceError> {
    if !canonical_uuid(world, Some(b'7'))
        || !envelope_valid(authority, epoch, revision, observed)
        || !valid(start, observed, days)
    {
        return Err(SourceError::InvalidInput);
    }
    finish(
        serde_json::to_string(&Wire {
            contract_version: 2,
            operation: "PublishWorldDailyKillsV2",
            source_authority: authority,
            world_id: world,
            day_start: start,
            projection_epoch: epoch.to_string(),
            projection_revision: revision.to_string(),
            source_observed_at: observed.to_string(),
            days,
        }),
        REQUEST_BYTES,
    )
}
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "golden boundary assertions use synthetic valid source records"
)]
mod tests {
    use super::*;
    const AUTH: &str = "oteryn:game:primary";
    const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
    #[test]
    fn daily_kills_wire_matches_shared_golden_and_omits_private_dimensions() {
        let days = [Day {
            day_end: "1790035200".into(),
            monsters_killed: "12345".into(),
            bosses_killed: "12".into(),
            complete: true,
        }];
        let wire = encode(AUTH, WORLD, "1789948800", 1, 2, 1790035201, &days).unwrap();
        assert_eq!(
            wire,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../docs/contracts/fixtures/public-projections/world_daily_kills_complete_v2.json"
            ))
        );
        assert!(!wire.contains("creature_key"));
        assert!(!wire.contains("occurrence"));
        assert!(!wire.contains("channel_id"));
    }
    #[test]
    fn daily_kills_day_alignment_counts_and_complete_cut_are_strict() {
        let mut days = [Day {
            day_end: "1790035200".into(),
            monsters_killed: u64::MAX.to_string(),
            bosses_killed: "0".into(),
            complete: false,
        }];
        assert!(encode(AUTH, WORLD, "1789948800", 1, 1, 1789948800, &days).is_ok());
        days[0].complete = true;
        assert!(encode(AUTH, WORLD, "1789948800", 1, 1, 1789948800, &days).is_err());
        days[0].complete = false;
        days[0].monsters_killed = "18446744073709551616".into();
        assert!(encode(AUTH, WORLD, "1789948800", 1, 1, 1790035201, &days).is_err());
        assert!(encode(AUTH, WORLD, "1789948801", 1, 1, 1790035201, &[]).is_err());
        assert!(encode(AUTH, WORLD, "0", 1, 1, 1790035201, &[]).is_err());
    }
}
