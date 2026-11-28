use std::f32::consts::PI;

use greyscreen_core::game::GameResult;
use greyscreen_core::gamedata::map_name;
use greyscreen_core::keybinds::Action;
use greyscreen_core::timeline::minute_boxes;
use greyscreen_engine::RecordingEntry;
use rune::{Color, El, Icon, TextStyle, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Keybind, Kit};

use super::result_word;
use crate::ui::library::format::{death_total, kda, mode_label};
use crate::ui::{MAC, Ui, format_clock};
use crate::view::TABULAR;
use crate::view::thumb::game_hero;
use crate::view::widgets::champion::champion_icon;
use crate::view::widgets::game_icons::FOLDER_REVEAL;
use crate::view::widgets::minute_timeline::minute_timeline;

const HEIGHT: f32 = 396.0;
const RADIUS: f32 = 12.0;
const INSET: f32 = 28.0;
const ACTIONS_BOTTOM: f32 = 76.0;
const TIMELINE_BOTTOM: f32 = 20.0;
const PLAY_SIZE: f32 = 64.0;
const CHAMPION_ICON: f32 = 28.0;

static PLAY: Icon = Icon {
    name: "greyscreen-hero-play",
    view: 16.0,
    body: r#"<path d="M4.5 2.5L13 8L4.5 13.5Z" fill="currentColor"/>"#,
};

pub(super) fn hero(s: &Ui, rec: &RecordingEntry, k: &Kit, when: &str, latest: bool) -> El {
    let meta = &rec.meta;
    let boxes = minute_boxes(&meta.events, &meta.game.info, meta.duration_secs);
    el().key("game-hero")
        .w_full()
        .h(HEIGHT)
        .radius(RADIUS)
        .clip_rounded(RADIUS)
        .fill(k.c.layer_02)
        .child(game_hero(k, rec, 1232.0, HEIGHT).w_full().h_full())
        .child(scrim(k))
        .child(play(k))
        .child(info(rec, k, when, latest))
        .child(actions(s, rec, k))
        .child(
            el().pin(Some(INSET), None, Some(INSET), Some(TIMELINE_BOTTOM))
                .child(minute_timeline(k, &boxes, 0.0)),
        )
}

fn scrim(k: &Kit) -> El {
    let page = k.c.page;
    el().key("game-hero-scrim")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .linear_gradient(
            PI,
            [
                (0.0, k.alpha(page, 0.15)),
                (0.5, k.alpha(page, 0.55)),
                (1.0, k.alpha(page, 0.92)),
            ],
        )
        .a11y_hidden()
}

fn play(k: &Kit) -> El {
    let disc = k
        .interactive(
            row()
                .key("game-play")
                .role("button")
                .label("Play game")
                .size(PLAY_SIZE, PLAY_SIZE)
                .center()
                .radius(PLAY_SIZE / 2.0)
                .fill(k.c.scrim)
                .border(1.0, k.light(0.2))
                .on_press(|ui: &mut Ui| ui.play_game_page()),
        )
        .child(rune::icon(&PLAY, 24.0).color(k.c.white));
    el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .center()
        .child(disc)
}

fn result_color(k: &Kit, result: GameResult) -> Color {
    match result {
        GameResult::Win => k.c.success.fg,
        GameResult::Loss => k.c.danger.fg,
        GameResult::Unknown => k.c.text_base,
    }
}

fn info(rec: &RecordingEntry, k: &Kit, when: &str, latest: bool) -> El {
    let meta = &rec.meta;
    let info = &meta.game.info;
    let kind = if latest { "Last game" } else { "Game" };
    let word = TextStyle {
        size: 56.0,
        weight: 650.0,
        line_height: 60.0,
        tracking: -1.68,
        ..k.ty.ui
    };
    let name = TextStyle {
        size: 16.0,
        line_height: 20.0,
        ..k.ty.ui_medium
    };
    let detail = TextStyle {
        size: 13.0,
        line_height: 20.0,
        ..k.ty.ui.features(TABULAR)
    };
    let figures = TextStyle {
        size: 15.0,
        line_height: 20.0,
        ..k.ty.ui_medium.features(TABULAR)
    };
    let score = kda(&meta.events, info);
    let mode = mode_label(&info.mode);
    let place = match info.map.and_then(map_name) {
        Some(map) if map != mode => format!("{map} · {mode}"),
        _ => mode,
    };
    let length = format_clock(meta.duration_secs.max(0.0) as u32);
    let stats = row()
        .key("game-kda")
        .gap(8.0)
        .center_y()
        .child(
            text(format!(
                "{} / {} / {}",
                score.kills, score.deaths, score.assists
            ))
            .font(figures)
            .color(k.c.text_base),
        )
        .child(text("KDA").font(k.ty.caption()).color(k.c.text_muted));
    col()
        .pin(Some(INSET), None, None, Some(ACTIONS_BOTTOM))
        .child(
            text(format!("{kind} · {when}"))
                .font(k.ty.caption())
                .color(k.c.text_muted),
        )
        .child(
            text(result_word(meta.game.result))
                .key("game-result")
                .font(word)
                .color(result_color(k, meta.game.result))
                .mt(4.0),
        )
        .child(
            row()
                .gap(10.0)
                .mt(12.0)
                .center_y()
                .child(champion_icon(k, &info.me.champion, CHAMPION_ICON))
                .child(
                    text(info.me.champion.clone())
                        .font(name)
                        .color(k.c.text_base),
                )
                .child(
                    text(format!("{place} · {length}"))
                        .font(detail)
                        .color(k.c.text_muted),
                ),
        )
        .child(stats.mt(12.0))
}

fn review_label(s: &Ui, k: &Kit) -> impl FnOnce(TextStyle, Color) -> El + 'static {
    let keys = s
        .settings
        .keymap()
        .bindings(Action::ReviewDeaths)
        .first()
        .map_or_else(|| vec!["D".to_owned()], |combo| combo.keycaps(MAC));
    let keybind = Keybind::new(&keys).build(k);
    move |style, color| {
        row()
            .gap(8.0)
            .center_y()
            .child(text("Review").font(style).color(color))
            .child(keybind)
    }
}

fn actions(s: &Ui, rec: &RecordingEntry, k: &Kit) -> El {
    let deaths = death_total(rec);
    let review = Button::new("Review")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .label_with(review_label(s, k))
        .disabled(deaths == 0)
        .on_press(|ui: &mut Ui| ui.review_game_page_deaths())
        .build(k)
        .key("game-review");
    let folder = IconButton::new(&FOLDER_REVEAL)
        .label("Show in folder")
        .help("Show in folder")
        .variant(IconButtonVariant::Neutral)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.reveal_game_page_folder())
        .build(k)
        .key("game-folder");
    row()
        .pin(None, None, Some(INSET), Some(ACTIONS_BOTTOM))
        .gap(8.0)
        .center_y()
        .child(review)
        .child(folder)
}
