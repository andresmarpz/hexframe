use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use greyscreen_qa::Case;
use serde_json::Value;

const RUN_TIMEOUT: Duration = Duration::from_secs(120);
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone)]
pub(crate) struct Sandbox {
    pub(crate) tmp: PathBuf,
}

impl Sandbox {
    pub(crate) fn command(&self, exe: &Path) -> Command {
        let mut cmd = Command::new(exe);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .env("TMPDIR", &self.tmp)
            .env("TMP", &self.tmp)
            .env("TEMP", &self.tmp)
            .env_remove("RUST_LOG");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd
    }

    pub(crate) fn exec(&self, mut cmd: Command) -> Result<()> {
        let mut child = cmd
            .spawn()
            .with_context(|| format!("cannot start {:?}", cmd.get_program()))?;
        let pipe = child.stderr.take();
        let reader = std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut pipe) = pipe {
                std::io::Read::read_to_string(&mut pipe, &mut text).ok();
            }
            text
        });
        let deadline = Instant::now() + RUN_TIMEOUT;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() > deadline {
                child.kill().ok();
                child.wait().ok();
                bail!("timed out after {} s", RUN_TIMEOUT.as_secs());
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let stderr = reader.join().unwrap_or_default();
        if !status.success() {
            let tail: Vec<&str> = stderr.lines().rev().take(6).collect();
            let tail: Vec<&str> = tail.into_iter().rev().collect();
            bail!("exit {status}: {}", tail.join(" | "));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AppRun<'a> {
    pub(crate) exe: &'a Path,
    pub(crate) sandbox: &'a Sandbox,
    pub(crate) library: Option<&'a Path>,
    pub(crate) dir: PathBuf,
}

impl AppRun<'_> {
    fn command(&self, case: &Case) -> Command {
        let mut cmd = self.sandbox.command(self.exe);
        let (w, h) = case.size;
        cmd.current_dir(&self.dir).arg("--qa").args([
            "--size",
            &format!("{w}x{h}"),
            "--scale",
            "1",
            "--theme",
            &case.theme,
        ]);
        if let Some(scene) = &case.scene {
            cmd.args(["--scene", scene]);
        }
        if case.library
            && let Some(library) = self.library
        {
            cmd.arg("--qa-library").arg(library);
        }
        cmd
    }

    fn script(&self, events: &[Value], name: &str) -> Result<Option<PathBuf>> {
        if events.is_empty() {
            return Ok(None);
        }
        let path = self.dir.join(format!("{name}.script.json"));
        std::fs::write(&path, serde_json::to_vec_pretty(events)?)?;
        Ok(Some(path))
    }

    pub(crate) fn tree_at(
        &self,
        case: &Case,
        events: &[Value],
        t: f64,
        name: &str,
    ) -> Result<Value> {
        let tree = self.dir.join(format!("{name}.tree.json"));
        let mut cmd = self.command(case);
        if let Some(script) = self.script(events, name)? {
            cmd.arg("--script").arg(script);
        }
        cmd.args(["--at", &t.to_string()]).arg("--tree").arg(&tree);
        self.sandbox.exec(cmd)?;
        read_json(&tree)
    }

    pub(crate) fn shoot(
        &self,
        case: &Case,
        events: &[Value],
        mark: &str,
        snap: Option<&Path>,
    ) -> Result<Value> {
        let tree = self.dir.join(format!("{mark}.tree.json"));
        let mut cmd = self.command(case);
        if let Some(script) = self.script(events, "case")? {
            cmd.arg("--script").arg(script);
        }
        cmd.args(["--at", &format!("mark:{mark}")])
            .arg("--tree")
            .arg(&tree);
        if let Some(snap) = snap {
            cmd.arg("--snap").arg(snap);
        }
        self.sandbox.exec(cmd)?;
        read_json(&tree)
    }
}

fn read_json(path: &Path) -> Result<Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("no tree at {}", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}
