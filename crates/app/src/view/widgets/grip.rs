use rune::{Color, Cursor, Drag, El, Phase, Tx, el};
use rune_kit::Kit;

use crate::ui::{PaneEdge, Ui};

const GRIP_WIDTH: f32 = 6.0;
const STEP: f64 = 8.0;

pub(crate) fn grip(s: &Ui, k: &Kit, edge: PaneEdge, key: &'static str, label: &'static str) -> El {
    let range = edge.range();
    el().key(key)
        .role("separator")
        .label(label)
        .range_value(
            f64::from(s.pane_width(edge)),
            f64::from(range.min),
            f64::from(range.max),
        )
        .range_step(STEP)
        .w(GRIP_WIDTH)
        .pin(None, Some(0.0), Some(0.0), Some(0.0))
        .center()
        .hover_group()
        .cursor(Cursor::ResizeColumn)
        .fill(Color::TRANSPARENT)
        .on_drag(
            move |ui: &mut Ui, drag: &mut Drag, tx: &mut Tx| match drag.phase {
                Phase::Down => ui.press_pane_edge(edge),
                Phase::Move => {
                    tx.instant();
                    ui.drag_pane_edge(edge, drag.dx);
                }
                Phase::Up => ui.release_pane_edge(drag.cancelled()),
            },
        )
        .on_double_press(move |ui: &mut Ui| ui.reset_pane_edge(edge))
        .on_step(move |ui: &mut Ui, steps, _| ui.step_pane_edge(edge, steps))
        .focusable(false)
        .child(
            el().w(2.0)
                .h_full()
                .fill(k.c.border_strong)
                .group_opacity(0.0, 0.7)
                .a11y_hidden(),
        )
}
