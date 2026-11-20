#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod logging;
mod tray;
mod ui;
mod view;

use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, col, text};

#[derive(Default)]
struct State;

fn view(_: &State, _: &mut Cx<State>) -> El {
    let theme = tok::theme(true);
    col()
        .w_full()
        .h_full()
        .center()
        .fill(theme.bg)
        .child(text("Greyscreen").font(type_::TITLE).color(theme.text))
}

fn main() {
    rune::run(AppSpec::<State> {
        min_size: Some((720.0, 520.0)),
        ..AppSpec::new("Greyscreen", (960.0, 640.0), State::default, view)
    });
}
