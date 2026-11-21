mod rows;

use std::borrow::Cow;
use std::rc::Rc;

use greyscreen_engine::{EngineCmd, EngineStatus};
use rune::{El, Icon, Key, LoopSpec, TextStyle, Tx, col, el, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Sidebar, SidebarHeader,
    TextInput, glyphs,
};

use crate::ui::library_format::summary;
use crate::ui::{ListItem, Ui, format_clock};

static LOGO: Icon = Icon {
    name: "greyscreen-logo",
    view: 1024.0,
    body: r#"<path fill="currentColor" fill-rule="evenodd" d="M468,101.4A88,88 0 0 1 556,101.4L845.59,268.6A88,88 0 0 1 889.59,344.81L889.59,679.19A88,88 0 0 1 845.59,755.4L556,922.6A88,88 0 0 1 468,922.6L178.41,755.4A88,88 0 0 1 134.41,679.19L134.41,344.81A88,88 0 0 1 178.41,268.6ZM248.6,346.07A24,24 0 0 0 236.6,366.86L236.6,657.14A24,24 0 0 0 248.6,677.93L500,823.07A24,24 0 0 0 524,823.07L775.4,677.93A24,24 0 0 0 787.4,657.14L787.4,366.86A24,24 0 0 0 775.4,346.07L524,200.93A24,24 0 0 0 500,200.93Z"/><path fill="currentColor" d="M682,487.75A28,28 0 0 1 682,536.25L448,671.35A28,28 0 0 1 406,647.1L406,376.9A28,28 0 0 1 448,352.65Z"/>"#,
};

static GEAR: Icon = Icon {
    name: "greyscreen-gear",
    view: 16.0,
    body: concat!(
        r#"<path d="M6.60 2.78 L6.78 1.11 L9.22 1.11 L9.40 2.78 L10.70 3.32 L12.02 2.27 L13.73 3.98 L12.68 5.30 L13.22 6.60 L14.89 6.78 L14.89 9.22 L13.22 9.40 L12.68 10.70 L13.73 12.02 L12.02 13.73 L10.70 12.68 L9.40 13.22 L9.22 14.89 L6.78 14.89 L6.60 13.22 L5.30 12.68 L3.98 13.73 L2.27 12.02 L3.32 10.70 L2.78 9.40 L1.11 9.22 L1.11 6.78 L2.78 6.60 L3.32 5.30 L2.27 3.98 L3.98 2.27 L5.30 3.32Z" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linejoin="round"/>"#,
        r#"<circle cx="8" cy="8" r="2.1" fill="none" stroke="currentColor" stroke-width="1.25"/>"#
    ),
};

pub(super) const ROW_HEIGHT: f32 = 56.0;
const PULSE_SECS: f64 = 1.4;
const TABULAR: &str = "'tnum' 1";

pub(super) fn content(s: &Ui, k: &Kit) -> El {
    let items: Rc<[ListItem]> = s.list_items().into();
    let mut sidebar = Sidebar::new()
        .label("Games")
        .row_height(ROW_HEIGHT)
        .header_padding(4.0, 8.0)
        .header(status_bar(s, k))
        .header(search(s, k));
    if items.is_empty() {
        sidebar = sidebar.child(empty(s, k));
    } else {
        let key_items = items.clone();
        let kit = k.clone();
        sidebar = sidebar.virtual_rows(
            items.len(),
            move |s: &Ui, i| key_of(s, &key_items[i]),
            move |s: &Ui, i| match &items[i] {
                ListItem::Live => rows::live(s, &kit),
                ListItem::Day(label) => SidebarHeader::new(label.clone()).build(&kit),
                ListItem::Game(index) => match s.recordings.get(*index) {
                    Some(rec) => {
                        rows::game(rec, s.selected_id() == Some(rec.meta.id.as_str()), &kit)
                    }
                    None => el(),
                },
            },
        );
    }
    if !s.recordings.is_empty() {
        sidebar = sidebar.footer(footer(s, k));
    }
    sidebar.build(k)
}

fn key_of<'a>(s: &'a Ui, item: &ListItem) -> Cow<'a, str> {
    match item {
        ListItem::Live => Cow::Borrowed("live-row"),
        ListItem::Day(label) => Cow::Owned(format!("day-{label}")),
        ListItem::Game(index) => s
            .recordings
            .get(*index)
            .map_or(Cow::Borrowed("missing"), |rec| {
                Cow::Borrowed(rec.meta.id.as_str())
            }),
    }
}

struct Status {
    label: String,
    dot: rune::Color,
    pulse: bool,
}

fn status(s: &Ui, k: &Kit) -> Status {
    let quiet = |label: &str, dot| Status {
        label: label.to_owned(),
        dot,
        pulse: false,
    };
    match &s.status {
        EngineStatus::Idle if s.settings.auto_record => quiet("Ready", k.c.success.icon),
        EngineStatus::Idle => quiet("Recording off", k.c.text_faint),
        EngineStatus::GameRunning => quiet("Not recording", k.c.text_faint),
        EngineStatus::Waiting => quiet("Waiting for game", k.c.warning.icon),
        EngineStatus::Finalizing => Status {
            label: "Saving…".to_owned(),
            dot: k.c.warning.icon,
            pulse: true,
        },
        EngineStatus::Recording { .. } => Status {
            label: format!("Recording {}", format_clock(s.elapsed_secs)),
            dot: k.c.danger.icon,
            pulse: true,
        },
        EngineStatus::Error(_) => quiet("Capture failed", k.c.danger.icon),
    }
}

pub(super) fn dot(color: rune::Color, pulse: bool) -> El {
    let dot = el().size(8.0, 8.0).radius(4.0).fill(color).a11y_hidden();
    if pulse {
        dot.looping(LoopSpec::opacity(PULSE_SECS, 0.0, 0.25, 1.0))
    } else {
        dot
    }
}

fn status_bar(s: &Ui, k: &Kit) -> El {
    let shown = status(s, k);
    let font = TextStyle {
        features: TABULAR,
        ..k.ty.ui_medium
    };
    let mut bar = row()
        .key("nav-status")
        .w_full()
        .h(32.0)
        .pl(4.0)
        .gap(8.0)
        .center_y()
        .child(rune::icon(&LOGO, 18.0).color(k.c.bg_accent).a11y_hidden())
        .child(dot(shown.dot, shown.pulse))
        .child(
            text(shown.label)
                .key("nav-status-label")
                .font(font)
                .color(k.c.text_base)
                .truncate(),
        )
        .child(el().grow(1.0).a11y_hidden());
    if matches!(s.status, EngineStatus::Recording { .. }) {
        bar = bar.child(
            Button::new("Save clip")
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Small)
                .on_press(|ui: &mut Ui| ui.send(EngineCmd::SaveClipNow))
                .build(k)
                .key("nav-save-clip"),
        );
    }
    bar.child(
        IconButton::new(&GEAR)
            .label("Settings")
            .help("Settings")
            .variant(IconButtonVariant::GhostMuted)
            .on_press(|ui: &mut Ui| ui.toggle_settings())
            .build(k)
            .key("nav-settings"),
    )
}

fn search(s: &Ui, k: &Kit) -> El {
    TextInput::new(&s.filter.champion)
        .label("Search games")
        .placeholder("Search games")
        .leading_icon(&glyphs::SEARCH)
        .clear_button()
        .width(None)
        .on_edit(|ui: &mut Ui| &mut ui.filter.champion)
        .on_key(search_key)
        .build(k)
        .key("filter-champion")
}

fn search_key(ui: &mut Ui, ev: &rune::KeyEvent, tx: &mut Tx) -> bool {
    match ev.key {
        Key::Up => ui.step_selection(-1),
        Key::Down => ui.step_selection(1),
        Key::Enter if ui.review.is_some() => tx.focus("review-video", true),
        _ => return false,
    }
    true
}

fn empty(s: &Ui, k: &Kit) -> El {
    let (title, detail, action) = if s.filter.is_active() {
        (
            "No games match",
            "Try another champion, mode or result.",
            Some(
                Button::new("Clear search")
                    .variant(ButtonVariant::Neutral)
                    .on_press(|ui: &mut Ui| ui.filter.clear())
                    .build(k)
                    .key("clear-filters"),
            ),
        )
    } else {
        (
            "No games yet",
            "Games you play are recorded and show up here.",
            None,
        )
    };
    let mut body = col()
        .key("games-empty")
        .w_full()
        .px(16.0)
        .py(32.0)
        .gap(12.0)
        .items_center()
        .child(text(title).font(k.ty.body_medium).color(k.c.text_base))
        .child(
            text(detail)
                .wrap()
                .max_w(240.0)
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    if let Some(action) = action {
        body = body.child(action);
    }
    body
}

fn footer(s: &Ui, k: &Kit) -> El {
    let games = s.filter.apply(&s.recordings);
    text(summary(&games, s.recordings.len()))
        .key("recordings-summary")
        .font(k.ty.small.features(TABULAR))
        .color(k.c.text_faint)
        .truncate()
        .px(8.0)
        .a11y_hidden()
}
