use std::path::Path;

use serde_json::Value;

use crate::check::Check;
use crate::element::Locator;

#[derive(Debug, thiserror::Error)]
pub enum CaseError {
    #[error("{path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("{path}: not JSON: {source}")]
    Json {
        path: String,
        source: serde_json::Error,
    },
    #[error("{case}: step {index}: {message}")]
    Step {
        case: String,
        index: usize,
        message: String,
    },
    #[error("{case}: {message}")]
    Case { case: String, message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyPress {
    pub name: String,
    pub cmd: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Tap(Locator),
    Menu(Locator),
    Hover(Locator),
    Type(String),
    Press(KeyPress),
    Drag { at: Locator, dx: f32, dy: f32 },
    Wheel { at: Locator, dy: f32 },
    Wait(f64),
    Check(Check),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub name: String,
    pub about: String,
    pub scene: Option<String>,
    pub size: (f32, f32),
    pub theme: String,
    pub library: bool,
    pub golden: bool,
    pub steps: Vec<Step>,
}

impl Case {
    pub fn load(path: &Path) -> Result<Case, CaseError> {
        let shown = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|source| CaseError::Read {
            path: shown.clone(),
            source,
        })?;
        let json: Value = serde_json::from_str(&text).map_err(|source| CaseError::Json {
            path: shown,
            source,
        })?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Case::from_json(name, &json)
    }

    pub fn from_json(name: String, json: &Value) -> Result<Case, CaseError> {
        let fail = |message: &str| CaseError::Case {
            case: name.clone(),
            message: message.to_owned(),
        };
        let size = match json["size"].as_array().map(Vec::as_slice) {
            None => (1280.0, 720.0),
            Some([w, h]) => match (w.as_f64(), h.as_f64()) {
                (Some(w), Some(h)) if w > 0.0 && h > 0.0 => (w as f32, h as f32),
                _ => return Err(fail("size is [width, height] above 0")),
            },
            Some(_) => return Err(fail("size is [width, height]")),
        };
        let steps = json["steps"]
            .as_array()
            .ok_or_else(|| fail("needs \"steps\": [...]"))?
            .iter()
            .enumerate()
            .map(|(index, step)| {
                parse_step(step).map_err(|message| CaseError::Step {
                    case: name.clone(),
                    index,
                    message,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !steps.iter().any(|s| matches!(s, Step::Check(_))) {
            return Err(fail("needs at least one check step"));
        }
        Ok(Case {
            about: json["about"].as_str().unwrap_or_default().to_owned(),
            scene: json["scene"].as_str().map(str::to_owned),
            size,
            theme: json["theme"].as_str().unwrap_or("dark").to_owned(),
            library: json["library"].as_bool().unwrap_or(false),
            golden: json["golden"].as_bool().unwrap_or(true),
            steps,
            name,
        })
    }

    pub fn checks(&self) -> impl Iterator<Item = &Check> {
        self.steps.iter().filter_map(|s| match s {
            Step::Check(check) => Some(check),
            _ => None,
        })
    }
}

fn locator(step: &Value, field: &str) -> Result<Locator, String> {
    step[field]
        .as_str()
        .map(Locator::parse)
        .ok_or_else(|| format!("\"{field}\" names an element"))
}

fn number(step: &Value, field: &str) -> f32 {
    step[field].as_f64().unwrap_or(0.0) as f32
}

fn locators(step: &Value, field: &str) -> Result<Vec<Locator>, String> {
    strings(step, field).map(|list| list.iter().map(|s| Locator::parse(s)).collect())
}

fn strings(step: &Value, field: &str) -> Result<Vec<String>, String> {
    match &step[field] {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => items
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("\"{field}\" holds strings"))
            })
            .collect(),
        _ => Err(format!("\"{field}\" is a list of strings")),
    }
}

fn parse_step(step: &Value) -> Result<Step, String> {
    if step.get("tap").is_some() {
        return Ok(Step::Tap(locator(step, "tap")?));
    }
    if step.get("menu").is_some() {
        return Ok(Step::Menu(locator(step, "menu")?));
    }
    if step.get("hover").is_some() {
        return Ok(Step::Hover(locator(step, "hover")?));
    }
    if let Some(text) = step.get("type") {
        let text = text.as_str().ok_or("\"type\" is the text to type")?;
        return Ok(Step::Type(text.to_owned()));
    }
    if let Some(name) = step.get("press") {
        let name = name
            .as_str()
            .ok_or("\"press\" is a key name like \"Escape\"")?;
        return Ok(Step::Press(KeyPress {
            name: name.to_owned(),
            cmd: step["cmd"].as_bool().unwrap_or(false),
            shift: step["shift"].as_bool().unwrap_or(false),
        }));
    }
    if step.get("drag").is_some() {
        return Ok(Step::Drag {
            at: locator(step, "drag")?,
            dx: number(step, "dx"),
            dy: number(step, "dy"),
        });
    }
    if step.get("wheel").is_some() {
        return Ok(Step::Wheel {
            at: locator(step, "wheel")?,
            dy: number(step, "dy"),
        });
    }
    if let Some(secs) = step.get("wait") {
        let secs = secs
            .as_f64()
            .filter(|s| *s >= 0.0)
            .ok_or("\"wait\" is seconds")?;
        return Ok(Step::Wait(secs));
    }
    if let Some(name) = step.get("check") {
        let name = name
            .as_str()
            .filter(|n| !n.is_empty())
            .ok_or("\"check\" is a name")?;
        return Ok(Step::Check(Check {
            name: name.to_owned(),
            snap: step["snap"].as_bool().unwrap_or(true),
            has: locators(step, "has")?,
            lacks: locators(step, "lacks")?,
            text: strings(step, "text")?,
            focused: step["focused"].as_str().map(Locator::parse),
        }));
    }
    Err("unknown step: use tap, menu, hover, type, press, drag, wheel, wait or check".into())
}

#[cfg(test)]
mod tests;
