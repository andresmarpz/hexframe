use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use greyscreen_core::library::ClipEntry;
use greyscreen_core::library::thumb::{GAME_THUMB_FILE, clip_thumb_file};
use greyscreen_engine::RecordingEntry;
use rune::{Color, El, Icon, Image, ImageCache, Waker, el, row};
use rune_kit::{Badge, BadgeVariant, Kit};

use super::champion::champion_icon;
use crate::ui::format_clock;

pub(crate) const ASPECT: f32 = 16.0 / 9.0;
const REMEMBERED: usize = 160;
const BADGE_ICON_MIN: f32 = 14.0;
const BADGE_ICON_MAX: f32 = 20.0;
const BADGE_INSET: f32 = 4.0;

static PLAY: Icon = Icon {
    name: "greyscreen-thumb-play",
    view: 16.0,
    body: r#"<polygon points="5.5,3.5 12.5,8 5.5,12.5" fill="currentColor" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>"#,
};

static WAKER: OnceLock<Waker> = OnceLock::new();

#[derive(Default)]
struct Thumbs {
    cache: Option<ImageCache>,
    images: HashMap<PathBuf, Option<Image>>,
}

thread_local! {
    static THUMBS: RefCell<Thumbs> = RefCell::new(Thumbs::default());
}

pub(crate) fn set_waker(waker: Waker) {
    drop(WAKER.set(waker));
}

pub(crate) fn invalidate(path: &Path) {
    THUMBS.with(|thumbs| {
        thumbs.borrow_mut().images.remove(path);
    });
}

fn image_for(path: &Path) -> Option<Image> {
    THUMBS.with(|thumbs| {
        let mut thumbs = thumbs.borrow_mut();
        let thumbs = &mut *thumbs;
        if let Some(known) = thumbs.images.get(path) {
            return known.clone();
        }
        if thumbs.images.len() >= REMEMBERED {
            thumbs.images.clear();
        }
        let image = path.is_file().then(|| {
            let cache = thumbs.cache.get_or_insert_with(|| {
                let waker = WAKER.get().cloned().unwrap_or_else(|| Arc::new(|| {}));
                ImageCache::new(waker)
            });
            let image = cache.file(path, 0.0, 1.0);
            if rune::is_headless() {
                image.wait().ok();
            }
            image
        });
        thumbs.images.insert(path.to_owned(), image.clone());
        image
    })
}

const RADIUS: f32 = 6.0;
const PLAY_CHIP: f32 = 28.0;
const PLAY_CHIP_SMALL: f32 = 20.0;

fn picture(k: &Kit, path: &Path) -> El {
    let shown = image_for(path).filter(|image| image.is_ready() && image.error().is_none());
    let frame = el().w_full().h_full().a11y_hidden();
    match shown {
        Some(image) => frame.pixels(image.pixels()),
        None => frame.fill(k.c.layer_02),
    }
}

fn framed(k: &Kit, path: &Path, shape: El) -> El {
    shape
        .radius(RADIUS)
        .clip_rounded(RADIUS)
        .border(0.5, k.c.border_base)
        .child(picture(k, path))
}

fn pin_bottom_right(badge: El) -> El {
    badge.pin(None, None, Some(BADGE_INSET), Some(BADGE_INSET))
}

fn badge_icon(width: f32) -> f32 {
    (width * 0.2).clamp(BADGE_ICON_MIN, BADGE_ICON_MAX)
}

fn champion_badge(k: &Kit, champion: &str, ring: Color, icon: f32) -> El {
    let size = icon + 3.0;
    let chip = row()
        .size(size, size)
        .center()
        .radius(size / 2.0)
        .fill(ring)
        .child(champion_icon(k, champion, icon));
    chip.pin(Some(BADGE_INSET), None, None, Some(BADGE_INSET))
}

fn play_overlay(k: &Kit, chip: f32) -> El {
    let disc = row()
        .size(chip, chip)
        .center()
        .radius(chip / 2.0)
        .fill(k.alpha(k.c.bg_base, 0.88))
        .child(rune::icon(&PLAY, chip * 0.5).color(k.c.text_base));
    el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .center()
        .group_opacity(0.0, 1.0)
        .a11y_hidden()
        .child(disc)
}

pub(crate) fn game_path(rec: &RecordingEntry) -> PathBuf {
    rec.folder.join(GAME_THUMB_FILE)
}

pub(crate) fn clip_path(rec: &RecordingEntry, clip: &ClipEntry) -> PathBuf {
    rec.folder.join(clip_thumb_file(clip))
}

fn duration_badge(k: &Kit, secs: f64) -> El {
    Badge::new(format_clock(secs.max(0.0) as u32))
        .variant(BadgeVariant::Neutral)
        .build(k)
}

pub(crate) fn game_thumb(k: &Kit, rec: &RecordingEntry, width: f32) -> El {
    let shape = el().size(width, (width / ASPECT).round());
    let champion = rec.meta.game.info.me.champion.as_str();
    framed(k, &game_path(rec), shape)
        .child(play_overlay(k, PLAY_CHIP))
        .child(champion_badge(k, champion, k.c.bg_base, badge_icon(width)))
}

pub(crate) fn clip_thumb(k: &Kit, rec: &RecordingEntry, clip: &ClipEntry, shape: El) -> El {
    let length = clip.end_secs - clip.start_secs;
    framed(k, &clip_path(rec, clip), shape)
        .child(play_overlay(k, PLAY_CHIP))
        .child(pin_bottom_right(duration_badge(k, length)))
}

pub(crate) fn clip_thumb_small(
    k: &Kit,
    rec: &RecordingEntry,
    clip: &ClipEntry,
    size: (f32, f32),
) -> El {
    framed(k, &clip_path(rec, clip), el().size(size.0, size.1))
        .child(play_overlay(k, PLAY_CHIP_SMALL))
}
