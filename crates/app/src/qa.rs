use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use jiff::Zoned;

pub(crate) const QA_FLAG: &str = "--qa";
pub(crate) const LIBRARY_FLAG: &str = "--qa-library";
pub(crate) const FIXED_NOW: &str = "2026-11-21T15:00:00+00:00[UTC]";

#[derive(Debug, Default, PartialEq)]
pub(crate) struct Mode {
    pub active: bool,
    pub library: Option<PathBuf>,
}

pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Mode {
    let mut mode = Mode::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == QA_FLAG {
            mode.active = true;
        } else if arg == LIBRARY_FLAG {
            mode.active = true;
            mode.library = args.next().map(PathBuf::from);
        }
    }
    mode
}

fn mode() -> &'static Mode {
    static MODE: OnceLock<Mode> = OnceLock::new();
    MODE.get_or_init(|| {
        if rune::is_headless() {
            parse(std::env::args().skip(1))
        } else {
            Mode::default()
        }
    })
}

pub(crate) fn active() -> bool {
    mode().active
}

pub(crate) fn library() -> Option<&'static Path> {
    mode().library.as_deref()
}

pub(crate) fn now() -> Zoned {
    if active() {
        FIXED_NOW.parse().unwrap_or_else(|_| Zoned::now())
    } else {
        Zoned::now()
    }
}

pub(crate) fn reveal(path: &Path) {
    if !rune::is_headless() && !cfg!(test) {
        greyscreen_engine::reveal(path);
    }
}

pub(crate) fn copy_file(path: &Path) -> Result<(), String> {
    if rune::is_headless() || cfg!(test) {
        return Ok(());
    }
    greyscreen_engine::copy_file(path)
}

#[cfg(test)]
mod tests;
