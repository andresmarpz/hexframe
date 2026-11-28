use super::{Scene, home};
use crate::view::icon_sheet;

pub(super) const SCENES: &[Scene] = &[("icons", |ui| {
    home(ui);
    icon_sheet::show();
})];
