use std::convert::Infallible;

use serde_json::json;

use super::*;
use crate::case::{Case, KeyPress};

fn check(name: &str) -> Step {
    Step::Check(Check {
        name: name.into(),
        snap: true,
        has: Vec::new(),
        lacks: Vec::new(),
        text: Vec::new(),
        focused: None,
    })
}

#[test]
fn plain_keys_go_to_rune_and_other_locators_are_resolved_where_the_step_runs() {
    let steps = vec![
        Step::Tap(Locator::parse("nav-games")),
        Step::Tap(Locator::parse("label:Lux")),
        check("opened"),
    ];
    let mut asked = Vec::new();
    let plan = compile(&steps, |events, t, locator| {
        asked.push((events.len(), t, locator.source().to_owned()));
        Ok::<_, Infallible>((12.0, 34.0))
    })
    .expect("plan");
    assert_eq!(asked, vec![(1, 1.0, "label:Lux".to_owned())]);
    assert_eq!(
        plan.events[0],
        json!({"t": 0.5, "type": "tap", "key": "nav-games"})
    );
    assert_eq!(
        plan.events[1],
        json!({"t": 1.0, "type": "tap", "x": 12.0, "y": 34.0})
    );
    assert_eq!(
        plan.events[2],
        json!({"t": 1.5, "type": "mark", "name": "check-0"})
    );
    assert_eq!(plan.checks[0].mark, "check-0");
}

#[test]
fn typing_waits_for_every_character_before_the_next_step() {
    let steps = vec![
        Step::Type("abcd".into()),
        Step::Press(KeyPress {
            name: "Escape".into(),
            cmd: false,
            shift: false,
        }),
        check("done"),
    ];
    let plan = compile(&steps, |_, _, _| Ok::<_, Infallible>((0.0, 0.0))).expect("plan");
    let press_t = plan.events[1]["t"].as_f64().expect("t");
    assert!(
        (press_t - (0.5 + 4.0 * 0.035 + 0.5)).abs() < 1e-9,
        "{press_t}"
    );
}

#[test]
fn resolver_errors_stop_the_plan() {
    let case = Case::from_json(
        "x".into(),
        &json!({"steps": [{"tap": "label:Nope"}, {"check": "c"}]}),
    )
    .expect("case");
    let err = compile(&case.steps, |_, _, l| Err(format!("no {}", l.source()))).unwrap_err();
    assert_eq!(err, "no label:Nope");
}
