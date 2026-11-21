use rune::Color;
use rune_kit::Kit;

fn css(c: Color) -> String {
    let hex = c.hex();
    match hex.strip_suffix("ff") {
        Some(opaque) => opaque.to_owned(),
        None => hex,
    }
}

fn colors(kit: &Kit) -> String {
    let c = &kit.c;
    let mut out = String::new();
    let mut put = |name: &str, color: Color| {
        out += &format!("  --{name}: {};\n", css(color));
    };
    put("page", c.page);
    put("surface", c.bg_base);
    put("surface-deep", c.bg_deep);
    put("layer-1", c.layer_01);
    put("layer-2", c.layer_02);
    put("layer-3", c.layer_03);
    put("layer-4", c.layer_04);
    put("accent", c.bg_accent);
    put("text", c.text_base);
    put("text-muted", c.text_muted);
    put("text-faint", c.text_faint);
    put("text-inverse", c.text_inverse);
    put("text-accent", c.text_accent);
    put("border-muted", c.border_muted);
    put("border", c.border_base);
    put("border-strong", c.border_strong);
    put("border-focus", c.border_focus);
    put("hover", c.hover);
    put("pressed", c.pressed);
    put("scrim", c.scrim);
    for (name, state) in [
        ("success", c.success),
        ("warning", c.warning),
        ("danger", c.danger),
        ("info", c.info),
    ] {
        put(&format!("{name}-bg"), state.bg);
        put(&format!("{name}-fg"), state.fg);
        put(&format!("{name}-border"), state.border);
        put(&format!("{name}-icon"), state.icon);
    }
    out
}

fn sizes(kit: &Kit) -> String {
    let mut out = String::new();
    let scale = [
        ("ui", kit.ty.ui),
        ("small", kit.ty.small),
        ("body", kit.ty.body),
        ("text-small", kit.ty.text_small),
        ("text-xsmall", kit.ty.text_xsmall),
        ("mono", kit.ty.mono),
    ];
    for (name, style) in scale {
        out += &format!("  --type-{name}-size: {}px;\n", style.size);
        out += &format!("  --type-{name}-line: {}px;\n", style.line_height);
    }
    out += &format!("  --weight-regular: {};\n", kit.ty.ui.weight);
    out += &format!("  --weight-medium: {};\n", kit.ty.ui_medium.weight);
    let r = kit.radius;
    for (name, value) in [
        ("xs", r.xs),
        ("sm", r.sm),
        ("md", r.md),
        ("lg", r.lg),
        ("xl", r.xl),
    ] {
        out += &format!("  --radius-{name}: {value}px;\n");
    }
    out
}

fn indent(css: &str) -> String {
    css.lines().map(|line| format!("  {line}\n")).collect()
}

fn main() {
    let dark = Kit::neutral(true);
    let light = Kit::neutral(false);
    print!(
        ":root {{\n  color-scheme: dark;\n{}{}}}\n\n@media (prefers-color-scheme: light) {{\n  :root {{\n    color-scheme: light;\n{}  }}\n}}\n",
        colors(&dark),
        sizes(&dark),
        indent(&colors(&light)),
    );
}
