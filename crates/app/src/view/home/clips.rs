use rune::{Cursor, El, el, row, text};
use rune_kit::Kit;

use crate::ui::library::format::{RecentClip, clip_title, game_secs_at, recent_clips};
use crate::ui::{Ui, format_clock};
use crate::view::thumb::{self, ASPECT};
use crate::view::{TABULAR, titled_card};

const LIMIT: usize = 4;
const GAP: f32 = 12.0;

pub(super) fn section(s: &Ui, k: &Kit) -> Option<El> {
    let clips = recent_clips(&s.recordings, LIMIT);
    if clips.is_empty() {
        return None;
    }
    let mut strip = row().w_full().padding(12.0).gap(GAP);
    for clip in &clips {
        strip = strip.child(item(clip, k));
    }
    for _ in clips.len()..LIMIT {
        strip = strip.child(el().w(0.0).grow(1.0).a11y_hidden());
    }
    Some(titled_card(k, "recent-clips", "Recent clips").child(strip))
}

fn item(clip: &RecentClip, k: &Kit) -> El {
    let champion = clip.entry.meta.game.info.me.champion.as_str();
    let title = clip_title(&clip.clip.label);
    let id = clip.entry.meta.id.clone();
    let index = clip.index;
    let game_time = format_clock(game_secs_at(clip.entry, clip.clip.start_secs) as u32);
    let picture = thumb::clip_thumb(k, clip.entry, clip.clip, el().w_full().aspect_ratio(ASPECT));
    rune::col()
        .key(format!("home-clip-{id}-{index}"))
        .role("button")
        .label(format!("Play {title} clip from {champion} game"))
        .w(0.0)
        .grow(1.0)
        .min_w(0.0)
        .gap(6.0)
        .padding(4.0)
        .radius(k.radius.md)
        .hover(k.c.hover)
        .hover_group()
        .cursor(Cursor::Pointer)
        .on_press(move |ui: &mut Ui| ui.open_clip(id.clone(), index))
        .child(picture)
        .child(
            text(title)
                .font(k.ty.body_medium)
                .color(k.c.text_base)
                .truncate(),
        )
        .child(
            text(format!("{champion} · {game_time}"))
                .font(k.ty.small.features(TABULAR))
                .color(k.c.text_muted)
                .truncate(),
        )
}
