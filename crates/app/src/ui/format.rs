use jiff::Zoned;

pub(crate) fn format_clock(secs: u32) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

pub(crate) fn format_size(bytes: u64) -> String {
    const MB: f64 = 1_000_000.0;
    let mb = bytes as f64 / MB;
    if mb >= 1000.0 {
        format!("{:.1} GB", mb / 1000.0)
    } else {
        format!("{mb:.0} MB")
    }
}

pub(crate) fn format_date(at: &Zoned) -> String {
    at.strftime("%Y-%m-%d %H:%M").to_string()
}

pub(crate) fn format_size_estimate(mb_per_minute: f64, native_height: Option<u32>) -> String {
    let per_minute = (mb_per_minute / 5.0).round().max(1.0) * 5.0;
    let per_game = per_minute * 30.0 / 1000.0;
    let game = if per_game < 10.0 {
        format!("{per_game:.1}")
    } else {
        format!("{per_game:.0}")
    };
    let suffix = native_height.map_or(String::new(), |h| format!(" (at {h}p)"));
    format!("About {per_minute:.0} MB per minute, about {game} GB for a 30-minute game{suffix}")
}
