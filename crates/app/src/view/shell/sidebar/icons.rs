use rune::Icon;

pub(in crate::view::shell) use crate::logo::LOGO;

pub(in crate::view::shell) static HOME: Icon = Icon {
    name: "greyscreen-home",
    view: 16.0,
    body: r#"<path d="M2.5 7.4 8 2.6l5.5 4.8M4 6.4V13h8V6.4M6.5 13V9.5h3V13" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub(in crate::view::shell) static GAMES: Icon = Icon {
    name: "greyscreen-games",
    view: 16.0,
    body: concat!(
        r#"<rect x="1.75" y="2.5" width="12.5" height="11" rx="2.25" fill="none" stroke="currentColor" stroke-width="1.5"/>"#,
        r#"<path d="M2 6.2h12M2 9.8h12M6 6.2v7.3" fill="none" stroke="currentColor" stroke-width="1.5"/>"#
    ),
};

pub(in crate::view::shell) static CLIPS: Icon = Icon {
    name: "greyscreen-clips",
    view: 16.0,
    body: concat!(
        r#"<rect x="1.75" y="3" width="12.5" height="10" rx="2.25" fill="none" stroke="currentColor" stroke-width="1.5"/>"#,
        r#"<path d="M6.8 5.9v4.2l3.4-2.1z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>"#
    ),
};

pub(in crate::view::shell) static GEAR: Icon = Icon {
    name: "greyscreen-gear",
    view: 16.0,
    body: concat!(
        r#"<path d="M6.60 2.78 L6.78 1.11 L9.22 1.11 L9.40 2.78 L10.70 3.32 L12.02 2.27 L13.73 3.98 L12.68 5.30 L13.22 6.60 L14.89 6.78 L14.89 9.22 L13.22 9.40 L12.68 10.70 L13.73 12.02 L12.02 13.73 L10.70 12.68 L9.40 13.22 L9.22 14.89 L6.78 14.89 L6.60 13.22 L5.30 12.68 L3.98 13.73 L2.27 12.02 L3.32 10.70 L2.78 9.40 L1.11 9.22 L1.11 6.78 L2.78 6.60 L3.32 5.30 L2.27 3.98 L3.98 2.27 L5.30 3.32Z" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linejoin="round"/>"#,
        r#"<circle cx="8" cy="8" r="2.1" fill="none" stroke="currentColor" stroke-width="1.25"/>"#
    ),
};

pub(super) static SIDEBAR: Icon = Icon {
    name: "greyscreen-sidebar",
    view: 16.0,
    body: concat!(
        r#"<rect x="1.75" y="2.5" width="12.5" height="11" rx="2.25" fill="none" stroke="currentColor" stroke-width="1.5"/>"#,
        r#"<path d="M6 2.7v10.6" fill="none" stroke="currentColor" stroke-width="1.5"/>"#
    ),
};
