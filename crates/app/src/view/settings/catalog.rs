use greyscreen_core::categories::EventCategory;
use greyscreen_core::keybinds::Action;

use crate::platform;
use crate::ui::SettingsSection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Row {
    RecordGames,
    FullGame,
    Quality,
    Resolution,
    FrameRate,
    Encoder,
    Audio,
    Cursor,
    NavStyle,
    SoundEffects,
    SoundVolume,
    Diagnostics,
    ClipsEnabled,
    Moments,
    ClipWindow,
    ClipHotkey,
    ReviewEvents,
    Folder,
    StorageLimit,
    Bind(Action),
    Version,
    Updates,
    AutoUpdates,
    Logs,
    StartWithWindows,
    RiotNotice,
}

impl Row {
    fn terms(self) -> String {
        let fixed = match self {
            Self::RecordGames => "record games automatic auto start recording league",
            Self::FullGame => "record the full game highlights only",
            Self::Quality => "quality balanced high max size disk bitrate estimate mb",
            Self::Resolution => "resolution 1080p 1440p native scale size",
            Self::FrameRate => "frame rate fps 30 60 smooth",
            Self::Encoder => {
                "encoder video hardware software gpu cpu nvidia nvenc amd amf intel quick sync"
            }
            Self::Audio => "audio sound game audio microphone discord",
            Self::Cursor => "cursor mouse pointer",
            Self::NavStyle => "navigation style layout sidebar dock menu tabs",
            Self::SoundEffects => {
                "sound effects sounds noise chime click beep notification mute quiet"
            }
            Self::SoundVolume => "sound effects volume loud quiet level sounds",
            Self::Diagnostics => {
                "detailed recording diagnostics debug health performance log frame drops stutter analyze"
            }
            Self::ClipsEnabled => "save highlight clips auto clips",
            Self::Moments => {
                "moments kills deaths assists multikills objectives structures aces dragon baron herald turrets"
            }
            Self::ClipWindow => "clip window before after seconds length duration pre post",
            Self::ClipHotkey => "save clip hotkey shortcut manual keybind",
            Self::ReviewEvents => return events_terms(),
            Self::Folder => "recordings folder location path change open directory",
            Self::StorageLimit => "storage limit disk space gb usage used delete",
            Self::Bind(action) => return format!("{} keybind shortcut hotkey key", action.label()),
            Self::Version => "version greyscreen",
            Self::Updates => "updates update check install release",
            Self::AutoUpdates => "check for updates automatically auto update",
            Self::Logs => "logs folder report problem support",
            Self::StartWithWindows => "start with windows startup login tray",
            Self::RiotNotice => "riot games notice legal trademark endorsed",
        };
        fixed.to_owned()
    }

    fn matches(self, query: &str) -> bool {
        let terms = self.terms().to_lowercase();
        query
            .split_whitespace()
            .all(|word| terms.contains(&word.to_lowercase()))
    }
}

fn events_terms() -> String {
    let names: Vec<&str> = EventCategory::ALL
        .iter()
        .map(|category| category.search_terms())
        .collect();
    format!(
        "review events shown show hide filter timeline reset default {}",
        names.join(" ")
    )
}

pub(super) struct SectionGroup {
    pub id: &'static str,
    pub title: Option<&'static str>,
    pub rows: Vec<Row>,
}

pub(super) fn groups_in(section: SettingsSection) -> Vec<SectionGroup> {
    match section {
        SettingsSection::General => {
            let mut app_rows = Vec::new();
            if platform::CAN_START_WITH_WINDOWS {
                app_rows.push(Row::StartWithWindows);
            }
            app_rows.push(Row::NavStyle);
            vec![
                SectionGroup {
                    id: "app",
                    title: Some("App"),
                    rows: app_rows,
                },
                SectionGroup {
                    id: "sound",
                    title: Some("Sound"),
                    rows: vec![Row::SoundEffects, Row::SoundVolume],
                },
                SectionGroup {
                    id: "updates",
                    title: Some("Updates"),
                    rows: vec![Row::Updates, Row::AutoUpdates],
                },
                SectionGroup {
                    id: "diagnostics",
                    title: Some("Diagnostics"),
                    rows: vec![Row::Diagnostics, Row::Logs],
                },
                SectionGroup {
                    id: "about",
                    title: Some("About"),
                    rows: vec![Row::Version, Row::RiotNotice],
                },
            ]
        }
        SettingsSection::Recording => vec![
            SectionGroup {
                id: "capture",
                title: Some("Capture"),
                rows: vec![Row::RecordGames, Row::FullGame],
            },
            SectionGroup {
                id: "video",
                title: Some("Video"),
                rows: vec![Row::Quality, Row::Resolution, Row::FrameRate, Row::Encoder],
            },
            SectionGroup {
                id: "audio",
                title: Some("Audio & input"),
                rows: vec![Row::Audio, Row::Cursor],
            },
        ],
        SettingsSection::Clips => vec![
            SectionGroup {
                id: "clips",
                title: Some("Highlight clips"),
                rows: vec![
                    Row::ClipsEnabled,
                    Row::Moments,
                    Row::ClipWindow,
                    Row::ClipHotkey,
                ],
            },
            SectionGroup {
                id: "review",
                title: Some("Review timeline"),
                rows: vec![Row::ReviewEvents],
            },
        ],
        SettingsSection::Storage => vec![SectionGroup {
            id: "storage",
            title: None,
            rows: vec![Row::Folder, Row::StorageLimit],
        }],
        SettingsSection::Keybinds => Vec::new(),
    }
}

pub(super) fn rows_in(section: SettingsSection) -> Vec<Row> {
    if section == SettingsSection::Keybinds {
        return Action::ALL.iter().map(|a| Row::Bind(*a)).collect();
    }
    groups_in(section)
        .into_iter()
        .flat_map(|g| g.rows)
        .collect()
}

pub(super) fn search(query: &str) -> Vec<(SettingsSection, Vec<Row>)> {
    SettingsSection::ALL
        .iter()
        .filter_map(|section| {
            let rows: Vec<Row> = rows_in(*section)
                .into_iter()
                .filter(|row| row.matches(query))
                .collect();
            (!rows.is_empty()).then_some((*section, rows))
        })
        .collect()
}
