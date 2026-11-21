use crate::qa::reveal;
use greyscreen_core::game::GameResult;
use rune::{El, el, row, text};
use rune_kit::{
    Badge, BadgeVariant, Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit,
    Toolbar, glyphs,
};

use super::{health, icons, tabular, tone_pill};
use crate::ui::library::format::{kda, mode_label, total_time, when};
use crate::ui::{ReviewState, Ui, format_clock};
use crate::view::overlays::delete_key;
use crate::view::widgets::champion::champion_icon;

const CHAMPION_ICON: f32 = 28.0;
const CHAMPION_MIN_WIDTH: f32 = 56.0;

pub(crate) fn result_label(result: GameResult) -> Option<&'static str> {
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
    let kda = kda(&meta.events, &meta.game.info);
    format!(
        "{}/{}/{} · {}",
        kda.kills,
        kda.deaths,
        kda.assists,
        duration_label(meta.duration_secs)
    )
}

pub(crate) fn toolbar(review: &ReviewState, k: &Kit) -> Toolbar {
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
        .shrink()
        .gap(10.0)
        .center_y()
        .child(champion_icon(k, champion, CHAMPION_ICON))
        .child(
            text(champion.to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .truncate()
                .min_w(CHAMPION_MIN_WIDTH),
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
    let when = when(&meta.started_at, &crate::qa::now());
    let line = line
        .child(
            text(when.label())
                .key("review-when")
                .font(k.ty.body)
                .color(k.c.text_muted)
                .truncate(),
        )
        .child(
            text(stats_line(review))
                .key("review-stats")
                .font(tabular(k.ty.body))
                .color(k.c.text_muted)
                .truncate(),
        );
    Toolbar::new()
        .leading(line)
        .trailing(deaths_button(review, k))
        .trailing(health::trigger(review, k))
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
