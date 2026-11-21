use jiff::Zoned;
use rune::{El, col, row, text};
use rune_kit::Kit;

use crate::ui::Ui;
use crate::ui::library_format::{WeekStats, total_time, week_stats};
use crate::view::{card, hairline, section_header, title_style};

const TABULAR: &str = "'tnum' 1";

pub(super) fn section(s: &Ui, k: &Kit, now: &Zoned) -> El {
    let stats = week_stats(&s.recordings, now);
    card(k)
        .key("this-week")
        .child(section_header(k, "This week"))
        .child(hairline(k))
        .child(
            row()
                .w_full()
                .px(16.0)
                .py(14.0)
                .gap(16.0)
                .child(stat(k, "week-games", games(&stats), "Games"))
                .child(stat(k, "week-record", record(&stats), "Wins–Losses"))
                .child(stat(k, "week-win-rate", win_rate(&stats), "Win rate"))
                .child(stat(
                    k,
                    "week-time",
                    total_time(stats.secs),
                    "Time recorded",
                )),
        )
}

fn games(stats: &WeekStats) -> String {
    stats.games.to_string()
}

fn record(stats: &WeekStats) -> String {
    format!("{}–{}", stats.wins, stats.losses)
}

fn win_rate(stats: &WeekStats) -> String {
    stats
        .win_rate()
        .map_or_else(|| "—".to_owned(), |rate| format!("{rate}%"))
}

fn stat(k: &Kit, key: &'static str, value: String, label: &'static str) -> El {
    col()
        .w(0.0)
        .grow(1.0)
        .min_w(0.0)
        .gap(2.0)
        .child(
            text(value)
                .key(key)
                .font(title_style(k).features(TABULAR))
                .color(k.c.text_base),
        )
        .child(text(label).font(k.ty.small).color(k.c.text_muted))
}
