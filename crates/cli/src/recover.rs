use std::path::Path;

use anyhow::{Context, Result};

pub(crate) fn run(path: &Path) -> Result<()> {
    let recovered = greyscreen_mp4::recover::recover(path)
        .with_context(|| format!("recovering {}", path.display()))?;
    println!("recovered: {}", recovered.display());
    Ok(())
}
