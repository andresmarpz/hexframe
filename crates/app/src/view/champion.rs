use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use greyscreen_core::paths;
use greyscreen_league::ChampionIcons;
use rune::{El, Image, ImageCache, Waker, el, text};
use rune_kit::Kit;

static WAKER: OnceLock<Waker> = OnceLock::new();

#[derive(Default)]
struct Icons {
    cache: Option<ImageCache>,
    index: Option<ChampionIcons>,
    images: HashMap<String, Option<Image>>,
}

thread_local! {
    static ICONS: RefCell<Icons> = RefCell::new(Icons::default());
}

pub(crate) fn set_waker(waker: Waker) {
    drop(WAKER.set(waker));
}

pub(crate) fn reload() {
    ICONS.with(|icons| {
        let mut icons = icons.borrow_mut();
        icons.index = None;
        icons.images.clear();
    });
}

fn image_for(name: &str) -> Option<Image> {
    ICONS.with(|icons| {
        let mut icons = icons.borrow_mut();
        let icons = &mut *icons;
        if let Some(known) = icons.images.get(name) {
            return known.clone();
        }
        let index = icons
            .index
            .get_or_insert_with(|| ChampionIcons::open(&paths::champion_icons_dir()));
        let image = index.icon_path(name).map(|path| {
            let cache = icons.cache.get_or_insert_with(|| {
                let waker = WAKER.get().cloned().unwrap_or_else(|| Arc::new(|| {}));
                ImageCache::new(waker)
            });
            let image = cache.file(&path, 0.0, 1.0);
            if rune::is_headless() {
                image.wait().ok();
            }
            image
        });
        icons.images.insert(name.to_owned(), image.clone());
        image
    })
}

pub(crate) fn champion_icon(k: &Kit, name: &str, size: f32) -> El {
    let radius = (size / 5.0).round();
    let shown = image_for(name).filter(|image| image.is_ready() && image.error().is_none());
    match shown {
        Some(image) => el()
            .size(size, size)
            .radius(radius)
            .pixels(image.pixels())
            .a11y_hidden(),
        None => monogram(k, name, size, radius),
    }
}

fn monogram(k: &Kit, name: &str, size: f32, radius: f32) -> El {
    let letter = name
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    el().size(size, size)
        .radius(radius)
        .fill(k.c.layer_02)
        .center()
        .a11y_hidden()
        .child(text(letter).font(k.ty.body_medium).color(k.c.text_muted))
}
