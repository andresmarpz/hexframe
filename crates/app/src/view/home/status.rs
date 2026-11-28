use greyscreen_core::problem::Problem;
use greyscreen_engine::EngineStatus;
use rune::{El, col, el, row, text};
use rune_kit::{ControlSize, IconButton, IconButtonVariant, Kit, glyphs};

use super::actions;
use crate::view::widgets::champion::champion_icon;
use crate::view::widgets::{state_pill, status_pill};
use crate::view::{card, title_style};
use crate::ui::library::format::mode_label;
use crate::ui::{Ui, format_clock};

pub(super) fn card_for(s: &Ui, k: &Kit) -> Option<El> {
    headline(s, k).map(|h| state_card(s, k, h))
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

fn headline(s: &Ui, k: &Kit) -> Option<Headline> {
    Some(match &s.status {
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
                pill: state_pill(k, "Recording", k.c.danger, true),
                title,
                detail,
                clock: Some(format_clock(s.elapsed_secs)),
                danger: false,
            }
        }
        EngineStatus::Error(message) => {
            let problem = Problem::from_error(message);
            Headline {
                pill: state_pill(k, "Error", k.c.danger, true),
                title: problem.title().to_owned(),
                detail: problem_detail(problem, message, k),
                clock: None,
                danger: true,
            }
        }
        EngineStatus::Finalizing => Headline {
            pill: status_pill(k, "Saving", k.c.layer_02, k.c.text_muted, Some(k.c.text_faint)),
            title: "Saving...".to_owned(),
            detail: muted(k, "Finishing the video file."),
            clock: None,
            danger: false,
        },
        _ => return None,
    })
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

fn state_card(s: &Ui, k: &Kit, h: Headline) -> El {
    let title_color = if h.danger {
        k.c.danger.fg
    } else {
        k.c.text_base
    };
    let left = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(10.0)
        .child(h.pill.self_start())
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
        .children(actions::bar(s, k));
    let mut right = col().items_end().gap(14.0);
    if h.danger {
        right = right.child(row().gap(10.0).center_y().child(dismiss_button(k)));
    }
    let has_clock = h.clock.is_some();
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
    let mut body = row()
        .w_full()
        .gap(16.0)
        .justify_between()
        .items_start()
        .child(left);
    if h.danger || has_clock {
        body = body.child(right);
    }
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
