use rune::Icon;

pub(super) static PLAY: Icon = Icon {
    name: "greyscreen-play",
    view: 16.0,
    body: r#"<polygon points="5,3 13,8 5,13" fill="currentColor" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>"#,
};

pub(super) static PAUSE: Icon = Icon {
    name: "greyscreen-pause",
    view: 16.0,
    body: r#"<rect x="3.5" y="3" width="3" height="10" rx="1" fill="currentColor"/><rect x="9.5" y="3" width="3" height="10" rx="1" fill="currentColor"/>"#,
};

pub(super) static EXPAND: Icon = Icon {
    name: "greyscreen-expand",
    view: 16.0,
    body: concat!(
        r#"<path d="M10 2.5h3.5V6M6 13.5H2.5V10M13.5 2.5 9.5 6.5M2.5 13.5l4-4" "#,
        r#"fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#
    ),
};

pub(super) static SHRINK: Icon = Icon {
    name: "greyscreen-shrink",
    view: 16.0,
    body: concat!(
        r#"<path d="M13 6H9.5V2.5M3 10h3.5v3.5M9.5 6.5l4-4M6.5 9.5l-4 4" "#,
        r#"fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#
    ),
};
