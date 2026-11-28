mod actions;
mod clips;
mod encoder;
mod feed;
mod first_run;
mod last_game;
mod status;
mod surface;
mod week;

use std::f32::consts::PI;

use rune::{El, col, el, row, text};
use rune_kit::Kit;

use super::layout::wide_scrolling_content;
use super::thumb::game_hero;
use crate::ui::{Detail, Screen, Ui};
use surface::{sized, surface};

const AMBIENT_HEIGHT: f32 = 504.0;
const AMBIENT_OPACITY: f32 = 0.42;
const AMBIENT_BLUR: f32 = 64.0;
const AMBIENT_WIDTH: f32 = 1600.0;
const CONTENT_TOP: f32 = 48.0;
const SECTION_GAP: f32 = 24.0;

pub(super) fn ambient(s: &Ui, k: &Kit) -> Option<El> {
    if s.screen != Screen::Home || s.detail() == Detail::Live {
        return None;
    }
    let entry = s.last_game.as_ref()?;
    let fade = el()
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .linear_gradient(PI, [(0.0, k.c.page.alpha(0.0)), (1.0, k.c.page)]);
    Some(
        el().key("home-ambient")
            .pin(Some(0.0), Some(0.0), Some(0.0), None)
            .h(AMBIENT_HEIGHT)
            .clip_rounded(0.0)
            .a11y_hidden()
            .child(
                game_hero(k, entry, AMBIENT_WIDTH, AMBIENT_HEIGHT)
                    .w_full()
                    .opacity(AMBIENT_OPACITY)
                    .blur(AMBIENT_BLUR),
            )
            .child(fade),
    )
}

pub(super) fn live(s: &Ui, k: &Kit) -> El {
    col()
        .key("home")
        .w_full()
        .h_full()
        .min_h(0.0)
        .gap(16.0)
        .children(encoder::error_card(s, k))
        .children(status::card_for(s, k))
        .child(feed::panel(s, k))
}

pub(super) fn page(s: &Ui, k: &Kit) -> El {
    let now = jiff::Zoned::now();
    let mut page = col()
        .key("home")
        .w_full()
        .gap(SECTION_GAP)
        .children(encoder::error_card(s, k))
        .children(status::card_for(s, k));
    if !s.settings.first_run_done {
        page = page.child(first_run::card_for(s, k));
    }
    if let Some(entry) = &s.last_game {
        page = page.child(last_game::card(entry, k, &now));
    }
    let week = week::section(s, k, &now);
    let recent = clips::section(s, k, &now);
    if week.is_some() || recent.is_some() {
        page = page.child(
            row()
                .w_full()
                .gap(SECTION_GAP)
                .items_start()
                .children(week)
                .children(recent),
        );
    }
    if s.recordings.is_empty() && s.settings.first_run_done {
        page = page.child(empty(k));
    }
    wide_scrolling_content(page, CONTENT_TOP)
}

fn empty(k: &Kit) -> El {
    surface(k)
        .key("home-empty")
        .padding(24.0)
        .gap(4.0)
        .child(
            text("No games yet")
                .font(sized(k.ty.ui_medium, 13.0, 18.0))
                .color(k.c.text_base),
        )
        .child(
            text("Your recorded games and clips show up here.")
                .font(sized(k.ty.ui, 13.0, 18.0))
                .color(k.c.text_muted),
        )
}
