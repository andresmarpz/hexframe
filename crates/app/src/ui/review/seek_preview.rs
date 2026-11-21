use std::path::PathBuf;

use greyscreen_core::library::storyboard::{self, Tile};

use super::{ReviewState, Source};
use crate::ui::Ui;

impl ReviewState {
    pub(crate) fn load_storyboard(&mut self) {
        self.storyboard = storyboard::read(&self.entry.folder);
    }

    pub(crate) fn wants_storyboard(&self) -> bool {
        self.storyboard.is_none() && self.player.is_some() && !self.entry.meta.video.is_empty()
    }

    pub(crate) fn hover_seek(&mut self, secs: Option<f64>) {
        let duration = self.duration().max(0.0);
        self.seek_hover = secs.map(|secs| secs.clamp(0.0, duration));
    }

    pub(crate) fn seek_preview(&self) -> Option<f64> {
        if self.duration() <= 0.0 {
            return None;
        }
        self.scrub.map(|scrub| scrub.target).or(self.seek_hover)
    }

    pub(crate) fn preview_tile(&self, secs: f64) -> Option<(PathBuf, Tile)> {
        if self.source != Source::Game {
            return None;
        }
        let board = self.storyboard.as_ref()?;
        let tile = board.tile_at(secs)?;
        Some((self.entry.folder.join(board.sheets.get(tile.sheet)?), tile))
    }
}

impl Ui {
    pub(crate) fn review_seek_hover(&mut self, window_x: Option<f32>) {
        if let Some(review) = &mut self.review {
            let [left, _, width, _] = review.timeline_rect;
            let secs = window_x
                .filter(|_| width > 0.0)
                .map(|x| f64::from(((x - left) / width).clamp(0.0, 1.0)) * review.duration());
            review.hover_seek(secs);
        }
    }

    pub(crate) fn storyboard_ready(&mut self, folder: &std::path::Path) {
        if let Some(review) = &mut self.review
            && review.entry.folder == folder
        {
            review.load_storyboard();
            if let Some(board) = &review.storyboard {
                for sheet in &board.sheets {
                    crate::view::thumb::invalidate(&folder.join(sheet));
                }
            }
        }
    }
}
