//! Gregorian UTC presentation for the server's absolute house dates.
pub fn format(milliseconds: i64) -> String {
    let seconds = milliseconds.div_euclid(1000);
    let days = seconds.div_euclid(86_400);
    let within = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        within / 3600,
        within / 60 % 60,
        within % 60
    )
}
/// Parse the displayed UTC calendar value, without using the workstation timezone.
pub fn parse(value: &str) -> Result<i64, &'static str> {
    if value.len() > 40 || !value.is_ascii() {
        return Err("Invalid UTC date");
    }
    let value = value
        .trim()
        .strip_suffix(" UTC")
        .ok_or("Use YYYY-MM-DD HH:MM:SS UTC")?;
    let (date, time) = value.split_once(' ').ok_or("Use YYYY-MM-DD HH:MM:SS UTC")?;
    let date = date
        .split('-')
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Invalid date")?;
    let time = time
        .split(':')
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Invalid time")?;
    let [year, month, day] = date.as_slice() else {
        return Err("Invalid date");
    };
    let [hour, minute, second] = time.as_slice() else {
        return Err("Invalid time");
    };
    if !(1970..=9999).contains(year)
        || !(1..=12).contains(month)
        || !(0..=23).contains(hour)
        || !(0..=59).contains(minute)
        || !(0..=59).contains(second)
    {
        return Err("Invalid UTC date");
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=days).contains(day) {
        return Err("Invalid day");
    }
    let y = year - i64::from(*month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = month + if *month > 2 { -3 } else { 9 };
    let doy = (153 * m + 2) / 5 + day - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    Ok((days * 86400 + hour * 3600 + minute * 60 + second) * 1000)
}

#[cfg(test)]
mod tests {
    #[test]
    fn house_confirmation_dates_preserve_utc_calendar_and_leap_day() {
        assert_eq!(super::format(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(super::format(951_868_799_000), "2000-02-29 23:59:59 UTC");
        assert_eq!(super::format(951_868_800_000), "2000-03-01 00:00:00 UTC");
        assert_eq!(super::parse("2000-02-29 23:59:59 UTC"), Ok(951_868_799_000));
        for invalid in [
            "1900-02-29 00:00:00 UTC",
            "2026-02-30 12:00:00 UTC",
            "2026-11-09 25:00:00 UTC",
            "2026-11-09 12:00:00",
        ] {
            assert!(super::parse(invalid).is_err());
        }
    }
}
