use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::{Color, Cursor, El, LoopSpec, Side, TextStyle, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use crate::ui::{Screen, Ui, format_clock};

const BLOCK_HEIGHT: f32 = 52.0;
const DOT_SIZE: f32 = 8.0;
const DOT_TARGET: f32 = 36.0;
const PULSE_SECS: f64 = 1.6;
const PULSE_LOW: f32 = 0.4;

struct Status {
    title: String,
    detail: String,
    dot: Color,
    pulse: bool,
    clock: Option<String>,
}

fn status(s: &Ui, k: &Kit) -> Status {
    let quiet = |title: &str, detail: &str, dot: Color| Status {
        title: title.to_owned(),
        detail: detail.to_owned(),
        dot,
        pulse: false,
        clock: None,
    };
    match &s.status {
        EngineStatus::Idle if s.settings.auto_record => {
            quiet("Ready", "Waiting for a game", k.c.success.icon)
        }
        EngineStatus::Idle => quiet("Recording off", "Turn it on in Home", k.c.text_faint),
        EngineStatus::GameRunning => quiet("Not recording", "A game is running", k.c.text_faint),
        EngineStatus::Waiting => quiet("League is running", "Game is loading", k.c.warning.icon),
        EngineStatus::Finalizing => quiet("Saving", "Finishing the video", k.c.warning.icon),
        EngineStatus::Error(_) => quiet("Capture failed", "Open Home to retry", k.c.danger.icon),
        EngineStatus::Recording { game, .. } => Status {
            title: "Recording".to_owned(),
            detail: match game {
                Some(info) => info.me.champion.clone(),
                None => "Test recording".to_owned(),
            },
            dot: k.c.danger.icon,
            pulse: true,
            clock: Some(format_clock(s.elapsed_secs)),
        },
    }
}

fn dot(st: &Status) -> El {
    let dot = el()
        .key("nav-status-dot")
        .size(DOT_SIZE, DOT_SIZE)
        .radius(DOT_SIZE / 2.0)
        .fill(st.dot)
        .a11y_hidden();
    if st.pulse {
        dot.looping(LoopSpec::opacity(PULSE_SECS, 0.0, PULSE_LOW, 1.0))
    } else {
        dot
    }
}

fn summary(st: &Status) -> String {
    match &st.clock {
        Some(clock) => format!("{} {clock}, {}", st.title, st.detail),
        None => format!("{}, {}", st.title, st.detail),
    }
}

pub(super) fn block(s: &Ui, k: &Kit) -> El {
    let st = status(s, k);
    let label = format!("Recorder: {}. Go to Home", summary(&st));
    let mut card = row()
        .key("nav-status")
        .role("button")
        .label(label)
        .focusable(true)
        .w_full()
        .h(BLOCK_HEIGHT)
        .px(12.0)
        .gap(10.0)
        .center_y()
        .radius(k.radius.lg)
        .fill(k.c.bg_base)
        .border(1.0, k.c.border_muted)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .on_press(|ui: &mut Ui| ui.go(Screen::Home))
        .child(
            row()
                .size(DOT_SIZE + 2.0, DOT_SIZE + 2.0)
                .center()
                .child(dot(&st)),
        )
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .child(
                    text(st.title.clone())
                        .key("nav-status-title")
                        .font(k.ty.body_medium)
                        .color(k.c.text_base)
                        .truncate()
                        .a11y_hidden(),
                )
                .child(
                    text(st.detail.clone())
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .truncate()
                        .a11y_hidden(),
                ),
        );
    if let Some(clock) = st.clock {
        let font = TextStyle {
            features: "'tnum' 1",
            ..k.ty.ui_medium
        };
        card = card.child(
            text(clock)
                .key("nav-status-clock")
                .font(font)
                .color(k.c.danger.fg)
                .a11y_hidden(),
        );
    }
    let mut block = col().w_full().gap(8.0).child(card);
    if matches!(s.status, EngineStatus::Recording { .. }) {
        block = block.child(
            Button::new("Save clip")
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Small)
                .on_press(|ui: &mut Ui| ui.send(EngineCmd::SaveClipNow))
                .build(k)
                .key("nav-save-clip"),
        );
    }
    block
}

pub(super) fn dot_button(s: &Ui, k: &Kit) -> El {
    let st = status(s, k);
    let tip = summary(&st);
    row()
        .key("nav-status")
        .role("button")
        .label(format!("Recorder: {tip}. Go to Home"))
        .help(tip)
        .help_side(Side::Right)
        .focusable(true)
        .size(DOT_TARGET, DOT_TARGET)
        .center()
        .radius(k.radius.lg)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .on_press(|ui: &mut Ui| ui.go(Screen::Home))
        .child(dot(&st))
}
