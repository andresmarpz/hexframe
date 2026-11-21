use jiff::Zoned;
use rune::{Cursor, El, col, el, row, text};
use rune_kit::Kit;

use super::widgets::thumb::{self, ASPECT};
use super::{card, hairline};
use crate::ui::library::format::{RecentClip, clip_title, game_secs_at, recent_clips, when};
use crate::ui::{Ui, format_clock};

const CARD_WIDTH: f32 = 256.0;
const GAP: f32 = 12.0;
const TABULAR: &str = "'tnum' 1";

pub(super) fn page(s: &Ui, k: &Kit) -> El {
    let clips = recent_clips(&s.recordings, usize::MAX);
    if clips.is_empty() {
        return col().key("clips").w_full().h(260.0).center().child(
            text("Clips you save show up here.")
                .key("clips-empty")
                .font(k.ty.body)
                .color(k.c.text_muted),
        );
    }
    let now = Zoned::now();
    let mut grid = row().key("clips-grid").w_full().flex_wrap().gap(GAP);
    for clip in &clips {
        grid = grid.child(item(clip, &now, k));
    }
    col().key("clips").w_full().child(grid)
}

fn item(clip: &RecentClip, now: &Zoned, k: &Kit) -> El {
    let entry = clip.entry;
    let champion = entry.meta.game.info.me.champion.as_str();
    let title = clip_title(&clip.clip.label);
    let id = entry.meta.id.clone();
    let index = clip.index;
    let game_time = format_clock(game_secs_at(entry, clip.clip.start_secs) as u32);
    let length = format_clock((clip.clip.end_secs - clip.clip.start_secs).max(0.0) as u32);
    let shown = when(&entry.meta.started_at, now);
    let font = k.ty.small.features(TABULAR);
    let picture = thumb::clip_thumb(k, entry, clip.clip, el().w_full().aspect_ratio(ASPECT));
    card(k)
        .key(format!("clip-{id}-{index}"))
        .role("button")
        .label(format!("Play {title} clip from {champion} game"))
        .w(CARD_WIDTH)
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
                .child(text(length).font(font).color(k.c.text_base))
                .child(el().grow(1.0).a11y_hidden())
                .child(
                    text(format!("{} {}", shown.day, shown.time))
                        .font(font)
                        .color(k.c.text_muted),
                ),
        )
}
