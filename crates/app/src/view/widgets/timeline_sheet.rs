use std::sync::atomic::{AtomicBool, Ordering};

use greyscreen_core::timeline::{BoxKind, MinuteBox, Multikill, ObjectiveKind, Side};
use rune::{El, col};
use rune_kit::Kit;

use super::minute_timeline::minute_timeline;

const CELL: f32 = 34.0;
const GAP: f32 = 2.0;

static ON: AtomicBool = AtomicBool::new(false);

pub(crate) fn show() {
    ON.store(true, Ordering::Relaxed);
}

pub(crate) fn shown() -> bool {
    ON.load(Ordering::Relaxed)
}

fn sample(minute: u32, kind: BoxKind, event: &str) -> MinuteBox {
    MinuteBox {
        minute,
        kind,
        events: vec![event.to_owned()],
    }
}

fn kill(minute: u32, multikill: Option<Multikill>, event: &str) -> MinuteBox {
    let kills = multikill.map_or(1, |m| u32::from(m.streak()));
    sample(minute, BoxKind::Kill { kills, multikill }, event)
}

fn samples() -> Vec<MinuteBox> {
    vec![
        sample(0, BoxKind::Empty, ""),
        kill(1, None, "Kill"),
        kill(2, Some(Multikill::Double), "Double Kill"),
        kill(3, Some(Multikill::Triple), "Triple Kill"),
        kill(4, Some(Multikill::Quadra), "Quadra Kill"),
        kill(5, Some(Multikill::Penta), "Penta Kill"),
        sample(6, BoxKind::Death, "Death"),
        sample(
            7,
            BoxKind::Objective {
                kind: ObjectiveKind::Dragon,
                side: Side::Mine,
            },
            "Dragon",
        ),
        sample(
            8,
            BoxKind::Objective {
                kind: ObjectiveKind::Baron,
                side: Side::Enemy,
            },
            "Enemy Baron",
        ),
    ]
}

pub(crate) fn sheet(k: &Kit) -> El {
    let boxes = samples();
    let width = CELL * boxes.len() as f32 + GAP * (boxes.len() - 1) as f32;
    col()
        .w_full()
        .h_full()
        .fill(k.c.page)
        .padding(32.0)
        .child(minute_timeline(k, &boxes, width))
}
