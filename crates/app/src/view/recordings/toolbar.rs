use rune::{El, Tx, el, row};
use rune_kit::{Kit, SegmentedControl, Select, SelectAppearance, TextInput, glyphs};

use crate::ui::{ResultFilter, Ui, modes_present};

const ALL_MODES: &str = "All modes";
const SEARCH_WIDTH: f32 = 220.0;
const MODE_WIDTH: f32 = 168.0;

pub(super) fn toolbar(s: &Ui, k: &Kit, layer: &mut Vec<El>) -> El {
    let filter = &s.filter;
    let search = TextInput::new(&filter.champion)
        .label("Filter by champion")
        .placeholder("Filter by champion…")
        .leading_icon(&glyphs::SEARCH)
        .clear_button()
        .width(Some(SEARCH_WIDTH))
        .on_edit(|ui: &mut Ui| &mut ui.filter.champion)
        .build(k)
        .key("filter-champion");
    let labels = ResultFilter::ALL.map(ResultFilter::label);
    let result = SegmentedControl::new(
        &labels,
        ResultFilter::ALL.iter().position(|r| *r == filter.result),
    )
    .label("Result")
    .fit()
    .on_pick(|ui: &mut Ui, i: usize, _: &mut Tx| ui.filter.result = ResultFilter::ALL[i])
    .build(k)
    .key("filter-result");
    let mut options = vec![ALL_MODES.to_owned()];
    options.extend(modes_present(&s.recordings));
    let mode = Select::new("filter-mode", &options)
        .current(Some(filter.mode.as_deref().unwrap_or(ALL_MODES)))
        .open(filter.mode_open)
        .width(MODE_WIDTH)
        .label("Mode")
        .appearance(SelectAppearance::Field)
        .on_toggle(|ui: &mut Ui| ui.filter.mode_open = !ui.filter.mode_open)
        .on_close(|ui: &mut Ui| ui.filter.mode_open = false)
        .on_choose(|ui: &mut Ui, value: &str, _: &mut Tx| {
            ui.filter.mode_open = false;
            ui.filter.mode = (value != ALL_MODES).then(|| value.to_owned());
        })
        .build(k, layer);
    row()
        .key("recordings-toolbar")
        .w_full()
        .h(36.0)
        .gap(8.0)
        .center_y()
        .child(search)
        .child(result)
        .child(el().grow(1.0))
        .child(mode)
}
