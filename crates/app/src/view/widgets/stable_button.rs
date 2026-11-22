use rune::{Color, El, TextStyle, el, text};
use rune_kit::{Kit, Spinner};

const SPINNER_SIZE: f32 = 14.0;

pub(crate) fn stable_label_content(
    widest: &'static str,
    shown: &str,
    font: TextStyle,
    ink: Color,
) -> El {
    el().center()
        .child(text(widest).font(font).opacity(0.0).a11y_hidden())
        .child(
            el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .center()
                .child(text(shown.to_owned()).font(font).color(ink)),
        )
}

pub(crate) fn busy_label_content(
    k: &Kit,
    label: &'static str,
    busy: bool,
    spinner_key: &'static str,
    font: TextStyle,
    ink: Color,
) -> El {
    let label_el = text(label)
        .font(font)
        .opacity(if busy { 0.0 } else { 1.0 })
        .color(ink);
    let label_el = if busy {
        label_el.a11y_hidden()
    } else {
        label_el
    };
    let overlay = busy.then(|| {
        el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
            .center()
            .child(Spinner::new().size(SPINNER_SIZE).build(k).key(spinner_key))
    });
    el().center().child(label_el).children(overlay)
}
