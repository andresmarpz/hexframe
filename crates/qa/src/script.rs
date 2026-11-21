use serde_json::{Map, Value, json};

use crate::SETTLE_SECS;
use crate::case::Step;
use crate::check::Check;
use crate::element::Locator;

const TYPE_EVERY: f64 = 0.035;
const DRAG_SECS: f64 = 0.3;
const AFTER_CHECK: f64 = 0.05;

#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Key(String),
    At(f32, f32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlannedCheck {
    pub check: Check,
    pub mark: String,
    pub t: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    pub events: Vec<Value>,
    pub checks: Vec<PlannedCheck>,
}

pub fn compile<E>(
    steps: &[Step],
    mut resolve: impl FnMut(&[Value], f64, &Locator) -> Result<(f32, f32), E>,
) -> Result<Plan, E> {
    let mut plan = Plan::default();
    let mut t = SETTLE_SECS;
    let mut target = |events: &[Value], t: f64, locator: &Locator| -> Result<Target, E> {
        match locator.plain_key() {
            Some(key) => Ok(Target::Key(key.to_owned())),
            None => resolve(events, t, locator).map(|(x, y)| Target::At(x, y)),
        }
    };
    for step in steps {
        match step {
            Step::Tap(at) | Step::Menu(at) | Step::Hover(at) => {
                let kind = match step {
                    Step::Tap(_) => "tap",
                    Step::Menu(_) => "secondary-click",
                    _ => "move",
                };
                let at = target(&plan.events, t, at)?;
                plan.events.push(event(t, kind, &at, []));
                t += SETTLE_SECS;
            }
            Step::Type(text) => {
                plan.events
                    .push(json!({"t": t, "type": "type", "text": text, "every": TYPE_EVERY}));
                t += TYPE_EVERY * text.chars().count() as f64 + SETTLE_SECS;
            }
            Step::Press(key) => {
                plan.events.push(json!({
                    "t": t, "type": "key", "name": key.name, "cmd": key.cmd, "shift": key.shift,
                }));
                t += SETTLE_SECS;
            }
            Step::Drag { at, dx, dy } => {
                let at = target(&plan.events, t, at)?;
                let extra = [
                    ("dx", json!(dx)),
                    ("dy", json!(dy)),
                    ("dur", json!(DRAG_SECS)),
                ];
                plan.events.push(event(t, "drag", &at, extra));
                t += DRAG_SECS + SETTLE_SECS;
            }
            Step::Wheel { at, dy } => {
                let at = target(&plan.events, t, at)?;
                plan.events
                    .push(event(t, "wheel", &at, [("dy", json!(dy))]));
                t += SETTLE_SECS;
            }
            Step::Wait(secs) => t += secs,
            Step::Check(check) => {
                let mark = format!("check-{}", plan.checks.len());
                plan.events
                    .push(json!({"t": t, "type": "mark", "name": mark}));
                plan.checks.push(PlannedCheck {
                    check: check.clone(),
                    mark,
                    t,
                });
                t += AFTER_CHECK;
            }
        }
    }
    Ok(plan)
}

fn event<const N: usize>(t: f64, kind: &str, at: &Target, extra: [(&str, Value); N]) -> Value {
    let mut map = Map::new();
    map.insert("t".into(), json!(t));
    map.insert("type".into(), json!(kind));
    match at {
        Target::Key(key) => {
            map.insert("key".into(), json!(key));
        }
        Target::At(x, y) => {
            map.insert("x".into(), json!(x));
            map.insert("y".into(), json!(y));
        }
    }
    for (k, v) in extra {
        map.insert(k.into(), v);
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests;
