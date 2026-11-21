use rune::Icon;

pub(crate) static EXPORT: Icon = Icon {
    name: "greyscreen-export",
    view: 16.0,
    body: concat!(
        r#"<path d="M8 10V2.5M5 5.2 8 2.2l3 3M3 9.5v3a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1v-3" "#,
        r#"fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#
    ),
};
