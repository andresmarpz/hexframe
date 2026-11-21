mod card;
mod death_summary;
mod deaths;
mod icons;
mod panel;
mod theater;
mod timeline;
mod transport;

#[cfg(test)]
mod tests;

use greyscreen_core::game::GameResult;
use greyscreen_engine::reveal;
use jiff::Zoned;
use rune::{Color, El, PlayerState, TextStyle, Tx, col, el, row, text};
use rune_kit::{
    Badge, BadgeVariant, Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit,
    Toolbar, glyphs,
};

use super::champion::champion_icon;
use super::delete::delete_key;
use crate::ui::library_format::{kda, mode_label, total_time, when};
use crate::ui::{ReviewState, Ui, format_clock};

const CHAMPION_ICON: f32 = 28.0;
const PAGE_PADDING: f32 = 16.0;
const NO_VIDEO: &str = "No video for this game";

pub(super) use card::hover_card;
pub(super) use theater::screen as theater;

pub(super) fn tabular(style: TextStyle) -> TextStyle {
    if style.features.is_empty() {
        style.features("'tnum' 1")
    } else {
        style
    }
}

pub(super) fn tone_pill(k: &Kit, label: &str, colors: rune_kit::StateColors) -> El {
    row()
        .h(22.0)
        .px(10.0)
        .center()
        .radius(11.0)
        .fill(colors.bg)
        .child(
            text(label.to_owned())
                .font(k.ty.small_medium)
                .color(colors.fg),
        )
}

pub(super) fn play_button(review: &ReviewState, k: &Kit, variant: IconButtonVariant) -> El {
    let (icon, name) = if review.is_playing() {
        (&icons::PAUSE, "Pause")
    } else {
        (&icons::PLAY, "Play")
    };
    IconButton::new(icon)
        .label(name)
        .help(name)
        .shortcut("Space")
        .variant(variant)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| {
            if let Some(review) = &ui.review {
                review.toggle();
            }
        })
        .build(k)
        .key("review-play")
}

pub(super) fn clock_text(review: &ReviewState, k: &Kit, color: Color) -> El {
    let clock = format!(
        "{} / {}",
        format_clock(review.time() as u32),
        format_clock(review.duration() as u32)
    );
    text(clock)
        .key("review-clock")
        .font(tabular(k.ty.body))
        .color(color)
}

pub(super) fn page(s: &Ui, k: &Kit) -> El {
    let Some(review) = &s.review else {
        return col().key("review").w_full().h_full();
    };
    match &review.death {
        Some(state) if state.summary => death_summary::screen(review, state, k),
        Some(state) => deaths::screen(review, state, &s.death_note, k),
        None => screen(review, k),
    }
}

fn screen(review: &ReviewState, k: &Kit) -> El {
    col()
        .key("review")
        .w_full()
        .h_full()
        .min_h(0.0)
        .padding(PAGE_PADDING)
        .child(
            row()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .gap(16.0)
                .child(player_column(review, k))
                .child(panel::panel(review, k)),
        )
}

pub(super) fn result_label(result: GameResult) -> Option<&'static str> {
    match result {
        GameResult::Win => Some("Victory"),
        GameResult::Loss => Some("Defeat"),
        GameResult::Unknown => None,
    }
}

fn duration_label(secs: f64) -> String {
    if secs < 60.0 {
        format_clock(secs as u32)
    } else {
        total_time(secs)
    }
}

fn stats_line(review: &ReviewState) -> String {
    let meta = &review.entry.meta;
    let kda = kda(&meta.events, &meta.game.info.me);
    format!(
        "{}/{}/{} · {}",
        kda.kills,
        kda.deaths,
        kda.assists,
        duration_label(meta.duration_secs)
    )
}

pub(super) fn toolbar(review: &ReviewState, k: &Kit) -> Toolbar {
    let meta = &review.entry.meta;
    let champion = meta.game.info.me.champion.as_str();
    let video = review.entry.folder.join(&meta.video);
    let folder = IconButton::new(&glyphs::FOLDER_OPEN)
        .label("Open folder")
        .help("Open folder")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .on_press(move |_: &mut Ui| reveal(&video))
        .build(k)
        .key("review-folder");
    let id = meta.id.clone();
    let delete = IconButton::new(&icons::TRASH)
        .label("Delete recording")
        .help("Delete recording")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .on_press(move |ui: &mut Ui| ui.ask_delete(id.clone()))
        .build(k)
        .key(delete_key(&meta.id));
    let mut line = row()
        .key("review-header")
        .gap(10.0)
        .center_y()
        .child(champion_icon(k, champion, CHAMPION_ICON))
        .child(
            text(champion.to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .truncate(),
        )
        .child(
            Badge::new(mode_label(&meta.game.info.mode))
                .variant(BadgeVariant::Neutral)
                .build(k),
        );
    if let Some(label) = result_label(meta.game.result) {
        let colors = match meta.game.result {
            GameResult::Win => k.c.success,
            _ => k.c.danger,
        };
        line = line.child(tone_pill(k, label, colors).key("review-result"));
    }
    let when = when(&meta.started_at, &Zoned::now());
    let line = line
        .child(
            text(format!("{} {}", when.day, when.time))
                .font(k.ty.body)
                .color(k.c.text_muted),
        )
        .child(
            text(stats_line(review))
                .font(tabular(k.ty.body))
                .color(k.c.text_muted),
        );
    Toolbar::new()
        .leading(line)
        .trailing(deaths_button(review, k))
        .trailing(folder)
        .trailing(delete)
}

fn deaths_button(review: &ReviewState, k: &Kit) -> El {
    let count = review.death_count();
    if count == 0 || review.player.is_none() || review.death.is_some() {
        return el();
    }
    Button::new(format!("Review deaths ({count})"))
        .variant(ButtonVariant::Submit)
        .control_size(ControlSize::Regular)
        .shortcut("D")
        .on_press(|ui: &mut Ui| ui.review_deaths())
        .build(k)
        .key("review-deaths")
}

fn player_column(review: &ReviewState, k: &Kit) -> El {
    col()
        .key("review-player")
        .grow(1.0)
        .min_w(0.0)
        .min_h(0.0)
        .gap(8.0)
        .child(video(review, k))
        .child(transport::transport(review, k, false))
}

fn video(review: &ReviewState, k: &Kit) -> El {
    let frame = el()
        .key("review-video")
        .focusable(true)
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .radius(8.0)
        .fill(Color::BLACK);
    video_frame(review, k, frame)
}

fn video_frame(review: &ReviewState, k: &Kit, frame: El) -> El {
    let frame = frame.on_double_press_tx(|ui: &mut Ui, tx: &mut Tx| {
        let on = !ui.in_theater();
        ui.set_theater(on, tx);
    });
    let message = |label: String| {
        frame
            .clone()
            .center()
            .child(text(label).font(k.ty.body).color(k.c.text_muted))
    };
    match &review.player {
        None => message(NO_VIDEO.into()),
        Some(player) => match player.state() {
            PlayerState::Failed(err) => message(err),
            _ => player.show(frame),
        },
    }
}
