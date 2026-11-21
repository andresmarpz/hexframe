use super::*;

fn el(key: &str, text: Option<&str>, focused: bool) -> Element {
    Element {
        path: format!("root/{key}"),
        key: Some(key.into()),
        text: text.map(str::to_owned),
        rect: [0.0, 0.0, 10.0, 10.0],
        opacity: 1.0,
        focused,
        ..Element::default()
    }
}

fn check() -> Check {
    Check {
        name: "c".into(),
        snap: false,
        has: vec![Locator::parse("dialog")],
        lacks: vec![Locator::parse("menu")],
        text: vec!["Victory".into()],
        focused: Some(Locator::parse("search")),
    }
}

#[test]
fn a_matching_tree_passes() {
    let els = [
        el("dialog", None, false),
        el("title", Some("Victory"), false),
        el("search", None, true),
    ];
    assert!(check().failures(&els).is_empty());
}

#[test]
fn each_broken_expectation_is_named() {
    let els = [el("menu", Some("Defeat"), false), el("search", None, false)];
    assert_eq!(
        check().failures(&els),
        vec![
            "missing dialog".to_owned(),
            "should not show menu".to_owned(),
            "no text \"Victory\"".to_owned(),
            "search is not focused".to_owned(),
        ]
    );
}
