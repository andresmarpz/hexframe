use crate::headless::reveal;
use rune::El;
use rune_kit::{Kit, Menu, MenuItem};

use crate::ui::Ui;
use crate::ui::library::format::{death_hint, death_total};

pub(crate) fn menu(s: &Ui, k: &Kit, layer: &mut Vec<El>) {
    let Some(open) = s.row_menu() else {
        return;
    };
    let Some(rec) = s.recordings.iter().find(|rec| rec.meta.id == open.id) else {
        return;
    };
    let id = rec.meta.id.clone();
    let mut menu = Menu::new("row-menu")
        .at(open.at.x, open.at.y)
        .open(true)
        .on_close(|ui: &mut Ui| ui.close_row_menu());
    menu = menu.item(MenuItem::new("Open").on_select({
        let id = id.clone();
        move |ui: &mut Ui| {
            ui.close_row_menu();
            ui.open_review(id.clone());
        }
    }));
    let deaths = death_total(rec);
    if deaths > 0 {
        let mut item = MenuItem::new(format!("Review deaths ({deaths})"));
        if let Some(hint) =
            death_hint(rec).and_then(|h| h.split_once(" · ").map(|(_, r)| r.to_owned()))
        {
            item = item.description(hint);
        }
        let id = id.clone();
        menu = menu.item(item.on_select(move |ui: &mut Ui| ui.review_deaths_of(id.clone())));
    }
    let folder = rec.folder.join(&rec.meta.video);
    menu = menu.item(
        MenuItem::new("Show in folder").on_select(move |ui: &mut Ui| {
            ui.close_row_menu();
            reveal(&folder);
        }),
    );
    menu = menu.separator().item(
        MenuItem::new("Delete…")
            .danger(true)
            .on_select(move |ui: &mut Ui| ui.ask_delete_from_menu(id.clone())),
    );
    menu.build(k, layer);
}
