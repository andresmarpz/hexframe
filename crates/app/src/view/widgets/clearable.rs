use rune::{TextEdit, Tx};
use rune_kit::TextInput;

use crate::ui::Ui;

pub(crate) fn clear_when_filled(
    input: TextInput,
    text: &str,
    field: fn(&mut Ui) -> &mut TextEdit,
) -> TextInput {
    if text.is_empty() {
        input
    } else {
        input
            .clear_button()
            .on_action(move |ui: &mut Ui, _: &mut Tx| field(ui).replace_text(""))
    }
}
