use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Snap {
    Matched { ratio: f64 },
    Changed { ratio: f64 },
    Resized,
    New,
    Added,
    Updated,
    Unchecked(&'static str),
}

impl Snap {
    pub(crate) fn failed(&self) -> bool {
        matches!(self, Snap::Changed { .. } | Snap::Resized)
    }

    pub(crate) fn is_new(&self) -> bool {
        matches!(self, Snap::New | Snap::Added)
    }

    pub(crate) fn word(&self) -> String {
        match self {
            Snap::Matched { ratio } => format!("matches ({:.3}% off)", ratio * 100.0),
            Snap::Changed { ratio } => format!("changed ({:.3}% of pixels)", ratio * 100.0),
            Snap::Resized => "size changed".into(),
            Snap::New => "new, no golden yet".into(),
            Snap::Added => "golden added".into(),
            Snap::Updated => "golden updated".into(),
            Snap::Unchecked(why) => format!("not compared: {why}"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Shot {
    pub(crate) actual: PathBuf,
    pub(crate) golden: Option<PathBuf>,
    pub(crate) diff: Option<PathBuf>,
    pub(crate) snap: Snap,
}

#[derive(Debug, Clone)]
pub(crate) struct CheckOutcome {
    pub(crate) name: String,
    pub(crate) failures: Vec<String>,
    pub(crate) shot: Option<Shot>,
}

impl CheckOutcome {
    pub(crate) fn failed(&self) -> bool {
        !self.failures.is_empty() || self.shot.as_ref().is_some_and(|s| s.snap.failed())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CaseOutcome {
    pub(crate) name: String,
    pub(crate) about: String,
    pub(crate) error: Option<String>,
    pub(crate) checks: Vec<CheckOutcome>,
}

impl CaseOutcome {
    pub(crate) fn failed(&self) -> bool {
        self.error.is_some() || self.checks.iter().any(CheckOutcome::failed)
    }
}
