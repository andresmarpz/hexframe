use rune::{Color, Cursor, Drag, El, Phase, Tx, el};

use crate::ui::{PaneEdge, Ui};

const GRIP_WIDTH: f32 = 6.0;
const STEP: f64 = 8.0;

pub(super) fn grip(s: &Ui, edge: PaneEdge, key: &'static str, label: &'static str) -> El {
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
        .cursor(Cursor::ResizeColumn)
        .fill(Color::TRANSPARENT)
        .opacity(0.02)
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
        .on_step(move |ui: &mut Ui, steps, _| ui.step_pane_edge(edge, steps as i32))
        .focusable(false)
}
