//! Pure stamina arithmetic from accepted OFFLINE-0 §3 and §5.
//!
//! Raw numbers are not authority: the future settlement owner must independently qualify
//! the latest logout marker, immediately preceding lease and database time anchors. It must
//! supply zero qualifying duration after a crash/missing marker and apply this result only
//! in its fenced, replayed build-state transaction. This module reads no clock or record,
//! issues no receipt and does not decide whether a session may settle.

/// OFFLINE-0 §3: 42 hours, represented as whole stamina minutes.
pub(crate) const MAX_STAMINA_MINUTES: u32 = 2_520;
/// OFFLINE-0 §5: the top three hours recover at the slower rate.
const NORMAL_STAMINA_MINUTES: u64 = 2_340;
/// OFFLINE-0 §5 caps the elapsed interval before applying the recovery delay.
pub(crate) const MAX_OFFLINE_SECONDS: u64 = 21 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StaminaOutOfRange;

/// Computes one proposed value; it does not consume a marker or mutate durable state.
/// `raw_offline_seconds` carries no provenance or current-authority guarantee. The caller
/// must qualify it before using the result; repeated calls are not settlement idempotency.
pub(crate) fn regenerate_stamina_minutes(
    stamina_minutes: u32,
    raw_offline_seconds: u64,
) -> Result<u32, StaminaOutOfRange> {
    if stamina_minutes > MAX_STAMINA_MINUTES {
        return Err(StaminaOutOfRange);
    }
    let recoverable_seconds = raw_offline_seconds
        .min(MAX_OFFLINE_SECONDS)
        .saturating_sub(600);
    if recoverable_seconds < 180 {
        return Ok(stamina_minutes);
    }

    let stamina = u64::from(stamina_minutes);
    let normal_minutes_missing = NORMAL_STAMINA_MINUTES - stamina.min(NORMAL_STAMINA_MINUTES);
    let normal_gain = recoverable_seconds / 180;
    let after = if normal_gain <= normal_minutes_missing {
        stamina + normal_gain
    } else {
        stamina.max(NORMAL_STAMINA_MINUTES)
            + (recoverable_seconds - 180 * normal_minutes_missing) / 360
    };
    // The validated starting value and this cap bound the conversion to 0..=2520.
    Ok(after.min(u64::from(MAX_STAMINA_MINUTES)) as u32)
}
