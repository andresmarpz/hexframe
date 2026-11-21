use std::path::Path;

use anyhow::{Context, Result};

pub(crate) fn run(video: &Path, at: f64, out: &Path) -> Result<()> {
    greyscreen_thumbs::write_thumbnail(video, at, out)
        .with_context(|| format!("thumbnail of {}", video.display()))?;
    println!("{}", out.display());
    Ok(())
}
