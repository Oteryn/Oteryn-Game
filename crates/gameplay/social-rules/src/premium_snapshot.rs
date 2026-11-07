//! Pure Premium snapshot identifier, timestamp and authority-interval arithmetic.

/// The largest revision on the wire, 2^53 - 1 (PREM-P §5.1).
pub const MAX_WIRE_REVISION: u64 = (1 << 53) - 1;

/// The cutoff and refresh arithmetic of Platform's Premium v1 snapshot contract.
/// Active and not-yet-effective snapshots supply their entitlement end; other states do not.
pub fn authority_timing_valid(
    issued: i64,
    valid: i64,
    refresh_after_us: i64,
    effective_until_us: Option<i64>,
    max_authority_lease_us: i64,
) -> bool {
    const SECOND_US: i64 = 1_000_000;
    let cutoff = match effective_until_us {
        Some(until) => (issued + max_authority_lease_us).min(until),
        None => issued + max_authority_lease_us,
    };
    let lease_s = (valid - issued) / SECOND_US;
    valid == cutoff
        && refresh_after_us == issued + 2 * lease_s / 3 * SECOND_US
        && refresh_after_us < valid
}

/// Whether a value has exactly `len` lowercase hexadecimal bytes.
pub fn lower_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A lowercase hyphenated RFC 9562 UUIDv7 (version 7, RFC variant).
pub fn uuid_v7(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 36
        && [8, 13, 18, 23].iter().all(|&i| b[i] == b'-')
        && value.split('-').map(str::len).eq([8, 4, 4, 4, 12])
        && lower_hex(&value.replace('-', ""), 32)
        && b[14] == b'7'
        && matches!(b[19], b'8' | b'9' | b'a' | b'b')
}

/// Lowercase hyphenated UUID text of an AccountId.
pub fn canonical_uuid(id: [u8; 16]) -> String {
    let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// `YYYY-MM-DDTHH:MM:SS[.ffffff]Z` (at most six fraction digits, no leap second, year 1970 or
/// later) as Unix microseconds. Any other form, offset included, is refused.
pub fn rfc3339_utc_micros(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' {
        return None;
    }
    if b[16] != b':' || *b.last()? != b'Z' {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let digits = b.get(range)?;
        digits
            .iter()
            .all(u8::is_ascii_digit)
            .then(|| std::str::from_utf8(digits).ok()?.parse().ok())?
    };
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let fraction = &b[19..b.len() - 1];
    let micros = match fraction {
        [] => 0,
        [b'.', digits @ ..] if (1..=6).contains(&digits.len()) => {
            let value = number(20..20 + digits.len())?;
            value * 10_i64.pow(6 - digits.len() as u32)
        }
        _ => return None,
    };
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if year < 1970
        || !(1..=12).contains(&month)
        || day < 1
        || day > month_days[usize::try_from(month - 1).ok()?]
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    // Days from civil (Howard Hinnant), valid for the proleptic Gregorian calendar.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y / 400;
    let yoe = y - era * 400;
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 86_400 + hour * 3_600 + minute * 60 + second) * 1_000_000) + micros)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premium_refresh_rounds_to_whole_seconds_and_respects_entitlement_cutoff() {
        const SECOND: i64 = 1_000_000;
        const HOUR: i64 = 3_600 * SECOND;
        let issued = 10 * HOUR;
        assert!(authority_timing_valid(
            issued,
            issued + HOUR,
            issued + 2_400 * SECOND,
            None,
            HOUR
        ));
        assert!(authority_timing_valid(
            issued,
            issued + 5 * SECOND,
            issued + 3 * SECOND,
            Some(issued + 5 * SECOND),
            HOUR
        ));
        assert!(!authority_timing_valid(
            issued,
            issued + 5 * SECOND,
            issued + 3 * SECOND + 1,
            Some(issued + 5 * SECOND),
            HOUR
        ));
        assert!(!authority_timing_valid(
            issued,
            issued + HOUR,
            issued + 2_400 * SECOND,
            Some(issued + 5 * SECOND),
            HOUR
        ));
        assert!(!authority_timing_valid(
            issued,
            issued,
            issued,
            Some(issued),
            HOUR
        ));
    }

    #[test]
    fn timestamps_keep_gregorian_leap_rules_and_microsecond_precision() {
        assert_eq!(rfc3339_utc_micros("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            rfc3339_utc_micros("2000-02-29T00:00:01.5Z"),
            Some(951_782_401_500_000)
        );
        for invalid in [
            "1900-02-29T00:00:00Z",
            "2100-02-29T00:00:00Z",
            "2000-02-30T00:00:00Z",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00.1234567Z",
            "2026-01-01T00:00:60Z",
        ] {
            assert_eq!(rfc3339_utc_micros(invalid), None, "{invalid}");
        }
    }
}
