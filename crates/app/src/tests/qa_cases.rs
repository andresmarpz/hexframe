use std::path::Path;

use greyscreen_qa::{Case, Element, KeyPress, Locator, SETTLE_SECS, Step};
use rune::{Key, KeyEvent, Mods, Runtime};

use super::runtime_sized;
use crate::ui::{Ui, scene_list};

const FRAME: f64 = 1.0 / 60.0;

fn cases() -> Vec<Case> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../qa/cases");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .expect("qa/cases")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|p| Case::load(p).unwrap_or_else(|e| panic!("{e}")))
        .collect()
}

fn elements(rt: &Runtime<Ui>) -> Vec<Element> {
    rt.elements()
        .into_iter()
        .map(|e| Element {
            path: e.path,
            key: e.key,
            label: e.label,
            text: e.text,
            rect: e.rect,
            opacity: e.opacity,
            focused: e.focused,
            exiting: e.exiting,
        })
        .collect()
}

fn run_for(rt: &mut Runtime<Ui>, secs: f64) {
    for _ in 0..(secs / FRAME).round() as usize {
        rt.frame(FRAME);
    }
}

fn at(rt: &Runtime<Ui>, locator: &Locator) -> Result<(f32, f32), String> {
    locator
        .find(&elements(rt))
        .map(Element::centre)
        .ok_or_else(|| format!("nothing shown matches {}", locator.source()))
}

fn key_event(press: &KeyPress) -> KeyEvent {
    let mods = Mods {
        cmd: press.cmd,
        shift: press.shift,
        ..Mods::default()
    };
    let key = match press.name.as_str() {
        "Escape" => Key::Escape,
        "Enter" => Key::Enter,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "ArrowLeft" => Key::Left,
        "ArrowRight" => Key::Right,
        one if one.chars().count() == 1 && press.cmd => {
            Key::Command(one.chars().next().unwrap_or(' ').to_ascii_lowercase())
        }
        one if one.chars().count() == 1 => Key::Text(one.to_owned()),
        other => panic!("the in-process replay has no key {other:?}; add it here"),
    };
    KeyEvent { key, mods }
}

fn replay(case: &Case) -> Result<Vec<String>, String> {
    let mut ui = Ui::preview();
    if let Some(scene) = &case.scene {
        let (_, apply) = scene_list()
            .iter()
            .find(|(name, _)| name == scene)
            .ok_or_else(|| format!("no scene {scene}"))?;
        apply(&mut ui);
    }
    let mut rt = runtime_sized(ui, case.size);
    rt.on_key = Some(Ui::on_key);
    run_for(&mut rt, SETTLE_SECS);
    let mut failures = Vec::new();
    for step in &case.steps {
        match step {
            Step::Tap(target) => {
                let (x, y) = at(&rt, target)?;
                rt.pointer_move(x, y);
                rt.pointer_down(x, y);
                rt.pointer_up(x, y);
            }
            Step::Menu(target) => {
                let (x, y) = at(&rt, target)?;
                rt.pointer_move(x, y);
                rt.secondary_down(x, y);
                rt.secondary_up(x, y);
            }
            Step::Hover(target) => {
                let (x, y) = at(&rt, target)?;
                rt.pointer_move(x, y);
            }
            Step::Type(text) => {
                for c in text.chars() {
                    rt.key(KeyEvent {
                        key: Key::Text(c.to_string()),
                        mods: Mods::default(),
                    });
                    rt.frame(FRAME);
                }
            }
            Step::Press(press) => rt.key(key_event(press)),
            Step::Drag { at: target, dx, dy } => {
                let (x, y) = at(&rt, target)?;
                rt.pointer_move(x, y);
                rt.pointer_down(x, y);
                for s in 1..=10 {
                    let f = s as f32 / 10.0;
                    rt.pointer_move(x + dx * f, y + dy * f);
                    rt.frame(FRAME);
                }
                rt.pointer_up(x + dx, y + dy);
            }
            Step::Wheel { at: target, dy } => {
                let (x, y) = at(&rt, target)?;
                rt.wheel(x, y, *dy);
            }
            Step::Wait(secs) => run_for(&mut rt, *secs),
            Step::Check(check) => {
                for failure in check.failures(&elements(&rt)) {
                    failures.push(format!("{}: {failure}", check.name));
                }
                continue;
            }
        }
        run_for(&mut rt, SETTLE_SECS);
    }
    Ok(failures)
}

#[test]
fn every_qa_case_passes_in_process() {
    let mut report = Vec::new();
    for case in cases().iter().filter(|c| !c.library) {
        match replay(case) {
            Ok(failures) => {
                report.extend(failures.into_iter().map(|f| format!("{}: {f}", case.name)))
            }
            Err(err) => report.push(format!("{}: {err}", case.name)),
        }
    }
    assert!(report.is_empty(), "\n{}", report.join("\n"));
}

#[test]
fn the_qa_clock_matches_the_runner() {
    assert_eq!(crate::qa::FIXED_NOW, greyscreen_qa::FIXED_NOW);
}
