//! Gregorian UTC presentation for the server's absolute house dates.
pub(super) fn format(milliseconds: i64) -> String {
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
#[cfg(test)]
mod tests {
    #[test]
    fn house_confirmation_dates_preserve_utc_calendar_and_leap_day() {
        assert_eq!(super::format(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(super::format(951_868_799_000), "2000-02-29 23:59:59 UTC");
        assert_eq!(super::format(951_868_800_000), "2000-03-01 00:00:00 UTC");
    }
}
