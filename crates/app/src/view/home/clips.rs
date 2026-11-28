use jiff::Zoned;
use rune::{Align, Cursor, El, col, el, row, text};
use rune_kit::Kit;

use super::surface::{header, sized, surface};
use crate::ui::library::format::{RecentClip, clip_title, game_secs_at, recent_clips};
use crate::ui::{Ui, format_clock};
use crate::view::{TABULAR, hairline, thumb};

const LIMIT: usize = 2;
const ROW_HEIGHT: f32 = 70.0;
const THUMB: (f32, f32) = (96.0, 54.0);
const WEEK_SECS: i64 = 7 * 24 * 60 * 60;

pub(super) fn section(s: &Ui, k: &Kit, now: &Zoned) -> Option<El> {
    let clips = recent_clips(&s.recordings, LIMIT);
    if clips.is_empty() {
        return None;
    }
    let fresh = fresh_count(s, now);
    let mut list = surface(k).key("recent-clips-card");
    for (n, clip) in clips.iter().enumerate() {
        if n > 0 {
            list = list.child(el().w_full().px(16.0).child(hairline(k)));
        }
        list = list.child(item(clip, k));
    }
    Some(
        col()
            .key("recent-clips")
            .w(0.0)
            .grow(1.0)
            .min_w(0.0)
            .gap(8.0)
            .child(header(
                k,
                "Recent clips",
                (fresh > 0).then(|| format!("{fresh} new")),
            ))
            .child(list),
    )
}

fn fresh_count(s: &Ui, now: &Zoned) -> usize {
    let cutoff = now.timestamp().as_second() - WEEK_SECS;
    s.recordings
        .iter()
        .flat_map(|entry| {
            let start = entry.meta.started_at.timestamp().as_second() as f64;
            entry.meta.clips.iter().map(move |clip| start + clip.start_secs)
        })
        .filter(|at| *at >= cutoff as f64)
        .count()
}

fn item(clip: &RecentClip, k: &Kit) -> El {
    let champion = clip.entry.meta.game.info.me.champion.as_str();
    let title = clip_title(&clip.clip.label);
    let id = clip.entry.meta.id.clone();
    let index = clip.index;
    let game_time = format_clock(game_secs_at(clip.entry, clip.clip.start_secs) as u32);
    let length = format_clock((clip.clip.end_secs - clip.clip.start_secs).max(0.0) as u32);
    row()
        .key(format!("home-clip-{id}-{index}"))
        .role("button")
        .label(format!("Play {title} clip from {champion} game"))
        .w_full()
        .h(ROW_HEIGHT)
        .px(16.0)
        .py(8.0)
        .gap(12.0)
        .center_y()
        .cursor(Cursor::Pointer)
        .hover(k.c.hover)
        .hover_group()
        .on_press(move |ui: &mut Ui| ui.open_clip(id.clone(), index))
        .child(thumb::clip_thumb_small(k, clip.entry, clip.clip, THUMB))
        .child(
            col()
                .w(0.0)
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(title)
                        .font(sized(k.ty.ui_medium, 13.0, 18.0))
                        .color(k.c.text_base)
                        .truncate(),
                )
                .child(
                    text(format!("at {game_time}"))
                        .font(sized(k.ty.ui, 12.0, 16.0).features(TABULAR))
                        .color(k.c.text_muted),
                ),
        )
        .child(
            text(length)
                .font(sized(k.ty.ui, 12.0, 16.0).features(TABULAR))
                .color(k.c.text_muted)
                .w(40.0)
                .align(Align::End),
        )
}
