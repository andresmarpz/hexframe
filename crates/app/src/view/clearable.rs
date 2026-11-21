use rune_kit::TextInput;

pub(super) fn clear_when_filled(input: TextInput, text: &str) -> TextInput {
    if text.is_empty() {
        input
    } else {
        input.clear_button()
    }
}
