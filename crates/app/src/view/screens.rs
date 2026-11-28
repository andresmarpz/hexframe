use rune::{El, Transition, col, row};
use rune_kit::{Divider, Kit, Page, Toolbar};

use super::layout::{fixed_content, scrolling_content};
use super::{clips, games, home, review, shell};
use crate::ui::{Detail, Screen, Ui};

fn settled(el: El) -> El {
    el.transition(Transition::none()).instant_changes()
}

pub(super) fn screen(s: &Ui, k: &Kit, review_width: Option<f32>) -> El {
    let (key, built) = match s.screen {
        Screen::Home => ("screen-home", home_screen(s, k)),
        Screen::Games => ("screen-games", games_screen(s, k, review_width)),
        Screen::Clips => ("screen-clips", clips_screen(s, k)),
    };
    settled(
        col()
            .key(key)
            .w_full()
            .h_full()
            .pt(shell::HEIGHT)
            .child(built),
    )
}

fn home_screen(s: &Ui, k: &Kit) -> El {
    let (main, toolbar) = match s.detail() {
        Detail::Live => (
            fixed_content(home::page(s, k)),
            Toolbar::new().title("Recording"),
        ),
        _ => (scrolling_content(home::page(s, k)), Toolbar::new()),
    };
    Page::new(main).toolbar(toolbar).build(k)
}

fn clips_screen(s: &Ui, k: &Kit) -> El {
    Page::new(scrolling_content(clips::page(s, k)))
        .title("Clips")
        .build(k)
}

fn games_screen(s: &Ui, k: &Kit, review_width: Option<f32>) -> El {
    let (main, toolbar) = match (s.detail(), &s.review) {
        (Detail::Game, Some(review)) => (
            review_page(s, k, review_width),
            review::toolbar(s, review, k),
        ),
        (Detail::Live, _) => (
            fixed_content(home::page(s, k)),
            Toolbar::new().title("Recording"),
        ),
        _ => (fixed_content(games::pick(s, k)), Toolbar::new()),
    };
    let hide_list = s.detail() == Detail::Game
        && (s.settings.games_list_hidden || s.review.as_ref().is_some_and(|r| r.death.is_some()));
    let mut body = row().key("games-body").w_full().h_full();
    if !hide_list {
        body = body
            .child(games::list(s, k))
            .child(Divider::vertical().build(k));
    }
    let body = body.child(col().grow(1.0).min_w(0.0).h_full().child(main));
    Page::new(body).toolbar(toolbar).build(k)
}

fn review_page(s: &Ui, k: &Kit, width: Option<f32>) -> El {
    review::page(s, k, width).pb(shell::REVIEW_ROOM)
}
