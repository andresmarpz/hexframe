use rune::{El, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Menu, MenuItem, Placement};

use crate::ui::{ReviewState, Ui};
use crate::view::review::tabular;

const MENU_WIDTH: f32 = 224.0;

fn menu(open: bool) -> Menu {
    Menu::new("filter-menu")
        .open(open)
        .placement(Placement::BottomStart)
        .min_width(MENU_WIDTH)
        .on_close(|ui: &mut Ui| ui.review_filter_menu(false))
}

pub(super) fn bar(review: &ReviewState, k: &Kit) -> El {
    let open = review.filter_menu;
    let hidden = review.hidden_events();
    let button = Button::new("Filter")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Small)
        .help("Choose which events to show")
        .expanded(open)
        .on_press(move |ui: &mut Ui| ui.review_filter_menu(!open))
        .build(k)
        .key("event-filter");
    let mut line = row()
        .key("event-filters")
        .w_full()
        .px(8.0)
        .py(8.0)
        .gap(8.0)
        .center_y()
        .child(menu(open).trigger(button).build(k, &mut Vec::new()));
    if hidden > 0 {
        line = line.child(
            text(format!("{hidden} hidden"))
                .key("event-filter-hidden")
                .font(tabular(k.ty.small))
                .color(k.c.text_muted),
        );
    }
    line = line.child(el().grow(1.0));
    if review.filter_differs_from_default() {
        line = line.child(
            Button::new("Reset")
                .variant(ButtonVariant::Ghost)
                .control_size(ControlSize::Small)
                .on_press_tx(|ui: &mut Ui, tx: &mut rune::Tx| ui.review_use_default_filter(tx))
                .build(k)
                .key("event-filter-custom"),
        );
    }
    line
}

pub(crate) fn overlay(review: &ReviewState, k: &Kit, layer: &mut Vec<El>) {
    if !review.filter_menu {
        return;
    }
    let differs = review.filter_differs_from_default();
    let mut menu = menu(true);
    for (at, group) in greyscreen_core::categories::EventCategory::GROUPS
        .iter()
        .enumerate()
    {
        if at > 0 {
            menu = menu.separator();
        }
        for category in *group {
            let category = *category;
            menu = menu.item(
                MenuItem::new(category.label())
                    .checked(review.filter.contains(category))
                    .on_select_tx(move |ui: &mut Ui, tx: &mut rune::Tx| {
                        ui.review_toggle_category(category, tx);
                    }),
            );
        }
    }
    menu = menu.separator().item(
        MenuItem::new("Save as my default")
            .disabled(!differs)
            .on_select_tx(|ui: &mut Ui, tx: &mut rune::Tx| ui.review_save_filter_as_default(tx)),
    );
    menu = menu.item(
        MenuItem::new("Use my default")
            .disabled(!differs)
            .on_select_tx(|ui: &mut Ui, tx: &mut rune::Tx| ui.review_use_default_filter(tx)),
    );
    menu.build(k, layer);
}
