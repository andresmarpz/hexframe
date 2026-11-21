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

pub(super) static ARROW_LEFT: Icon = Icon {
    name: "greyscreen-arrow-left",
    view: 16.0,
    body: r#"<path d="M13 8H3M7 4 3 8l4 4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub(super) static REWIND: Icon = Icon {
    name: "greyscreen-rewind",
    view: 16.0,
    body: concat!(
        r#"<path d="M3.2 8a4.8 4.8 0 1 0 1.5-3.5M3 2.5v2.8h2.8" "#,
        r#"fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#
    ),
};

pub(super) static FORWARD: Icon = Icon {
    name: "greyscreen-forward",
    view: 16.0,
    body: concat!(
        r#"<path d="M12.8 8a4.8 4.8 0 1 1-1.5-3.5M13 2.5v2.8h-2.8" "#,
        r#"fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#
    ),
};

pub(super) static VOLUME: Icon = Icon {
    name: "greyscreen-volume",
    view: 16.0,
    body: concat!(
        r#"<path d="M2.5 6v4h2.5l3.5 3V3L5 6z" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round"/>"#,
        r#"<path d="M11 5.5a3.5 3.5 0 0 1 0 5M12.8 3.7a6 6 0 0 1 0 8.6" "#,
        r#"fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>"#
    ),
};

pub(super) static MUTED: Icon = Icon {
    name: "greyscreen-muted",
    view: 16.0,
    body: concat!(
        r#"<path d="M2.5 6v4h2.5l3.5 3V3L5 6z" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round"/>"#,
        r#"<path d="M11.2 6l3.3 4M14.5 6l-3.3 4" "#,
        r#"fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>"#
    ),
};

pub(super) static LIST: Icon = Icon {
    name: "greyscreen-list",
    view: 16.0,
    body: concat!(
        r#"<path d="M5.5 4h8M5.5 8h8M5.5 12h8" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>"#,
        r#"<circle cx="2.5" cy="4" r="1" fill="currentColor"/><circle cx="2.5" cy="8" r="1" fill="currentColor"/><circle cx="2.5" cy="12" r="1" fill="currentColor"/>"#
    ),
};

pub(super) static TRASH: Icon = Icon {
    name: "greyscreen-trash",
    view: 16.0,
    body: concat!(
        r#"<path d="M2.5 4.5h11M6 4.5V3a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v1.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
        r#"<path d="M4 4.5l.6 8a1.5 1.5 0 0 0 1.5 1.4h3.8a1.5 1.5 0 0 0 1.5-1.4l.6-8M6.8 7v4.5M9.2 7v4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#
    ),
};
