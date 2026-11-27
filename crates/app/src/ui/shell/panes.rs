use greyscreen_core::settings::{LIST_WIDTH, NAV_WIDTH, PaneWidth};
use greyscreen_engine::EngineCmd;

use crate::ui::Ui;

pub(crate) const STEP: i32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaneEdge {
    Nav,
    List,
}

impl PaneEdge {
    pub(crate) fn range(self) -> PaneWidth {
        match self {
            PaneEdge::Nav => NAV_WIDTH,
            PaneEdge::List => LIST_WIDTH,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PaneDrag {
    edge: PaneEdge,
    start: u32,
}

impl Ui {
    pub(crate) fn pane_width(&self, edge: PaneEdge) -> u32 {
        match edge {
            PaneEdge::Nav => self.settings.nav_width,
            PaneEdge::List => self.settings.list_width,
        }
    }

    pub(crate) fn press_pane_edge(&mut self, edge: PaneEdge) {
        self.pane_drag = Some(PaneDrag {
            edge,
            start: self.pane_width(edge),
        });
    }

    pub(crate) fn drag_pane_edge(&mut self, edge: PaneEdge, travel: f32) {
        let Some(drag) = self.pane_drag.filter(|drag| drag.edge == edge) else {
            return;
        };
        let width = (drag.start as f32 + travel).round().max(0.0) as u32;
        self.set_pane_width(edge, width);
    }

    pub(crate) fn release_pane_edge(&mut self, cancelled: bool) {
        let Some(drag) = self.pane_drag.take() else {
            return;
        };
        if cancelled {
            self.set_pane_width(drag.edge, drag.start);
        } else {
            self.send(EngineCmd::ApplySettings(self.settings.clone()));
        }
    }

    pub(crate) fn reset_pane_edge(&mut self, edge: PaneEdge) {
        self.pane_drag = None;
        self.change(|s| set(s, edge, edge.range().default));
    }

    pub(crate) fn step_pane_edge(&mut self, edge: PaneEdge, steps: i32) {
        let width = self.pane_width(edge).saturating_add_signed(steps * STEP);
        self.change(|s| set(s, edge, width));
    }

    fn set_pane_width(&mut self, edge: PaneEdge, width: u32) {
        set(&mut self.settings, edge, edge.range().clamp(width));
    }
}

fn set(settings: &mut greyscreen_core::settings::Settings, edge: PaneEdge, width: u32) {
    match edge {
        PaneEdge::Nav => settings.nav_width = width,
        PaneEdge::List => settings.list_width = width,
    }
}
