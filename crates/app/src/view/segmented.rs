use rune::{El, TextStyle, Tx, el, row, text};
use rune_kit::Kit;

const HEIGHT: f32 = 28.0;
const RADIUS: f32 = 6.0;

pub(super) fn segmented<S: AsRef<str>>(
    k: &Kit,
    key: &'static str,
    label: &str,
    options: &[S],
    selected: Option<usize>,
    pick: impl Fn(&mut crate::ui::Ui, usize, &mut Tx) + Clone + 'static,
) -> El {
    let c = &k.c;
    let count = options.len().max(1);
    let font = TextStyle {
        line_height: 13.0,
        ..k.ty.ui_medium
    };
    let mut group = row()
        .key(key)
        .role("radiogroup")
        .focus_group()
        .label(label.to_owned())
        .h(HEIGHT)
        .center_y()
        .radius(RADIUS)
        .fill(c.layer_01)
        .box_shadow(rune::Shadow {
            spread: 0.5,
            color: c.border_base,
            ..Default::default()
        })
        .equal_columns(count);
    if let Some(index) = selected {
        group = group.child(
            el().key("indicator")
                .h(HEIGHT)
                .radius(RADIUS)
                .fill(c.bg_accent)
                .token_motion(k.motion.control)
                .grid_cell(index)
                .a11y_hidden(),
        );
    }
    for (i, option) in options.iter().enumerate() {
        let on = selected == Some(i);
        let ink = if on { c.text_inverse } else { c.text_muted };
        let mut label = text(option.as_ref().to_owned())
            .key("label")
            .font(font)
            .color(ink)
            .truncate()
            .a11y_hidden();
        if !on {
            label = label.group_color(c.text_base);
        }
        let pick = pick.clone();
        let mut segment = row()
            .key(format!("segment{i}"))
            .role("radio")
            .focusable(true)
            .focus_selected(on)
            .label(option.as_ref().to_owned())
            .position_in_set(i + 1, count)
            .grow(1.0)
            .shrink()
            .h(HEIGHT)
            .px(12.0)
            .center()
            .radius(RADIUS)
            .grid_cell(i)
            .child(label)
            .focus_ring(2.0, 1.0, c.border_focus)
            .on_press_tx(move |ui: &mut crate::ui::Ui, tx: &mut Tx| pick(ui, i, tx));
        if !on {
            segment = k.interactive(segment);
        }
        group = group.child(segment);
    }
    group
}
