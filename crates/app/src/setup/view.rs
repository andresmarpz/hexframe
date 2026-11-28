use rune::{Align, Cx, El, col, row, text};
use rune_kit::{Button, ButtonVariant, Checkbox, ControlSize, Kit, Spinner};

use super::state::{Phase, SetupState};
use crate::logo::LOGO;

const LOGO_SIZE: f32 = 56.0;
const CONTENT_WIDTH: f32 = 340.0;
const ACTION_HEIGHT: f32 = 112.0;
const PRIVACY_URL: &str = "https://greyscreen.app/privacy/";

pub(super) fn root(s: &SetupState, cx: &mut Cx<SetupState>) -> El {
    let k = Kit::neutral(cx.system_dark());
    let body = col()
        .key("setup")
        .w_full()
        .h_full()
        .fill(k.c.bg_deep)
        .center()
        .gap(24.0)
        .child(heading(&k))
        .child(action(s, &k))
        .child(footer(s, &k));
    rune_kit::root(body, Vec::new())
}

fn footer(s: &SetupState, k: &Kit) -> El {
    col()
        .center()
        .gap(4.0)
        .child(
            text(format!("Installs to {}", s.install_path))
                .key("setup-path")
                .font(k.ty.small)
                .color(k.c.text_weak)
                .wrap()
                .align(Align::Center)
                .w(CONTENT_WIDTH),
        )
        .child(privacy_link(k))
}

fn heading(k: &Kit) -> El {
    col()
        .key("setup-heading")
        .center()
        .gap(10.0)
        .child(rune::icon(&LOGO, LOGO_SIZE).color(k.c.text_base).a11y_hidden())
        .child(
            text("Greyscreen")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(
            text("Records your League games on your GPU. Installs to your user folder, no admin needed.")
                .key("setup-blurb")
                .font(k.ty.body)
                .color(k.c.text_muted)
                .wrap()
                .align(Align::Center)
                .w(CONTENT_WIDTH),
        )
}

fn action(s: &SetupState, k: &Kit) -> El {
    let block = col()
        .key("setup-action")
        .center()
        .gap(14.0)
        .size(CONTENT_WIDTH, ACTION_HEIGHT);
    match &s.phase {
        Phase::Ready => block
            .child(options(s, k))
            .child(install_button(k, "Install")),
        Phase::Closing => block.child(busy(k, "Closing the running Greyscreen...")),
        Phase::Copying => block.child(busy(k, "Installing...")),
        Phase::Done => block
            .child(
                text("Greyscreen is installed. You can delete this download.")
                    .key("setup-done")
                    .font(k.ty.body)
                    .color(k.c.success.fg)
                    .wrap()
                    .align(Align::Center)
                    .w_full(),
            )
            .child(
                Button::new("Open Greyscreen")
                    .variant(ButtonVariant::Contrast)
                    .control_size(ControlSize::Large)
                    .on_press(|s: &mut SetupState| s.open_installed())
                    .build(k)
                    .key("setup-open"),
            ),
        Phase::Failed(reason) => block
            .child(
                text(reason.clone())
                    .key("setup-error")
                    .font(k.ty.body)
                    .color(k.c.danger.fg)
                    .wrap()
                    .align(Align::Center)
                    .w_full(),
            )
            .child(
                row()
                    .gap(8.0)
                    .child(
                        Button::new("Run without installing")
                            .variant(ButtonVariant::Neutral)
                            .control_size(ControlSize::Large)
                            .on_press(|s: &mut SetupState| s.run_here())
                            .build(k)
                            .key("setup-run-here"),
                    )
                    .child(install_button(k, "Try again")),
            ),
    }
}

fn options(s: &SetupState, k: &Kit) -> El {
    Checkbox::new(s.start_with_windows)
        .label("Start with Windows")
        .on_toggle(|s: &mut SetupState| s.toggle_start_with_windows())
        .build(k)
        .key("setup-start-with-windows")
}

fn install_button(k: &Kit, label: &str) -> El {
    Button::new(label.to_owned())
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Large)
        .on_press(|s: &mut SetupState| s.install())
        .build(k)
        .key("setup-install")
}

fn privacy_link(k: &Kit) -> El {
    Button::new("Privacy policy")
        .variant(ButtonVariant::GhostMuted)
        .control_size(ControlSize::Small)
        .on_press_tx(|_: &mut SetupState, tx: &mut rune::Tx| {
            if let Err(err) = tx.open_url(PRIVACY_URL) {
                tracing::warn!(%err, "cannot open the privacy policy");
            }
        })
        .build(k)
        .key("setup-privacy")
}

fn busy(k: &Kit, label: &str) -> El {
    col()
        .center()
        .gap(14.0)
        .w_full()
        .child(Spinner::new().build(k))
        .child(
            text(label.to_owned())
                .key("setup-busy")
                .font(k.ty.body)
                .color(k.c.text_muted),
        )
}
