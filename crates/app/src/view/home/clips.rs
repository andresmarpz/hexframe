use rune::{Cursor, El, col, el, row, text};
use rune_kit::Kit;

use crate::ui::library_format::{RecentClip, clip_title, game_secs_at, recent_clips};
use crate::ui::{Ui, format_clock};
use crate::view::champion::champion_icon;
use crate::view::{card, hairline};

pub(super) const LIMIT: usize = 6;
const ROW_HEIGHT: f32 = 52.0;
const ICON_SIZE: f32 = 32.0;
const TABULAR: &str = "'tnum' 1";

pub(super) fn section(s: &Ui, k: &Kit) -> El {
    let clips = recent_clips(&s.recordings, LIMIT);
    let mut grid = col().w_full().padding(8.0);
    for pair in clips.chunks(2) {
        let mut line = row().w_full().gap(8.0);
        for clip in pair {
            line = line.child(item(clip, k));
        }
        if pair.len() == 1 {
            line = line.child(el().w(0.0).grow(1.0).a11y_hidden());
        }
        grid = grid.child(line);
    }
    let body = if clips.is_empty() {
        row().w_full().h(ROW_HEIGHT).px(16.0).center_y().child(
            text("Clips you save show up here.")
                .key("recent-clips-empty")
                .font(k.ty.body)
                .color(k.c.text_muted),
        )
    } else {
        grid
    };
    card(k)
        .key("recent-clips")
        .child(
            row().w_full().h(44.0).px(16.0).center_y().child(
                text("Recent clips")
                    .font(k.ty.body_medium)
                    .color(k.c.text_base),
            ),
        )
        .child(hairline(k))
        .child(body)
}

fn item(clip: &RecentClip, k: &Kit) -> El {
    let champion = clip.entry.meta.game.info.me.champion.as_str();
    let title = clip_title(&clip.clip.label);
    let id = clip.entry.meta.id.clone();
    let index = clip.index;
    let game_time = format_clock(game_secs_at(clip.entry, clip.clip.start_secs) as u32);
    row()
        .key(format!("home-clip-{id}-{index}"))
        .role("button")
        .label(format!("Play {title} clip from {champion} game"))
        .w(0.0)
        .grow(1.0)
        .min_w(0.0)
        .h(ROW_HEIGHT - 8.0)
        .px(8.0)
        .gap(12.0)
        .center_y()
        .radius(k.radius.md)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .on_press(move |ui: &mut Ui| ui.open_clip(id.clone(), index))
        .child(champion_icon(k, champion, ICON_SIZE))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .child(
                    text(title)
                        .font(k.ty.body_medium)
                        .color(k.c.text_base)
                        .truncate(),
                )
                .child(
                    text(champion.to_owned())
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .truncate(),
                ),
        )
        .child(
            text(game_time)
                .font(k.ty.ui.features(TABULAR))
                .color(k.c.text_muted),
        )
}
