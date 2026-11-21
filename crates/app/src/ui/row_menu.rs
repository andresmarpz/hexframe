use rune::PointerPress;

use super::Ui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RowMenu<'a> {
    pub id: &'a str,
    pub at: PointerPress,
}

impl Ui {
    pub(crate) fn open_row_menu(&mut self, id: String, at: PointerPress) {
        self.row_menu = Some((id, at));
    }

    pub(crate) fn close_row_menu(&mut self) {
        self.row_menu = None;
    }

    pub(crate) fn row_menu(&self) -> Option<RowMenu<'_>> {
        let (id, at) = self.row_menu.as_ref()?;
        Some(RowMenu { id, at: *at })
    }

    pub(crate) fn review_deaths_of(&mut self, id: String) {
        self.close_row_menu();
        self.open_review(id);
        self.review_deaths();
    }

    pub(crate) fn ask_delete_from_menu(&mut self, id: String) {
        self.close_row_menu();
        self.ask_delete(id);
    }
}
