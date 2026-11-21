use crate::element::{Element, Locator};

#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub name: String,
    pub snap: bool,
    pub has: Vec<Locator>,
    pub lacks: Vec<Locator>,
    pub text: Vec<String>,
    pub focused: Option<Locator>,
}

impl Check {
    pub fn failures(&self, elements: &[Element]) -> Vec<String> {
        let mut out = Vec::new();
        for locator in &self.has {
            if locator.find(elements).is_none() {
                out.push(format!("missing {}", locator.source()));
            }
        }
        for locator in &self.lacks {
            if locator.find(elements).is_some() {
                out.push(format!("should not show {}", locator.source()));
            }
        }
        for needle in &self.text {
            if !elements.iter().any(|e| e.shown() && e.says(needle)) {
                out.push(format!("no text {needle:?}"));
            }
        }
        if let Some(locator) = &self.focused {
            let focused = elements.iter().any(|e| e.focused && locator.matches(e));
            if !focused {
                out.push(format!("{} is not focused", locator.source()));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
