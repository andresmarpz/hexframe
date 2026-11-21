use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Element {
    pub path: String,
    pub key: Option<String>,
    pub label: Option<String>,
    pub text: Option<String>,
    pub rect: [f32; 4],
    pub opacity: f32,
    pub focused: bool,
    pub exiting: bool,
}

impl Element {
    pub fn from_tree(tree: &Value) -> Vec<Element> {
        let Some(list) = tree["elements"].as_array() else {
            return Vec::new();
        };
        list.iter().map(Element::from_json).collect()
    }

    fn from_json(e: &Value) -> Element {
        let string = |k: &str| e[k].as_str().map(str::to_owned);
        let mut rect = [0.0; 4];
        for (slot, v) in rect
            .iter_mut()
            .zip(e["rect"].as_array().into_iter().flatten())
        {
            *slot = v.as_f64().unwrap_or(0.0) as f32;
        }
        Element {
            path: string("path").unwrap_or_default(),
            key: string("key"),
            label: string("label"),
            text: string("text"),
            rect,
            opacity: e["opacity"].as_f64().unwrap_or(1.0) as f32,
            focused: e["focused"].as_bool().unwrap_or(false),
            exiting: e["exiting"].as_bool().unwrap_or(false),
        }
    }

    pub fn centre(&self) -> (f32, f32) {
        let [x, y, w, h] = self.rect;
        (x + w / 2.0, y + h / 2.0)
    }

    pub fn shown(&self) -> bool {
        !self.exiting && self.opacity > 0.01 && self.rect[2] > 0.0 && self.rect[3] > 0.0
    }

    pub fn says(&self, needle: &str) -> bool {
        [&self.text, &self.label]
            .into_iter()
            .flatten()
            .any(|s| s.contains(needle))
    }

    fn within(&self, scope: &str) -> bool {
        self.path.split('/').any(|segment| segment == scope)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Target {
    Key(String),
    Label(String),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Locator {
    source: String,
    scope: Option<String>,
    target: Target,
}

impl Locator {
    pub fn parse(source: &str) -> Locator {
        let (scope, rest) = match source.split_once(">>") {
            Some((scope, rest)) => (Some(scope.trim().to_owned()), rest.trim()),
            None => (None, source.trim()),
        };
        let target = if let Some(label) = rest.strip_prefix("label:") {
            Target::Label(label.to_owned())
        } else if let Some(text) = rest.strip_prefix("text:") {
            Target::Text(text.to_owned())
        } else {
            Target::Key(rest.to_owned())
        };
        Locator {
            source: source.to_owned(),
            scope,
            target,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn plain_key(&self) -> Option<&str> {
        match (&self.scope, &self.target) {
            (None, Target::Key(key)) => Some(key),
            _ => None,
        }
    }

    pub fn matches(&self, e: &Element) -> bool {
        if let Some(scope) = &self.scope
            && !e.within(scope)
        {
            return false;
        }
        match &self.target {
            Target::Key(key) => {
                e.key.as_deref() == Some(key.as_str())
                    || e.path == *key
                    || e.path.ends_with(&format!("/{key}"))
            }
            Target::Label(label) => e
                .label
                .as_deref()
                .is_some_and(|l| l.contains(label.as_str())),
            Target::Text(text) => e.text.as_deref().is_some_and(|t| t.contains(text.as_str())),
        }
    }

    pub fn find<'a>(&self, elements: &'a [Element]) -> Option<&'a Element> {
        elements
            .iter()
            .filter(|e| self.matches(e))
            .find(|e| e.shown())
    }
}

#[cfg(test)]
mod tests;
