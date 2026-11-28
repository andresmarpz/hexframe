use std::sync::atomic::{AtomicBool, Ordering};

use greyscreen_engine::RecordingEntry;
use rune::El;
use rune::{Icon, col, el, row, text};
use rune_kit::Kit;

use super::game_icons::{
    BARON, DEATH, DRAGON, ELDER, FOLDER_REVEAL, GRUBS, HERALD, INHIBITOR, KILL, SETTINGS_GEAR,
    TOWER,
};
use super::thumb::game_hero;

const SIZES: [f32; 3] = [12.0, 14.0, 16.0];
const BOX: f32 = 32.0;

static ON: AtomicBool = AtomicBool::new(false);

pub(crate) fn show() {
    ON.store(true, Ordering::Relaxed);
}

pub(crate) fn shown() -> bool {
    ON.load(Ordering::Relaxed)
}

fn strip(k: &Kit, icon: &'static Icon, boxed: bool) -> El {
    let mut cells = row().gap(12.0).items_center();
    for size in SIZES {
        let glyph = rune::icon(icon, size).color(k.c.text_base);
        cells = cells.child(if boxed {
            row()
                .size(BOX, BOX)
                .center()
                .radius(6.0)
                .fill(k.c.layer_03)
                .child(glyph)
        } else {
            row().size(BOX, BOX).center().child(glyph)
        });
    }
    cells
}

fn line(k: &Kit, name: &'static str, icon: &'static Icon) -> El {
    row()
        .gap(16.0)
        .items_center()
        .child(
            text(name)
                .font(k.ty.body_medium)
                .color(k.c.text_muted)
                .w(96.0),
        )
        .child(strip(k, icon, false))
        .child(strip(k, icon, true))
}

pub(crate) fn sheet(k: &Kit, entry: Option<&RecordingEntry>) -> El {
    let icons: [(&'static str, &'static Icon); 11] = [
        ("kill", &KILL),
        ("death", &DEATH),
        ("dragon", &DRAGON),
        ("elder", &ELDER),
        ("baron", &BARON),
        ("herald", &HERALD),
        ("grubs", &GRUBS),
        ("tower", &TOWER),
        ("inhibitor", &INHIBITOR),
        ("folder", &FOLDER_REVEAL),
        ("gear", &SETTINGS_GEAR),
    ];
    let mut page = col()
        .w_full()
        .h_full()
        .fill(k.c.page)
        .px(24.0)
        .py(24.0)
        .gap(4.0);
    for (name, icon) in icons {
        page = page.child(line(k, name, icon));
    }
    if let Some(entry) = entry {
        page = page
            .child(el().h(12.0))
            .child(game_hero(k, entry, 320.0, 180.0));
    }
    page
}
