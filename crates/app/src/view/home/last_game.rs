use greyscreen_core::game::GameResult;
use greyscreen_core::gamedata::map_name;
use greyscreen_core::timeline::{kill_participation, minute_boxes};
use greyscreen_engine::RecordingEntry;
use jiff::Zoned;
use rune::{Cursor, El, Icon, TextStyle, col, el, row, text};
use rune_kit::Kit;

use super::surface::{sized, surface};
use crate::ui::library::format::{kda, mode_label, review_progress};
use crate::ui::{Ui, format_clock};
use crate::view::widgets::champion::champion_icon;
use crate::view::widgets::minute_timeline::minute_timeline;
use crate::view::{TABULAR, hairline, thumb};

const THUMB_SIZE: (f32, f32) = (192.0, 108.0);
const PLAY: f32 = 32.0;
const TIMELINE_WIDTH: f32 = 848.0;

static PLAY_GLYPH: Icon = Icon {
    name: "home-play",
    view: 32.0,
    body: r#"<path d="M13 10.5L21.5 16L13 21.5Z" fill="currentColor"/>"#,
};

static CHEVRON: Icon = Icon {
    name: "home-chevron",
    view: 16.0,
    body: r#"<path d="M6 3.5L10.5 8L6 12.5" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub(super) fn card(entry: &RecordingEntry, k: &Kit, now: &Zoned) -> El {
    let champion = entry.meta.game.info.me.champion.as_str();
    let id = entry.meta.id.clone();
    surface(k)
        .key("last-game-card")
        .role("button")
        .label(format!("Open {champion} game"))
        .cursor(Cursor::Pointer)
        .hover(k.c.hover)
        .on_press(move |ui: &mut Ui| ui.open_game(id.clone()))
        .child(summary(entry, k, now))
        .child(el().w_full().px(16.0).child(hairline(k)))
        .child(timeline(entry, k))
}

fn summary(entry: &RecordingEntry, k: &Kit, now: &Zoned) -> El {
    row()
        .w_full()
        .padding(16.0)
        .gap(16.0)
        .center_y()
        .child(picture(entry, k))
        .child(
            col()
                .w(0.0)
                .grow(1.0)
                .min_w(0.0)
                .h(THUMB_SIZE.1)
                .justify_between()
                .child(title_line(entry, k, now))
                .child(champion_line(entry, k))
                .child(stats(entry, k)),
        )
        .child(
            rune::icon(&CHEVRON, 16.0)
                .color(k.c.text_faint)
                .a11y_hidden(),
        )
}

fn picture(entry: &RecordingEntry, k: &Kit) -> El {
    let (w, h) = THUMB_SIZE;
    let disc = row()
        .size(PLAY, PLAY)
        .center()
        .radius(PLAY / 2.0)
        .fill(k.c.scrim)
        .border(0.5, k.light(0.2))
        .child(rune::icon(&PLAY_GLYPH, PLAY).color(k.c.white));
    el().key("last-game-thumb")
        .size(w, h)
        .radius(8.0)
        .clip_rounded(8.0)
        .fill(k.c.layer_02)
        .center()
        .child(thumb::game_hero(k, entry, w, h).pin(Some(0.0), Some(0.0), None, None))
        .child(disc)
        .a11y_hidden()
}

fn title_line(entry: &RecordingEntry, k: &Kit, now: &Zoned) -> El {
    let (word, color) = match entry.meta.game.result {
        GameResult::Win => ("Victory", k.c.success.fg),
        GameResult::Loss => ("Defeat", k.c.danger.fg),
        GameResult::Unknown => ("Game", k.c.text_base),
    };
    let word_style = TextStyle {
        weight: 600.0,
        tracking: -0.2,
        ..sized(k.ty.body_medium, 20.0, 26.0)
    };
    row()
        .w_full()
        .justify_between()
        .center_y()
        .child(
            text(word)
                .key("last-game-result")
                .font(word_style)
                .color(color),
        )
        .child(
            text(format!("Last game · {}", ago(entry, now)))
                .font(sized(k.ty.ui, 12.0, 16.0))
                .color(k.c.text_muted),
        )
}

fn champion_line(entry: &RecordingEntry, k: &Kit) -> El {
    let info = &entry.meta.game.info;
    let mut parts = vec![info.me.champion.clone()];
    let mode = mode_label(&info.mode);
    let map = info.map.and_then(map_name).map(str::to_owned);
    parts.extend(map.filter(|map| *map != mode));
    parts.push(mode);
    parts.push(format_clock(entry.meta.duration_secs as u32));
    row()
        .gap(8.0)
        .center_y()
        .child(champion_icon(k, &info.me.champion, 20.0))
        .child(
            text(parts.join(" · "))
                .font(sized(k.ty.ui, 13.0, 18.0).features(TABULAR))
                .color(k.c.text_muted)
                .truncate(),
        )
}

fn stats(entry: &RecordingEntry, k: &Kit) -> El {
    let meta = &entry.meta;
    let figures = kda(&meta.events, &meta.game.info);
    let mut items = vec![(
        format!(
            "{} / {} / {}",
            figures.kills, figures.deaths, figures.assists
        ),
        "KDA".to_owned(),
    )];
    if let Some(percent) = kill_participation(&meta.events, &meta.game.info) {
        items.push((format!("{percent}%"), "kill participation".to_owned()));
    }
    items.push((meta.clips.len().to_string(), "clips".to_owned()));
    if let Some(progress) = review_progress(entry) {
        items.push((progress.replace('/', " / "), "deaths reviewed".to_owned()));
    }
    let mut line = row().key("last-game-stats").gap(16.0).center_y();
    for (n, (value, label)) in items.into_iter().enumerate() {
        if n > 0 {
            line = line.child(el().size(1.0, 16.0).fill(k.c.border_muted).a11y_hidden());
        }
        line = line.child(stat(k, value, label));
    }
    line
}

fn stat(k: &Kit, value: String, label: String) -> El {
    let number = sized(k.ty.ui_medium.features(TABULAR), 15.0, 20.0);
    let caption = sized(k.ty.ui.features(TABULAR), 12.0, 16.0);
    row()
        .gap(6.0)
        .center_y()
        .child(text(value).font(number).color(k.c.text_base))
        .child(text(label).font(caption).color(k.c.text_muted))
}

fn timeline(entry: &RecordingEntry, k: &Kit) -> El {
    let meta = &entry.meta;
    let boxes = minute_boxes(&meta.events, &meta.game.info, meta.duration_secs);
    col()
        .w_full()
        .px(16.0)
        .py(14.0)
        .child(minute_timeline(k, &boxes, TIMELINE_WIDTH))
}

fn ago(entry: &RecordingEntry, now: &Zoned) -> String {
    let ended = entry.meta.started_at.timestamp().as_second() + entry.meta.duration_secs as i64;
    let secs = (now.timestamp().as_second() - ended).max(0);
    let unit =
        |count: i64, name: &str| format!("{count} {name}{} ago", if count == 1 { "" } else { "s" });
    match secs {
        0..=59 => "just now".to_owned(),
        60..=3599 => unit(secs / 60, "minute"),
        3600..=86399 => unit(secs / 3600, "hour"),
        _ => unit(secs / 86400, "day"),
    }
}
