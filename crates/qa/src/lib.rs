mod case;
mod check;
mod element;
mod image;
mod script;

pub use case::{Case, CaseError, KeyPress, Step};
pub use check::Check;
pub use element::{Element, Locator};
pub use image::{Diff, ImageError, Rgba, compare};
pub use script::{Plan, PlannedCheck, Target, compile};

pub const FIXED_NOW: &str = "2026-11-21T15:00:00+00:00[UTC]";
pub const SETTLE_SECS: f64 = 0.5;
