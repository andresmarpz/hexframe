use greyscreen_core::problem::Problem;
use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::{Cursor, El, TextStyle, Transition, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Switch};

use super::sidebar::icons::{CLIPS, FEEDBACK, GAMES, GEAR, HOME};
use crate::ui::{Detail, Screen, Ui, format_clock};
use crate::view::champion::champion_icon;
use crate::view::widgets::dot;

const HEIGHT: f32 = 44.0;
const BOTTOM_GAP: f32 = 16.0;
pub(in crate::view) const CLEARANCE: f32 = HEIGHT + BOTTOM_GAP * 2.0;
const PAD: f32 = 4.0;
const IDLE_WIDTH: f32 = 128.0;
const RECORDING_WIDTH: f32 = 252.0;
const SLIDE: f32 = HEIGHT + BOTTOM_GAP + 8.0;
const HOVER_ZONE: f32 = 104.0;
const FRAME: f32 = 8.0;
const REVIEW_PADDING: f32 = 16.0;
pub(in crate::view) const REVIEW_ROOM: f32 = CLEARANCE - FRAME - REVIEW_PADDING;
const BLUR: f32 = 18.0;
const GLASS_ALPHA: f32 = 0.78;

pub(in crate::view) fn layer(s: &Ui, k: &Kit) -> El {
    let mut layer = row()
        .key("dock-layer")
        .pin(Some(0.0), None, Some(0.0), Some(BOTTOM_GAP))
        .justify_center()
        .animated_changes();
    if s.dock.visible {
        layer = layer.child(dock(s, k));
    }
    layer
}

pub(in crate::view) fn hover_zone(s: &Ui) -> Option<El> {
    (s.detail() == Detail::Game).then(|| {
        el().key("dock-zone")
            .pin(Some(0.0), None, Some(0.0), Some(0.0))
            .h(HOVER_ZONE)
            .on_hover_change(|ui: &mut Ui, inside, tx| ui.dock_hover(inside, tx))
            .on_pointer_move_local(|ui: &mut Ui, _, tx| ui.wake_dock(tx.clock()))
            .a11y_hidden()
    })
}

fn dock(s: &Ui, k: &Kit) -> El {
    let surface = row()
        .key("dock")
        .role("navigation")
        .label("Navigation")
        .h(HEIGHT)
        .padding(PAD)
        .gap(2.0)
        .center_y()
        .radius(HEIGHT / 2.0)
        .border(1.0, k.c.border_strong)
        .backdrop_blur(BLUR)
        .fill(k.alpha(k.c.bg_base, GLASS_ALPHA))
        .width_leads()
        .animated_changes()
        .transition(Transition::offset(0.0, SLIDE).combined(Transition::opacity()))
        .child(record_segment(s, k))
        .child(divider(k))
        .child(tab(s, k, Screen::Home, "Home", &HOME, "dock-home"))
        .child(tab(s, k, Screen::Games, "Games", &GAMES, "dock-games"))
        .child(tab(s, k, Screen::Clips, "Clips", &CLIPS, "dock-clips"))
        .child(divider(k))
        .child(settings_button(s, k))
        .child(feedback_button(s, k));
    k.elevate(surface, &k.elevation.floating)
}

fn divider(k: &Kit) -> El {
    el().w(1.0)
        .h(20.0)
        .mx(4.0)
        .fill(k.c.border_muted)
        .a11y_hidden()
}

fn tab(
    s: &Ui,
    k: &Kit,
    screen: Screen,
    label: &'static str,
    icon: &'static rune::Icon,
    key: &'static str,
) -> El {
    let variant = if s.screen == screen && !s.settings_open {
        ButtonVariant::Neutral
    } else {
        ButtonVariant::Ghost
    };
    Button::new(label)
        .icon(icon)
        .variant(variant)
        .control_size(ControlSize::Large)
        .on_press(move |ui: &mut Ui| ui.go(screen))
        .build(k)
        .key(key)
}

fn settings_button(s: &Ui, k: &Kit) -> El {
    let variant = if s.settings_open {
        ButtonVariant::Neutral
    } else {
        ButtonVariant::Ghost
    };
    Button::new("Settings")
        .icon(&GEAR)
        .variant(variant)
        .control_size(ControlSize::Large)
        .on_press(|ui: &mut Ui| ui.toggle_settings())
        .build(k)
        .key("dock-settings")
}

fn feedback_button(s: &Ui, k: &Kit) -> El {
    let variant = if s.feedback.open {
        IconButtonVariant::Neutral
    } else {
        IconButtonVariant::Ghost
    };
    IconButton::new(&FEEDBACK)
        .label("Send feedback")
        .help("Send feedback")
        .variant(variant)
        .control_size(ControlSize::Large)
        .on_press_tx(|ui: &mut Ui, tx: &mut rune::Tx| ui.open_feedback(tx))
        .build(k)
        .key("dock-feedback")
}

fn record_segment(s: &Ui, k: &Kit) -> El {
    let segment = row()
        .key("dock-record")
        .h_full()
        .center_y()
        .radius((HEIGHT - PAD * 2.0) / 2.0)
        .clip_rounded((HEIGHT - PAD * 2.0) / 2.0);
    match &s.status {
        EngineStatus::Recording { game, .. } => {
            let who = game
                .as_ref()
                .map_or("Test".to_owned(), |info| info.me.champion.clone());
            segment
                .w(RECORDING_WIDTH)
                .fill(k.c.danger.bg)
                .child(live(s, k, &who, game.is_some()))
                .child(clip_button(k))
        }
        _ => segment.w(IDLE_WIDTH).child(armed(s, k)),
    }
}

fn armed(s: &Ui, k: &Kit) -> El {
    let on = s.settings.auto_record;
    let (dot_color, ink) = match (&s.status, on) {
        (EngineStatus::Error(_), _) => (k.c.danger.icon, k.c.danger.fg),
        (_, true) => (k.c.success.icon, k.c.text_base),
        (_, false) => (k.c.text_faint, k.c.text_muted),
    };
    let label = match &s.status {
        EngineStatus::Finalizing => "Saving",
        EngineStatus::Waiting => "Loading",
        EngineStatus::Error(_) => "Error",
        _ => "Rec",
    };
    let tip = match &s.status {
        EngineStatus::Error(message) => {
            let problem = Problem::from_error(message);
            format!("{}. {}", problem.title(), problem.action())
        }
        _ => String::new(),
    };
    let armed = row()
        .key("dock-armed")
        .w_full()
        .px(12.0)
        .gap(8.0)
        .center_y()
        .child(dot(dot_color))
        .child(text(label).font(k.ty.ui_medium).color(ink))
        .child(col().grow(1.0).a11y_hidden())
        .child(
            Switch::new(on)
                .label("Record games".to_owned())
                .hide_label()
                .control_size(ControlSize::Small)
                .on_toggle(|ui: &mut Ui| ui.change(|s| s.auto_record = !s.auto_record))
                .build(k)
                .key("record-games"),
        );
    if tip.is_empty() {
        armed
    } else {
        armed.help(tip)
    }
}

fn live(s: &Ui, k: &Kit, who: &str, show_champion: bool) -> El {
    let clock = format_clock(s.elapsed_secs);
    let font = TextStyle {
        features: "'tnum' 1",
        ..k.ty.ui_medium
    };
    let mut body = row()
        .key("dock-live")
        .role("button")
        .label(format!("Recording {who}, {clock}. Go to Home"))
        .focusable(true)
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .pl(12.0)
        .gap(8.0)
        .center_y()
        .cursor(Cursor::Pointer)
        .on_press(|ui: &mut Ui| ui.go(Screen::Home))
        .child(dot(k.c.danger.icon))
        .child(
            text(format!("REC {clock}"))
                .key("dock-elapsed")
                .font(font)
                .color(k.c.danger.fg)
                .a11y_hidden(),
        )
        .child(text("·").font(k.ty.ui).color(k.c.text_faint).a11y_hidden());
    if show_champion {
        body = body.child(champion_icon(k, who, 18.0));
    }
    body.child(
        col().grow(1.0).min_w(0.0).child(
            text(who.to_owned())
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .truncate()
                .a11y_hidden(),
        ),
    )
}

fn clip_button(k: &Kit) -> El {
    el().pr(PAD / 2.0).child(
        Button::new("Clip")
            .icon(&CLIPS)
            .variant(ButtonVariant::Neutral)
            .control_size(ControlSize::Large)
            .help("Save the last seconds as a clip")
            .on_press(|ui: &mut Ui| ui.send(EngineCmd::SaveClipNow))
            .build(k)
            .key("dock-clip"),
    )
}
