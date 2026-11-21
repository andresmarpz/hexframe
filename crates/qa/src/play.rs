use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use greyscreen_qa::{Case, Element, Rgba, compare, compile};

use crate::outcome::{CaseOutcome, CheckOutcome, Shot, Snap};
use crate::run::AppRun;

#[derive(Debug, Clone)]
pub(crate) struct Judge {
    pub(crate) goldens: PathBuf,
    pub(crate) update: bool,
    pub(crate) tolerance: u8,
    pub(crate) max_diff: f64,
    pub(crate) custom_library: bool,
}

pub(crate) fn play(case: &Case, app: &AppRun<'_>, judge: &Judge) -> CaseOutcome {
    let (checks, error) = match try_play(case, app, judge) {
        Ok(checks) => (checks, None),
        Err(err) => (Vec::new(), Some(format!("{err:#}"))),
    };
    CaseOutcome {
        name: case.name.clone(),
        about: case.about.clone(),
        error,
        checks,
    }
}

fn try_play(case: &Case, app: &AppRun<'_>, judge: &Judge) -> Result<Vec<CheckOutcome>> {
    fs::create_dir_all(&app.dir)?;
    let mut probes = 0;
    let plan = compile(&case.steps, |events, t, locator| -> Result<(f32, f32)> {
        probes += 1;
        let tree = app.tree_at(case, events, t, &format!("probe-{probes}"))?;
        let elements = Element::from_tree(&tree);
        locator
            .find(&elements)
            .map(Element::centre)
            .ok_or_else(|| anyhow!("at {t:.2} s nothing shown matches {}", locator.source()))
    })?;
    let mut out = Vec::new();
    for planned in &plan.checks {
        let name = &planned.check.name;
        let snap = planned
            .check
            .snap
            .then(|| app.dir.join(format!("{name}.png")));
        let tree = app.shoot(case, &plan.events, &planned.mark, snap.as_deref())?;
        let failures = planned.check.failures(&Element::from_tree(&tree));
        let shot = snap
            .map(|actual| judge.shot(case, name, actual))
            .transpose()?;
        out.push(CheckOutcome {
            name: name.clone(),
            failures,
            shot,
        });
    }
    Ok(out)
}

impl Judge {
    fn shot(&self, case: &Case, check: &str, actual: PathBuf) -> Result<Shot> {
        let unchecked = |why| Shot {
            actual: actual.clone(),
            golden: None,
            diff: None,
            snap: Snap::Unchecked(why),
        };
        if !case.golden {
            return Ok(unchecked("the case keeps no golden"));
        }
        if case.library && self.custom_library {
            return Ok(unchecked("a custom library"));
        }
        let golden = self.goldens.join(&case.name).join(format!("{check}.png"));
        if !golden.is_file() {
            let snap = if self.update {
                adopt(&actual, &golden)?;
                Snap::Updated
            } else {
                Snap::New
            };
            return Ok(Shot {
                actual,
                golden: None,
                diff: None,
                snap,
            });
        }
        let shown_golden = actual.with_extension("golden.png");
        fs::copy(&golden, &shown_golden)?;
        let diff = compare(&Rgba::read(&golden)?, &Rgba::read(&actual)?, self.tolerance);
        let diff_path = match &diff.image {
            Some(image) if diff.changed > 0 => {
                let path = actual.with_extension("diff.png");
                image.write(&path)?;
                Some(path)
            }
            _ => None,
        };
        let snap = if diff.within(self.max_diff) {
            Snap::Matched {
                ratio: diff.ratio(),
            }
        } else if self.update {
            adopt(&actual, &golden)?;
            Snap::Updated
        } else if diff.same_size {
            Snap::Changed {
                ratio: diff.ratio(),
            }
        } else {
            Snap::Resized
        };
        Ok(Shot {
            actual,
            golden: Some(shown_golden),
            diff: diff_path,
            snap,
        })
    }
}

fn adopt(actual: &Path, golden: &Path) -> Result<()> {
    if let Some(parent) = golden.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(actual, golden)?;
    Ok(())
}
