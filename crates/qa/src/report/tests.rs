use std::path::PathBuf;

use super::*;
use crate::outcome::{CheckOutcome, Snap};

fn case(snaps: Vec<Snap>) -> CaseOutcome {
    CaseOutcome {
        name: "case".into(),
        about: String::new(),
        error: None,
        checks: snaps
            .into_iter()
            .map(|snap| CheckOutcome {
                name: "check".into(),
                failures: Vec::new(),
                shot: Some(Shot {
                    actual: PathBuf::new(),
                    golden: None,
                    diff: None,
                    snap,
                }),
            })
            .collect(),
    }
}

#[test]
fn goldens_written_by_this_run_count_as_new() {
    let outcomes = [case(vec![
        Snap::New,
        Snap::Added,
        Snap::Updated,
        Snap::Matched { ratio: 0.0 },
    ])];
    assert_eq!(totals(&outcomes).new, 2);
}
