use jiff::Zoned;
use rune::{Cursor, El, col, el, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, TextInput, glyphs,
};

use super::widgets::thumb::{self, ASPECT};
use super::widgets::{EXPORT, clear_when_filled};
use super::{TABULAR, card, hairline};
use crate::ui::library::format::{RecentClip, clip_title, game_secs_at, recent_clips, when};
use crate::ui::{Ui, format_clock};

const COLUMNS: usize = 3;
const GAP: f32 = 12.0;
const SEARCH_WIDTH: f32 = 320.0;

pub(super) fn page(s: &Ui, k: &Kit) -> El {
    let all = recent_clips(&s.recordings, usize::MAX);
    if all.is_empty() {
        return col().key("clips").w_full().h(260.0).center().child(
            text("Clips you save show up here.")
                .key("clips-empty")
                .font(k.ty.body)
                .color(k.c.text_muted),
        );
    }
    let query = s.clips_search.text.trim().to_ascii_lowercase();
    let clips: Vec<RecentClip<'_>> = if query.is_empty() {
        all
    } else {
        all.into_iter()
            .filter(|c| matches_query(c, &query))
            .collect()
    };
    let body = if clips.is_empty() {
        no_matches(k)
    } else {
        let now = crate::qa::now();
        let mut grid = col().key("clips-grid").w_full().gap(GAP);
        for chunk in clips.chunks(COLUMNS) {
            let mut line = row().w_full().gap(GAP);
            for clip in chunk {
                line = line.child(item(clip, &now, k));
            }
            for _ in chunk.len()..COLUMNS {
                line = line.child(el().w(0.0).grow(1.0).a11y_hidden());
            }
            grid = grid.child(line);
        }
        grid
    };
    col()
        .key("clips")
        .w_full()
        .gap(GAP)
        .child(el().w_full().max_w(SEARCH_WIDTH).child(search(s, k)))
        .child(body)
}

fn matches_query(clip: &RecentClip<'_>, query: &str) -> bool {
    clip_title(&clip.clip.label)
        .to_ascii_lowercase()
        .contains(query)
        || clip
            .entry
            .meta
            .game
            .info
            .me
            .champion
            .to_ascii_lowercase()
            .contains(query)
}

fn search(s: &Ui, k: &Kit) -> El {
    let input = TextInput::new(&s.clips_search)
        .label("Search clips")
        .placeholder("Search clips by champion or moment")
        .leading_icon(&glyphs::SEARCH);
    clear_when_filled(input, &s.clips_search.text, |ui| &mut ui.clips_search)
        .width(None)
        .on_edit(|ui: &mut Ui| &mut ui.clips_search)
        .build(k)
        .key("clips-search")
}

fn no_matches(k: &Kit) -> El {
    col()
        .w_full()
        .h(220.0)
        .gap(12.0)
        .center()
        .child(
            text("No clips match")
                .font(k.ty.body_medium)
                .color(k.c.text_base),
        )
        .child(
            Button::new("Clear search")
                .variant(ButtonVariant::Neutral)
                .on_press(|ui: &mut Ui| ui.clips_search.replace_text(""))
                .build(k)
                .key("clips-clear-search"),
        )
}

fn item(clip: &RecentClip, now: &Zoned, k: &Kit) -> El {
    let entry = clip.entry;
    let champion = entry.meta.game.info.me.champion.as_str();
    let title = clip_title(&clip.clip.label);
    let id = entry.meta.id.clone();
    let index = clip.index;
    let game_time = format_clock(game_secs_at(entry, clip.clip.start_secs) as u32);
    let shown = when(&entry.meta.started_at, now);
    let font = k.ty.small.features(TABULAR);
    let export_id = id.clone();
    let export = IconButton::new(&EXPORT)
        .label("Export clip")
        .help("Export…")
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Small)
        .on_press(move |ui: &mut Ui| ui.open_export_clip(&export_id, index))
        .build(k)
        .key(format!("export-clip-{id}-{index}"));
    let picture = thumb::clip_thumb(k, entry, clip.clip, el().w_full().aspect_ratio(ASPECT));
    card(k)
        .key(format!("clip-{id}-{index}"))
        .role("button")
        .label(format!("Play {title} clip from {champion} game"))
        .w(0.0)
        .grow(1.0)
        .min_w(0.0)
        .hover_group()
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .on_press(move |ui: &mut Ui| ui.open_clip(id.clone(), index))
        .child(
            col().w_full().padding(8.0).gap(8.0).child(picture).child(
                col()
                    .w_full()
                    .px(4.0)
                    .child(
                        text(title)
                            .font(k.ty.body_medium)
                            .color(k.c.text_base)
                            .truncate(),
                    )
                    .child(
                        text(format!("{champion} · {game_time}"))
                            .font(font)
                            .color(k.c.text_muted)
                            .truncate(),
                    ),
            ),
        )
        .child(hairline(k))
        .child(
            row()
                .w_full()
                .px(12.0)
                .h(32.0)
                .gap(8.0)
                .center_y()
                .child(text(shown.label()).font(font).color(k.c.text_muted))
                .child(el().grow(1.0).a11y_hidden())
                .child(export),
        )
}
