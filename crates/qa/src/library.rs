use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use greyscreen_qa::FIXED_NOW;

use crate::run::Sandbox;

const DEMO_GAMES: &str = "6";

pub(crate) fn prepare(
    out: &Path,
    source: Option<&Path>,
    cli: &Path,
    sandbox: &Sandbox,
) -> Result<PathBuf> {
    let dest = out.join("library");
    if dest.exists() {
        fs::remove_dir_all(&dest).with_context(|| format!("cannot clear {}", dest.display()))?;
    }
    fs::create_dir_all(&dest)?;
    match source {
        Some(source) => {
            let source = source
                .canonicalize()
                .with_context(|| format!("no library at {}", source.display()))?;
            if dest.canonicalize()?.starts_with(&source) {
                bail!("--out must be outside the library");
            }
            copy_shallow(&source, &dest)?;
        }
        None => {
            let mut cmd = sandbox.command(cli);
            cmd.args([
                "demo", "seed", "--games", DEMO_GAMES, "--now", FIXED_NOW, "--dir",
            ])
            .arg(&dest);
            sandbox.exec(cmd).context("cannot seed the demo library")?;
        }
    }
    Ok(dest)
}

// Headless runs never decode video, so an empty file stands in for each one.
fn copy_shallow(from: &Path, to: &Path) -> Result<()> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            fs::create_dir_all(&target)?;
            copy_shallow(&entry.path(), &target)?;
        } else if kind.is_file() {
            if is_video(&entry.path()) {
                fs::File::create(&target)?;
            } else {
                fs::copy(entry.path(), &target)?;
            }
        }
    }
    Ok(())
}

fn is_video(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
}

#[cfg(test)]
mod tests;
