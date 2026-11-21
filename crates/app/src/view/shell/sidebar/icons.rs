use rune::Icon;

pub(super) static LOGO: Icon = Icon {
    name: "greyscreen-logo",
    view: 1024.0,
    body: r#"<path fill="currentColor" fill-rule="evenodd" d="M468,101.4A88,88 0 0 1 556,101.4L845.59,268.6A88,88 0 0 1 889.59,344.81L889.59,679.19A88,88 0 0 1 845.59,755.4L556,922.6A88,88 0 0 1 468,922.6L178.41,755.4A88,88 0 0 1 134.41,679.19L134.41,344.81A88,88 0 0 1 178.41,268.6ZM248.6,346.07A24,24 0 0 0 236.6,366.86L236.6,657.14A24,24 0 0 0 248.6,677.93L500,823.07A24,24 0 0 0 524,823.07L775.4,677.93A24,24 0 0 0 787.4,657.14L787.4,366.86A24,24 0 0 0 775.4,346.07L524,200.93A24,24 0 0 0 500,200.93Z"/><path fill="currentColor" d="M682,487.75A28,28 0 0 1 682,536.25L448,671.35A28,28 0 0 1 406,647.1L406,376.9A28,28 0 0 1 448,352.65Z"/>"#,
};

pub(super) static HOME: Icon = Icon {
    name: "greyscreen-home",
    view: 16.0,
    body: r#"<path d="M2.5 7.4 8 2.6l5.5 4.8M4 6.4V13h8V6.4M6.5 13V9.5h3V13" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub(super) static GAMES: Icon = Icon {
    name: "greyscreen-games",
    view: 16.0,
    body: concat!(
        r#"<rect x="1.75" y="2.5" width="12.5" height="11" rx="2.25" fill="none" stroke="currentColor" stroke-width="1.5"/>"#,
        r#"<path d="M2 6.2h12M2 9.8h12M6 6.2v7.3" fill="none" stroke="currentColor" stroke-width="1.5"/>"#
    ),
};

pub(super) static GEAR: Icon = Icon {
    name: "greyscreen-gear",
    view: 16.0,
    body: concat!(
        r#"<path d="M6.60 2.78 L6.78 1.11 L9.22 1.11 L9.40 2.78 L10.70 3.32 L12.02 2.27 L13.73 3.98 L12.68 5.30 L13.22 6.60 L14.89 6.78 L14.89 9.22 L13.22 9.40 L12.68 10.70 L13.73 12.02 L12.02 13.73 L10.70 12.68 L9.40 13.22 L9.22 14.89 L6.78 14.89 L6.60 13.22 L5.30 12.68 L3.98 13.73 L2.27 12.02 L3.32 10.70 L2.78 9.40 L1.11 9.22 L1.11 6.78 L2.78 6.60 L3.32 5.30 L2.27 3.98 L3.98 2.27 L5.30 3.32Z" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linejoin="round"/>"#,
        r#"<circle cx="8" cy="8" r="2.1" fill="none" stroke="currentColor" stroke-width="1.25"/>"#
    ),
};
