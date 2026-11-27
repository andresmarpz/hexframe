use rune::Icon;

pub(in crate::view::shell) use crate::logo::LOGO;

pub(in crate::view::shell) static HOME: Icon = Icon {
    name: "greyscreen-home",
    view: 16.0,
    body: r#"<path d="M2.5 7.5 8 2.5l5.5 5M4.5 6.5v7h7v-7M6.5 13.5v-4h3v4" fill="none" stroke="currentColor" stroke-width="1.33" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub(in crate::view::shell) static GAMES: Icon = Icon {
    name: "greyscreen-games",
    view: 16.0,
    body: concat!(
        r#"<rect x="1.5" y="2.5" width="13" height="11" rx="2" fill="none" stroke="currentColor" stroke-width="1.33"/>"#,
        r#"<path d="M1.5 6.5h13M1.5 10.5h13M6.5 6.5v7" fill="none" stroke="currentColor" stroke-width="1.33"/>"#
    ),
};

pub(in crate::view::shell) static CLIPS: Icon = Icon {
    name: "greyscreen-clips",
    view: 16.0,
    body: concat!(
        r#"<rect x="1.5" y="3.5" width="13" height="9" rx="2" fill="none" stroke="currentColor" stroke-width="1.33"/>"#,
        r#"<path d="M6.5 5.5v5l4-2.5z" fill="none" stroke="currentColor" stroke-width="1.33" stroke-linejoin="round"/>"#
    ),
};

pub(in crate::view::shell) static GEAR: Icon = Icon {
    name: "greyscreen-gear",
    view: 16.0,
    body: concat!(
        r#"<path d="M6.60 2.78 L6.78 1.11 L9.22 1.11 L9.40 2.78 L10.70 3.32 L12.02 2.27 L13.73 3.98 L12.68 5.30 L13.22 6.60 L14.89 6.78 L14.89 9.22 L13.22 9.40 L12.68 10.70 L13.73 12.02 L12.02 13.73 L10.70 12.68 L9.40 13.22 L9.22 14.89 L6.78 14.89 L6.60 13.22 L5.30 12.68 L3.98 13.73 L2.27 12.02 L3.32 10.70 L2.78 9.40 L1.11 9.22 L1.11 6.78 L2.78 6.60 L3.32 5.30 L2.27 3.98 L3.98 2.27 L5.30 3.32Z" fill="none" stroke="currentColor" stroke-width="1.33" stroke-linejoin="round"/>"#,
        r#"<circle cx="8" cy="8" r="2.1" fill="none" stroke="currentColor" stroke-width="1.33"/>"#
    ),
};

pub(in crate::view::shell) static FEEDBACK: Icon = Icon {
    name: "greyscreen-feedback",
    view: 16.0,
    body: concat!(
        r#"<path d="M3.5 2.5h9a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2H8.5L5.5 14v-2.5h-2a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2Z" fill="none" stroke="currentColor" stroke-width="1.33" stroke-linejoin="round"/>"#,
        r#"<path d="M5.5 5.5h5M5.5 8.5h3" fill="none" stroke="currentColor" stroke-width="1.33" stroke-linecap="round"/>"#
    ),
};

pub(super) static SIDEBAR: Icon = Icon {
    name: "greyscreen-sidebar",
    view: 16.0,
    body: concat!(
        r#"<rect x="1.5" y="2.5" width="13" height="11" rx="2" fill="none" stroke="currentColor" stroke-width="1.33"/>"#,
        r#"<path d="M6.5 2.5v11" fill="none" stroke="currentColor" stroke-width="1.33"/>"#
    ),
};
