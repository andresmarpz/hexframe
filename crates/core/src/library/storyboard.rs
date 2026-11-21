use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

pub const STORYBOARD_FILE: &str = "storyboard.json";
pub const STORYBOARD_VERSION: u32 = 1;
pub const TILE_WIDTH: u32 = 160;
pub const TILE_HEIGHT: u32 = 90;
pub const COLUMNS: u32 = 10;
pub const ROWS: u32 = 10;

const SHEET_STEM: &str = "storyboard-";
const TARGET_FRAMES: f64 = 500.0;
const MAX_INTERVAL_SECS: u32 = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Storyboard {
    pub version: u32,
    pub interval_secs: u32,
    pub frames: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub columns: u32,
    pub rows: u32,
    pub sheets: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub sheet: usize,
    pub x: u32,
    pub y: u32,
}

pub fn interval_for(duration_secs: f64) -> u32 {
    let wanted = (duration_secs.max(0.0) / TARGET_FRAMES).ceil();
    (wanted as u32).clamp(1, MAX_INTERVAL_SECS)
}

pub fn sheet_file(index: usize) -> String {
    format!("{SHEET_STEM}{index}.jpg")
}

impl Storyboard {
    pub fn plan(duration_secs: f64) -> Option<Self> {
        if !duration_secs.is_finite() || duration_secs < 1.0 {
            return None;
        }
        let interval_secs = interval_for(duration_secs);
        let frames = (duration_secs / f64::from(interval_secs)).floor() as u32 + 1;
        let per_sheet = COLUMNS * ROWS;
        let sheets = (0..frames.div_ceil(per_sheet) as usize)
            .map(sheet_file)
            .collect();
        Some(Self {
            version: STORYBOARD_VERSION,
            interval_secs,
            frames,
            tile_width: TILE_WIDTH,
            tile_height: TILE_HEIGHT,
            columns: COLUMNS,
            rows: ROWS,
            sheets,
        })
    }

    pub fn times(&self) -> impl Iterator<Item = f64> + use<> {
        let interval = f64::from(self.interval_secs);
        (0..self.frames).map(move |frame| f64::from(frame) * interval)
    }

    pub fn tile_at(&self, secs: f64) -> Option<Tile> {
        if self.frames == 0 || !self.is_consistent() {
            return None;
        }
        let nearest = (secs.max(0.0) / f64::from(self.interval_secs)).round() as u32;
        let frame = nearest.min(self.frames - 1);
        let per_sheet = self.columns * self.rows;
        let slot = frame % per_sheet;
        Some(Tile {
            sheet: (frame / per_sheet) as usize,
            x: slot % self.columns * self.tile_width,
            y: slot / self.columns * self.tile_height,
        })
    }

    pub fn sheet_size(&self) -> (u32, u32) {
        (self.columns * self.tile_width, self.rows * self.tile_height)
    }

    fn is_consistent(&self) -> bool {
        let per_sheet = u64::from(self.columns) * u64::from(self.rows);
        self.version == STORYBOARD_VERSION
            && self.interval_secs > 0
            && self.frames > 0
            && self.tile_width > 0
            && self.tile_height > 0
            && per_sheet > 0
            && self.sheets.len() as u64 == u64::from(self.frames).div_ceil(per_sheet)
            && self.sheets.iter().all(|name| plain_name(name))
    }
}

fn plain_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\']) && name != ".." && name != "."
}

pub fn read(folder: &Path) -> Option<Storyboard> {
    let bytes = fs::read(folder.join(STORYBOARD_FILE)).ok()?;
    let board: Storyboard = serde_json::from_slice(&bytes).ok()?;
    (board.is_consistent() && board.sheets.iter().all(|s| folder.join(s).is_file()))
        .then_some(board)
}

pub fn write(folder: &Path, board: &Storyboard) -> io::Result<()> {
    let tmp = folder.join(format!("{STORYBOARD_FILE}.tmp"));
    fs::write(&tmp, serde_json::to_vec(board).map_err(io::Error::other)?)?;
    fs::rename(&tmp, folder.join(STORYBOARD_FILE))
}

pub fn files(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name == STORYBOARD_FILE
                || name == format!("{STORYBOARD_FILE}.tmp")
                || (name.starts_with(SHEET_STEM) && name.contains(".jpg"))
        })
        .map(|entry| entry.path())
        .collect()
}

pub fn remove(folder: &Path) {
    for file in files(folder) {
        if let Err(err) = fs::remove_file(&file) {
            tracing::debug!(%err, file = %file.display(), "cannot remove a storyboard file");
        }
    }
}

pub fn bytes(folder: &Path) -> u64 {
    files(folder)
        .iter()
        .filter_map(|file| fs::metadata(file).ok())
        .map(|meta| meta.len())
        .sum()
}
