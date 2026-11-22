use crate::qa::reveal;
use greyscreen_core::game::GameResult;
use greyscreen_core::health::Level;
use rune::{El, el, row, text};
use rune_kit::{
    Badge, BadgeVariant, Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit,
    Menu, MenuItem, Placement, Toolbar,
};

use super::{health, icons, tabular, tone_pill};
use crate::ui::library::format::{kda, mode_label, total_time, when};
use crate::ui::{ReviewState, Ui, format_clock};
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

pub(crate) fn toolbar(s: &Ui, review: &ReviewState, k: &Kit) -> Toolbar {
    let meta = &review.entry.meta;
    let champion = meta.game.info.me.champion.as_str();
    let mut line = row().key("review-header").shrink().gap(10.0).center_y();
    if review.death.is_none() {
        line = line.child(games_list_toggle(s, k));
    }
    line = line
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
    let mut bar = Toolbar::new()
        .leading(line)
        .trailing(deaths_button(review, k))
        .trailing(more_trigger(review, k));
    if review.death.is_none() {
        bar = bar.trailing(inspector_toggle(s, k));
    }
    bar
}

fn games_list_toggle(s: &Ui, k: &Kit) -> El {
    let hidden = s.settings.games_list_hidden;
    let label = if hidden {
        "Show games list"
    } else {
        "Hide games list"
    };
    IconButton::new(&icons::PANEL_LEFT)
        .label(label)
        .help(label)
        .shortcut("G")
        .variant(if hidden {
            IconButtonVariant::Ghost
        } else {
            IconButtonVariant::Neutral
        })
        .control_size(ControlSize::Regular)
        .on_press_tx(|ui: &mut Ui, tx: &mut rune::Tx| {
            tx.instant();
            ui.change(|s| s.games_list_hidden = !s.games_list_hidden);
        })
        .build(k)
        .key("review-toggle-games-list")
}

fn inspector_toggle(s: &Ui, k: &Kit) -> El {
    let hidden = s.settings.review_inspector_hidden;
    let label = if hidden {
        "Show events panel"
    } else {
        "Hide events panel"
    };
    IconButton::new(&icons::PANEL_RIGHT)
        .label(label)
        .help(label)
        .shortcut("I")
        .variant(if hidden {
            IconButtonVariant::Ghost
        } else {
            IconButtonVariant::Neutral
        })
        .control_size(ControlSize::Regular)
        .on_press_tx(|ui: &mut Ui, tx: &mut rune::Tx| {
            tx.instant();
            ui.change(|s| s.review_inspector_hidden = !s.review_inspector_hidden);
        })
        .build(k)
        .key("review-toggle-inspector")
}

fn health_alert(review: &ReviewState) -> Option<Level> {
    let findings = review.entry.meta.health.as_ref()?.findings();
    if findings.iter().any(|f| f.level == Level::Bad) {
        Some(Level::Bad)
    } else if findings.iter().any(|f| f.level == Level::Warn) {
        Some(Level::Warn)
    } else {
        None
    }
}

fn more_menu(open: bool) -> Menu {
    Menu::new("review-more-menu")
        .open(open)
        .placement(Placement::BottomEnd)
        .min_width(184.0)
        .on_close(|ui: &mut Ui| ui.review_more_menu(false))
}

fn more_trigger(review: &ReviewState, k: &Kit) -> El {
    let open = review.more_menu;
    let mut button = IconButton::new(&icons::MORE)
        .label("More actions")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .expanded(open)
        .on_press(move |ui: &mut Ui| ui.review_more_menu(!open));
    if !open && !review.health_open {
        button = button.help("More actions");
    }
    let button = button.build(k).key("review-more");
    let trigger = match health_alert(review) {
        Some(level) => {
            let color = if level == Level::Bad {
                k.c.danger.icon
            } else {
                k.c.warning.icon
            };
            el().child(button).child(
                el().key("review-more-alert")
                    .w(6.0)
                    .h(6.0)
                    .radius(3.0)
                    .fill(color)
                    .pin(None, Some(3.0), Some(3.0), None)
                    .a11y_hidden(),
            )
        }
        None => button,
    };
    let anchored = more_menu(open).trigger(trigger).build(k, &mut Vec::new());
    health::anchor(anchored, review.health_open, k)
}

pub(crate) fn more_overlay(review: &ReviewState, k: &Kit, layer: &mut Vec<El>) {
    if !review.more_menu {
        return;
    }
    let meta = &review.entry.meta;
    let video = review.entry.folder.join(&meta.video);
    let id = meta.id.clone();
    let mut health = MenuItem::new("Recording health").on_select(|ui: &mut Ui| {
        ui.review_more_menu(false);
        ui.review_health(true);
    });
    if health_alert(review).is_some() {
        health = health.description("Issues found");
    }
    more_menu(true)
        .item(health)
        .item(
            MenuItem::new("Show in folder").on_select(move |ui: &mut Ui| {
                ui.review_more_menu(false);
                reveal(&video);
            }),
        )
        .separator()
        .item(
            MenuItem::new("Delete…")
                .danger(true)
                .on_select(move |ui: &mut Ui| {
                    ui.review_more_menu(false);
                    ui.ask_delete(id.clone());
                }),
        )
        .build(k, layer);
}

fn deaths_button(review: &ReviewState, k: &Kit) -> El {
    let count = review.death_count();
    if count == 0 || review.player.is_none() || review.death.is_some() {
        return el();
    }
    Button::new(format!("Review deaths ({count})"))
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Regular)
        .shortcut("D")
        .on_press(|ui: &mut Ui| ui.review_deaths())
        .build(k)
        .key("review-deaths")
}
