use std::path::Path;
use std::time::Instant;

use anyhow::{Result, anyhow};

pub(crate) fn run(folder: &Path) -> Result<()> {
    let started = Instant::now();
    let frames = greyscreen_engine::build_storyboard(folder).map_err(|err| anyhow!(err))?;
    println!(
        "{frames} frames in {:.1} s: {}",
        started.elapsed().as_secs_f64(),
        folder.display()
    );
    Ok(())
}
