mod hero;
mod highlights;

use greyscreen_core::game::GameResult;
use greyscreen_engine::RecordingEntry;
use rune::{Cursor, El, ScrollSpec, TextStyle, col, row, text};
use rune_kit::Kit;

use crate::ui::Ui;
use crate::ui::library::ago::ago;
use crate::view::shell;

const GUTTER: f32 = 24.0;
const SECTION_GAP: f32 = 16.0;
const CRUMB_TOP: f32 = 12.0;
const CRUMB_BOTTOM: f32 = 8.0;
const CRUMB_PAD: f32 = 4.0;

pub(super) fn view(s: &Ui, k: &Kit) -> El {
    let Some(rec) = s.game_page_entry() else {
        return col().key("game-page").w_full().h_full();
    };
    let now = jiff::Zoned::now();
    let when = ago(&rec.meta.started_at, &now);
    let mut sections = col()
        .w_full()
        .px(GUTTER)
        .pb(shell::CLEARANCE)
        .gap(SECTION_GAP)
        .child(hero::hero(s, rec, k, &when, is_latest(s, rec)));
    if let Some(strip) = highlights::section(s, rec, k) {
        sections = sections.child(strip);
    }
    col()
        .key("game-page")
        .w_full()
        .h_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(breadcrumb(rec, k, &when))
        .child(sections)
}

fn is_latest(s: &Ui, rec: &RecordingEntry) -> bool {
    s.recordings
        .iter()
        .map(|r| r.meta.started_at.timestamp())
        .max()
        .is_some_and(|latest| latest == rec.meta.started_at.timestamp())
}

pub(super) fn result_word(result: GameResult) -> &'static str {
    match result {
        GameResult::Win => "Victory",
        GameResult::Loss => "Defeat",
        GameResult::Unknown => "Unknown",
    }
}

fn crumb(k: &Kit, label: impl Into<String>, strong: bool) -> El {
    let color = if strong {
        k.c.text_base
    } else {
        k.c.text_muted
    };
    text(label)
        .font(TextStyle {
            size: 13.0,
            line_height: 16.0,
            ..k.ty.ui
        })
        .color(color)
}

fn breadcrumb(rec: &RecordingEntry, k: &Kit, when: &str) -> El {
    let champion = rec.meta.game.info.me.champion.as_str();
    let head = match rec.meta.game.result {
        GameResult::Unknown => format!("{champion} ·"),
        result => format!("{champion} · {} ·", result_word(result)),
    };
    let back = k.interactive(
        row()
            .key("game-back")
            .role("button")
            .label("Back to games")
            .padding(CRUMB_PAD)
            .radius(k.radius.sm)
            .cursor(Cursor::Pointer)
            .on_press(|ui: &mut Ui| ui.close_game())
            .child(crumb(k, "Games", true)),
    );
    row()
        .key("game-breadcrumb")
        .w_full()
        .pl(GUTTER - CRUMB_PAD)
        .pr(GUTTER)
        .pt(CRUMB_TOP)
        .pb(CRUMB_BOTTOM)
        .gap(4.0)
        .center_y()
        .child(back)
        .child(crumb(k, "›", false))
        .child(
            row()
                .gap(4.0)
                .child(crumb(k, head, true))
                .child(crumb(k, when, false)),
        )
}
