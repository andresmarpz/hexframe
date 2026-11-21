use super::format_clock;

#[test]
fn a_clock_under_an_hour_shows_minutes_and_seconds() {
    assert_eq!(format_clock(0), "00:00");
    assert_eq!(format_clock(9), "00:09");
    assert_eq!(format_clock(59 * 60 + 59), "59:59");
}

#[test]
fn a_clock_of_an_hour_or_more_shows_the_hour() {
    assert_eq!(format_clock(3600), "1:00:00");
    assert_eq!(format_clock(3600 + 15 * 60 + 30), "1:15:30");
    assert_eq!(format_clock(10 * 3600 + 5 * 60 + 9), "10:05:09");
}
