mod rows;

use std::borrow::Cow;
use std::rc::Rc;

use rune::{El, Key, LoopSpec, Tx, col, el, text};
use rune_kit::{Button, ButtonVariant, Kit, Sidebar, SidebarHeader, TextInput, glyphs};

use super::widgets::clear_when_filled;
use super::widgets::grip;
use crate::ui::library::format::summary;
use crate::ui::{ListItem, PaneEdge, Ui};

pub(super) const ROW_HEIGHT: f32 = 60.0;
const PULSE_SECS: f64 = 1.4;
const TABULAR: &str = "'tnum' 1";

pub(super) fn list(s: &Ui, k: &Kit) -> El {
    col()
        .key("games-list")
        .w(s.pane_width(PaneEdge::List) as f32)
        .h_full()
        .min_h(0.0)
        .child(content(s, k))
        .child(grip(s, PaneEdge::List, "list-grip", "Resize games list"))
}

fn content(s: &Ui, k: &Kit) -> El {
    let items: Rc<[ListItem]> = s.list_items().into();
    let mut sidebar = Sidebar::new()
        .label("Games")
        .row_height(ROW_HEIGHT)
        .header_padding(4.0, 8.0)
        .header(search(s, k));
    if items.is_empty() {
        sidebar = sidebar.child(empty(s, k));
    } else {
        let key_items = items.clone();
        let kit = k.clone();
        sidebar = sidebar.virtual_rows(
            items.len(),
            move |s: &Ui, i| key_of(s, &key_items[i]),
            move |s: &Ui, i| match &items[i] {
                ListItem::Live => rows::live(s, &kit),
                ListItem::Day(label) => SidebarHeader::new(label.clone()).build(&kit),
                ListItem::Game(index) => match s.recordings.get(*index) {
                    Some(rec) => {
                        rows::game(rec, s.selected_id() == Some(rec.meta.id.as_str()), &kit)
                    }
                    None => el(),
                },
            },
        );
    }
    if !s.recordings.is_empty() {
        sidebar = sidebar.footer(footer(s, k));
    }
    sidebar.build(k)
}

fn key_of<'a>(s: &'a Ui, item: &ListItem) -> Cow<'a, str> {
    match item {
        ListItem::Live => Cow::Borrowed("live-row"),
        ListItem::Day(label) => Cow::Owned(format!("day-{label}")),
        ListItem::Game(index) => s
            .recordings
            .get(*index)
            .map_or(Cow::Borrowed("missing"), |rec| {
                Cow::Borrowed(rec.meta.id.as_str())
            }),
    }
}

pub(super) fn dot(color: rune::Color, pulse: bool) -> El {
    let dot = el().size(8.0, 8.0).radius(4.0).fill(color).a11y_hidden();
    if pulse {
        dot.looping(LoopSpec::opacity(PULSE_SECS, 0.0, 0.25, 1.0))
    } else {
        dot
    }
}

fn search(s: &Ui, k: &Kit) -> El {
    let input = TextInput::new(&s.filter.champion)
        .label("Search games")
        .placeholder("Search games")
        .leading_icon(&glyphs::SEARCH);
    clear_when_filled(input, &s.filter.champion.text)
        .width(None)
        .on_edit(|ui: &mut Ui| &mut ui.filter.champion)
        .on_key(search_key)
        .build(k)
        .key("filter-champion")
}

fn search_key(ui: &mut Ui, ev: &rune::KeyEvent, tx: &mut Tx) -> bool {
    match ev.key {
        Key::Up => ui.step_selection(-1),
        Key::Down => ui.step_selection(1),
        Key::Enter if ui.review.is_some() => tx.focus("review-video", true),
        _ => return false,
    }
    true
}

pub(super) fn pick(s: &Ui, k: &Kit) -> El {
    let (title, detail) = if s.recordings.is_empty() {
        (
            "No games yet",
            "Games you play are recorded and show up here.",
        )
    } else {
        ("Pick a game", "Choose a game from the list to review it.")
    };
    col()
        .key("games-pick")
        .w_full()
        .h_full()
        .gap(4.0)
        .center()
        .child(text(title).font(k.ty.body_medium).color(k.c.text_base))
        .child(text(detail).font(k.ty.body).color(k.c.text_muted))
}

fn empty(s: &Ui, k: &Kit) -> El {
    let (title, detail, action) = if s.filter.is_active() {
        (
            "No games match",
            "Try another champion, mode or result.",
            Some(
                Button::new("Clear search")
                    .variant(ButtonVariant::Neutral)
                    .on_press(|ui: &mut Ui| ui.filter.clear())
                    .build(k)
                    .key("clear-filters"),
            ),
        )
    } else {
        (
            "No games yet",
            "Games you play are recorded and show up here.",
            None,
        )
    };
    let mut body = col()
        .key("games-empty")
        .w_full()
        .px(16.0)
        .py(32.0)
        .gap(12.0)
        .items_center()
        .child(text(title).font(k.ty.body_medium).color(k.c.text_base))
        .child(
            text(detail)
                .wrap()
                .max_w(240.0)
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    if let Some(action) = action {
        body = body.child(action);
    }
    body
}

fn footer(s: &Ui, k: &Kit) -> El {
    let games = s.filter.apply(&s.recordings);
    text(summary(&games, s.recordings.len()))
        .key("recordings-summary")
        .font(k.ty.small.features(TABULAR))
        .color(k.c.text_faint)
        .truncate()
        .px(8.0)
        .pb(super::layout::dock_clearance(s))
        .a11y_hidden()
}
