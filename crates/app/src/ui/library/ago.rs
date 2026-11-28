use jiff::Zoned;

use super::format::when;

const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;

fn plural(count: i64, unit: &str) -> String {
    if count == 1 {
        format!("1 {unit} ago")
    } else {
        format!("{count} {unit}s ago")
    }
}

pub(crate) fn ago(at: &Zoned, now: &Zoned) -> String {
    let secs = now.timestamp().as_second() - at.timestamp().as_second();
    match secs {
        i64::MIN..MINUTE => "Just now".to_owned(),
        MINUTE..HOUR => plural(secs / MINUTE, "minute"),
        HOUR..DAY => plural(secs / HOUR, "hour"),
        DAY..WEEK => plural(secs / DAY, "day"),
        _ => when(at, now).day,
    }
}
