mod actions;
mod clips;
mod encoder;
mod feed;
mod first_run;
mod last_game;
mod week;

use greyscreen_core::problem::Problem;
use greyscreen_engine::EngineStatus;
use rune::{Color, El, col, el, row, text};
use rune_kit::{ControlSize, IconButton, IconButtonVariant, Kit, Switch, glyphs};

use super::widgets::champion::champion_icon;
use super::{card, title_style};
use crate::ui::library::format::mode_label;
use crate::ui::{Detail, Ui, format_clock};

pub(super) fn page(s: &Ui, k: &Kit) -> El {
    let page = col()
        .key("home")
        .w_full()
        .gap(16.0)
        .children(encoder::error_card(s, k))
        .child(state_card(s, k));
    if s.detail() == Detail::Live {
        return page.h_full().min_h(0.0).child(feed::panel(s, k));
    }
    let now = crate::qa::now();
    let mut page = page;
    if !s.settings.first_run_done {
        page = page.child(first_run::card_for(s, k));
    }
    if let Some(entry) = &s.last_game {
        page = page.child(last_game::panel(entry, k));
    }
    if let Some(recent) = clips::section(s, k) {
        page = page.child(recent);
    }
    page.child(week::section(s, k, &now))
}

struct Headline {
    pill: El,
    title: String,
    detail: El,
    clock: Option<String>,
    danger: bool,
}

fn muted(k: &Kit, line: impl Into<String>) -> El {
    text(line.into()).font(k.ty.body).color(k.c.text_muted)
}

fn headline(s: &Ui, k: &Kit) -> Headline {
    let quiet = |label: &str, title: &str, detail: &str| Headline {
        pill: pill(k, label, k.c.text_faint, k.c.layer_02, k.c.text_muted),
        title: title.to_owned(),
        detail: muted(k, detail),
        clock: None,
        danger: false,
    };
    match &s.status {
        EngineStatus::Idle if s.settings.auto_record => quiet(
            "Idle",
            "Waiting for a game",
            "Greyscreen starts recording when a game starts.",
        ),
        EngineStatus::Idle => quiet(
            "Off",
            "Recording off",
            "Games won't be recorded until you turn this on.",
        ),
        EngineStatus::GameRunning => quiet(
            "Off",
            "Game in progress, not recording",
            "Turn on Record games to start recording this game.",
        ),
        EngineStatus::Waiting => quiet(
            "Waiting",
            "League is running",
            "Waiting for the game to load.",
        ),
        EngineStatus::Finalizing => quiet("Saving", "Saving...", "Finishing the video file."),
        EngineStatus::Recording { game, .. } => {
            let (title, detail) = match game {
                Some(info) => (
                    info.me.champion.clone(),
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(champion_icon(k, &info.me.champion, 20.0))
                        .child(muted(k, format!("{} · Recording", mode_label(&info.mode)))),
                ),
                None => (
                    "Test recording".to_owned(),
                    muted(k, "Recording the primary screen."),
                ),
            };
            Headline {
                pill: pill_recording(k),
                title,
                detail,
                clock: Some(format_clock(s.elapsed_secs)),
                danger: false,
            }
        }
        EngineStatus::Error(message) => {
            let problem = Problem::from_error(message);
            Headline {
                pill: pill(k, "Error", k.c.danger.icon, k.c.danger.bg, k.c.danger.fg),
                title: problem.title().to_owned(),
                detail: problem_detail(problem, message, k),
                clock: None,
                danger: true,
            }
        }
    }
}

fn problem_detail(problem: Problem, raw: &str, k: &Kit) -> El {
    let mut lines = col().gap(4.0).child(
        text(problem.action())
            .key("error-action")
            .font(k.ty.body)
            .color(k.c.text_base),
    );
    if let Some(also) = problem.also() {
        lines = lines.child(muted(k, also));
    }
    if problem == Problem::Other {
        lines = lines.child(
            text(raw.to_owned())
                .key("error-message")
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    }
    lines
}

fn pill_recording(k: &Kit) -> El {
    pill(
        k,
        "Recording",
        k.c.danger.icon,
        k.c.danger.bg,
        k.c.danger.fg,
    )
}

fn state_card(s: &Ui, k: &Kit) -> El {
    let h = headline(s, k);
    let title_color = if h.danger {
        k.c.danger.fg
    } else {
        k.c.text_base
    };
    let left = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(10.0)
        .child(h.pill)
        .child(
            col()
                .gap(2.0)
                .child(
                    text(h.title)
                        .key("status-headline")
                        .font(title_style(k))
                        .color(title_color),
                )
                .child(h.detail),
        )
        .child(el().h(2.0))
        .child(actions::bar(s, k));
    let mut controls = row().gap(10.0).center_y().child(record_switch(s, k));
    if h.danger {
        controls = controls.child(dismiss_button(k));
    }
    let mut right = col().items_end().gap(14.0).child(controls);
    if let Some(clock) = h.clock {
        right = right.child(
            col()
                .items_end()
                .gap(2.0)
                .child(
                    text(clock)
                        .key("elapsed")
                        .font(rune::TextStyle {
                            size: 40.0,
                            line_height: 44.0,
                            ..k.ty.mono
                        })
                        .color(k.c.text_base),
                )
                .child(text("elapsed").font(k.ty.small).color(k.c.text_muted)),
        );
    }
    let body = row()
        .w_full()
        .gap(16.0)
        .justify_between()
        .items_start()
        .child(left)
        .child(right);
    let mut card = card(k).key("state-card").padding(20.0);
    if h.danger {
        card = card.border(1.0, k.c.danger.icon);
    }
    card.child(body)
}

fn dismiss_button(k: &Kit) -> El {
    IconButton::new(&glyphs::CLOSE)
        .label("Dismiss")
        .help("Dismiss")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .on_press(|ui: &mut Ui| ui.dismiss_error())
        .build(k)
        .key("dismiss-error")
}

fn record_switch(s: &Ui, k: &Kit) -> El {
    row()
        .gap(10.0)
        .center_y()
        .child(
            text("Record games")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(
            Switch::new(s.settings.auto_record)
                .label("Record games".to_owned())
                .hide_label()
                .on_toggle(|ui: &mut Ui| ui.change(|s| s.auto_record = !s.auto_record))
                .build(k)
                .key("record-games"),
        )
}

fn pill(k: &Kit, label: &str, dot: Color, fill: Color, ink: Color) -> El {
    row()
        .self_start()
        .h(24.0)
        .px(10.0)
        .gap(6.0)
        .center_y()
        .radius(12.0)
        .fill(fill)
        .child(el().size(8.0, 8.0).radius(4.0).fill(dot).a11y_hidden())
        .child(text(label).font(k.ty.ui_medium).color(ink))
}
