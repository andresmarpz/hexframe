use greyscreen_core::keybinds::Group;
use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Dialog, Kit, SidebarRow, TextInput, glyphs};

mod about;
mod catalog;
mod clips;
mod keybinds;
mod recording;
mod storage;
mod updates;

use super::{card, hairline};
use crate::ui::{SettingsSection, Ui};
use catalog::Row;

const SHEET_WIDTH: f32 = 760.0;
const BODY_HEIGHT: f32 = 484.0;
const NAV_WIDTH: f32 = 188.0;
const SCROLL_FADE: f32 = 20.0;
const SUBTITLE: &str = "Changes are saved automatically.";

pub(super) fn sheet(s: &Ui, k: &Kit, layer: &mut Vec<El>) {
    if !s.settings_open {
        return;
    }
    let query = s.settings_search.text.trim().to_owned();
    let mut popups = Vec::new();
    let content = if query.is_empty() {
        section_content(s, k, &mut popups, s.settings_section)
    } else {
        search_content(s, k, &mut popups, &query)
    };
    let body = col()
        .key("settings-sheet")
        .w_full()
        .h(BODY_HEIGHT)
        .child(search_bar(s, k))
        .child(hairline(k))
        .child(
            row()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .child(nav(s, k, query.is_empty()))
                .child(el().w(1.0).h_full().fill(k.c.border_muted).a11y_hidden())
                .child(scroller(content)),
        );
    Dialog::new("settings", "Settings")
        .description(SUBTITLE)
        .width(SHEET_WIDTH)
        .fit()
        .open(true)
        .on_close(|ui: &mut Ui| ui.close_settings())
        .child(body)
        .build(k, layer);
    layer.extend(popups);
}

fn search_bar(s: &Ui, k: &Kit) -> El {
    let input = TextInput::new(&s.settings_search)
        .label("Search settings")
        .placeholder("Search settings…")
        .leading_icon(&glyphs::SEARCH)
        .clear_button()
        .width(Some(SHEET_WIDTH - 32.0))
        .on_edit(|ui: &mut Ui| &mut ui.settings_search)
        .build(k)
        .key("settings-search");
    row().w_full().px(16.0).pb(12.0).child(input)
}

fn nav(s: &Ui, k: &Kit, active: bool) -> El {
    let mut list = col()
        .key("settings-nav")
        .w(NAV_WIDTH)
        .h_full()
        .padding(8.0)
        .gap(2.0);
    for section in SettingsSection::ALL {
        list = list.child(
            SidebarRow::new(section.label())
                .selected(active && s.settings_section == section)
                .on_select(move |ui: &mut Ui, _| ui.pick_settings_section(section))
                .build(k)
                .key(format!("settings-nav-{}", section.id())),
        );
    }
    list
}

fn scroller(content: El) -> El {
    col()
        .key("settings-scroll")
        .grow(1.0)
        .h_full()
        .min_w(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_edge_fade(SCROLL_FADE)
        .child(col().w_full().padding(16.0).gap(12.0).child(content))
}

fn section_content(s: &Ui, k: &Kit, layer: &mut Vec<El>, section: SettingsSection) -> El {
    let mut page = col()
        .key(format!("settings-{}", section.id()))
        .w_full()
        .gap(12.0);
    match section {
        SettingsSection::Recording => page = page.children(recording::notice(s, k)),
        SettingsSection::Keybinds => page = page.child(keybinds::header(s, k)),
        _ => {}
    }
    let rows = catalog::rows_in(section);
    if section == SettingsSection::Keybinds {
        return page.children(keybind_groups(s, k, layer, &rows));
    }
    page.child(group(
        s,
        k,
        layer,
        format!("{}-card", section.id()),
        section,
        &rows,
    ))
}

fn keybind_groups(s: &Ui, k: &Kit, layer: &mut Vec<El>, rows: &[Row]) -> Vec<El> {
    Group::ALL
        .iter()
        .map(|g| {
            let in_group: Vec<Row> = rows
                .iter()
                .copied()
                .filter(|r| matches!(r, Row::Bind(a) if a.group() == *g))
                .collect();
            col()
                .w_full()
                .gap(8.0)
                .child(
                    text(g.label())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_muted)
                        .ml(4.0),
                )
                .child(group(
                    s,
                    k,
                    layer,
                    format!("keybinds-{}", g.label().to_lowercase()),
                    SettingsSection::Keybinds,
                    &in_group,
                ))
        })
        .collect()
}

fn search_content(s: &Ui, k: &Kit, layer: &mut Vec<El>, query: &str) -> El {
    let found = catalog::search(query);
    if found.is_empty() {
        return col()
            .key("settings-no-results")
            .w_full()
            .py(48.0)
            .items_center()
            .gap(4.0)
            .child(
                text(format!("No settings match “{query}”"))
                    .font(k.ty.body_medium)
                    .color(k.c.text_base),
            )
            .child(
                text("Try a different word, like “quality” or “folder”.")
                    .font(k.ty.small)
                    .color(k.c.text_muted),
            );
    }
    let mut page = col().key("settings-results").w_full().gap(16.0);
    for (section, rows) in found {
        page = page.child(
            col()
                .w_full()
                .gap(8.0)
                .child(
                    text(section.label())
                        .font(k.ty.ui_medium)
                        .color(k.c.text_muted)
                        .ml(4.0),
                )
                .child(group(
                    s,
                    k,
                    layer,
                    format!("{}-results", section.id()),
                    section,
                    &rows,
                )),
        );
    }
    page
}

fn group(
    s: &Ui,
    k: &Kit,
    layer: &mut Vec<El>,
    key: String,
    section: SettingsSection,
    rows: &[Row],
) -> El {
    let dimmed = section == SettingsSection::Clips && !s.settings.clips.enabled;
    let mut body = card(k).key(key);
    let last = rows.len().saturating_sub(1);
    for (i, row) in rows.iter().enumerate() {
        let built = render(s, k, layer, *row);
        let off = dimmed && matches!(row, Row::Moments | Row::ClipWindow);
        body = body.child(if off { built.opacity(0.45) } else { built });
        if i < last {
            body = body.child(hairline(k));
        }
    }
    body
}

fn render(s: &Ui, k: &Kit, layer: &mut Vec<El>, row: Row) -> El {
    match row {
        Row::RecordGames
        | Row::FullGame
        | Row::Quality
        | Row::Resolution
        | Row::FrameRate
        | Row::Audio
        | Row::Cursor => recording::render(s, k, layer, row),
        Row::ClipsEnabled | Row::Moments | Row::ClipWindow | Row::ClipHotkey => {
            clips::render(s, k, row)
        }
        Row::Folder | Row::StorageLimit => storage::render(s, k, layer, row),
        Row::Bind(action) => keybinds::bind_row(s, k, action),
        Row::Version
        | Row::Updates
        | Row::AutoUpdates
        | Row::Logs
        | Row::StartWithWindows
        | Row::RiotNotice => about::render(s, k, row),
    }
}

fn setting_row(k: &Kit, key: &str, label: &str, description: Option<&str>, control: El) -> El {
    row()
        .key(key.to_owned())
        .w_full()
        .min_h(52.0)
        .px(16.0)
        .py(8.0)
        .gap(16.0)
        .center_y()
        .justify_between()
        .child(copy(k, label, description))
        .child(control)
}

fn stacked_row(k: &Kit, key: &str, label: &str, description: Option<&str>, control: El) -> El {
    col()
        .key(key.to_owned())
        .w_full()
        .px(16.0)
        .py(12.0)
        .gap(10.0)
        .child(copy(k, label, description))
        .child(control)
}

fn copy(k: &Kit, label: &str, description: Option<&str>) -> El {
    let mut copy = col().gap(2.0).grow(1.0).min_w(0.0).child(
        text(label.to_owned())
            .font(k.ty.body_medium)
            .color(k.c.text_base),
    );
    if let Some(description) = description {
        copy = copy.child(
            text(description.to_owned())
                .wrap()
                .w_full()
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    }
    copy
}

fn toggle_row(
    k: &Kit,
    key: &'static str,
    label: &str,
    description: Option<&str>,
    on: bool,
    flip: fn(&mut Ui),
) -> El {
    let switch = rune_kit::Switch::new(on)
        .label(label.to_owned())
        .hide_label()
        .on_toggle(flip)
        .build(k)
        .key(format!("{key}-switch"));
    setting_row(k, key, label, description, switch)
}
