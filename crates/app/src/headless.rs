use std::path::Path;

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
