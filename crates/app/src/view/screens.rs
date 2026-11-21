use greyscreen_core::settings::NavStyle;
use rune::{El, Transition, col, el, row};
use rune_kit::{Divider, Kit, Page, Toolbar};

use super::layout::{fixed_content, scrolling_content};
use super::{clips, games, home, review, shell};
use crate::ui::{Detail, Screen, Ui};

fn settled(el: El) -> El {
    el.transition(Transition::none()).instant_changes()
}

pub(super) fn screen(s: &Ui, k: &Kit, inset: f32, review_width: Option<f32>) -> El {
    let (key, built) = match s.screen {
        Screen::Home => ("screen-home", home_screen(s, k, inset)),
        Screen::Games => ("screen-games", games_screen(s, k, inset, review_width)),
        Screen::Clips => ("screen-clips", clips_screen(s, k, inset)),
    };
    settled(col().key(key).w_full().h_full().child(built))
}

fn home_screen(s: &Ui, k: &Kit, inset: f32) -> El {
    let (main, toolbar) = match s.detail() {
        Detail::Live => (
            fixed_content(s, home::page(s, k)),
            Toolbar::new().title("Recording"),
        ),
        _ => (scrolling_content(s, home::page(s, k)), Toolbar::new()),
    };
    Page::new(main)
        .toolbar(toolbar)
        .titlebar_inset(inset)
        .build(k)
}

fn clips_screen(s: &Ui, k: &Kit, inset: f32) -> El {
    Page::new(scrolling_content(s, clips::page(s, k)))
        .title("Clips")
        .titlebar_inset(inset)
        .build(k)
}

fn games_screen(s: &Ui, k: &Kit, inset: f32, review_width: Option<f32>) -> El {
    let (main, toolbar) = match (s.detail(), &s.review) {
        (Detail::Game, Some(review)) => {
            (review_page(s, k, review_width), review::toolbar(review, k))
        }
        (Detail::Live, _) => (
            fixed_content(s, home::page(s, k)),
            Toolbar::new().title("Recording"),
        ),
        _ => (fixed_content(s, games::pick(s, k)), Toolbar::new()),
    };
    let body = row()
        .key("games-body")
        .w_full()
        .h_full()
        .child(games::list(s, k))
        .child(Divider::vertical().build(k))
        .child(col().grow(1.0).min_w(0.0).h_full().child(main));
    Page::new(body)
        .toolbar(toolbar)
        .titlebar_inset(inset)
        .build(k)
}

fn review_page(s: &Ui, k: &Kit, width: Option<f32>) -> El {
    let page = review::page(s, k, width);
    if s.settings.nav_style == NavStyle::Sidebar {
        return page;
    }
    let room = if s.dock.visible {
        shell::dock::REVIEW_ROOM
    } else {
        0.0
    };
    col()
        .w_full()
        .h_full()
        .child(page.grow(1.0).min_h(0.0))
        .child(el().key("dock-reserve").w_full().h(room).a11y_hidden())
}
