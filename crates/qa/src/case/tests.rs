use serde_json::json;

use super::*;

#[test]
fn a_case_reads_its_steps_and_defaults() {
    let case = Case::from_json(
        "open".into(),
        &json!({
            "scene": "games-pick",
            "steps": [
                {"tap": "games-list >> label:Lux"},
                {"type": "ahri"},
                {"press": "k", "cmd": true},
                {"drag": "timeline", "dx": 40},
                {"wait": 0.25},
                {"check": "opened", "has": ["review-player"], "lacks": ["games-pick"], "text": ["Victory"], "snap": false}
            ]
        }),
    )
    .expect("case");
    assert_eq!(case.size, (1280.0, 720.0));
    assert_eq!(case.theme, "dark");
    assert!(case.golden && !case.library);
    assert_eq!(case.steps.len(), 6);
    assert!(matches!(&case.steps[2], Step::Press(k) if k.cmd && k.name == "k"));
    let check = case.checks().next().expect("check");
    assert_eq!(check.name, "opened");
    assert!(!check.snap);
    assert_eq!(check.has, vec![Locator::parse("review-player")]);
}

#[test]
fn a_case_without_checks_is_rejected() {
    let err = Case::from_json("x".into(), &json!({"steps": [{"tap": "a"}]})).unwrap_err();
    assert!(err.to_string().contains("check"), "{err}");
}

#[test]
fn an_unknown_step_names_its_index() {
    let err = Case::from_json(
        "x".into(),
        &json!({"steps": [{"check": "a"}, {"swipe": "b"}]}),
    )
    .unwrap_err();
    assert!(err.to_string().contains("step 1"), "{err}");
}

#[test]
fn every_checked_in_case_parses() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../qa/cases");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("qa/cases") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "json") {
            Case::load(&path).unwrap_or_else(|e| panic!("{e}"));
            count += 1;
        }
    }
    assert!(count > 0);
}
