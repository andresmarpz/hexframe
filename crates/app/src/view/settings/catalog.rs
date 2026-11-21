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
    Audio,
    Cursor,
    NavStyle,
    ClipsEnabled,
    Moments,
    ClipWindow,
    ClipHotkey,
    ReviewEvents,
    ReviewReset,
    Folder,
    StorageLimit,
    Bind(Action),
    Version,
    Updates,
    AutoUpdates,
    Logs,
    Feedback,
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
            Self::Audio => "audio sound game audio microphone discord",
            Self::Cursor => "cursor mouse pointer",
            Self::NavStyle => "navigation style layout sidebar dock menu tabs",
            Self::ClipsEnabled => "save highlight clips auto clips",
            Self::Moments => {
                "moments kills deaths assists multikills objectives structures aces dragon baron herald turrets"
            }
            Self::ClipWindow => "clip window before after seconds length duration pre post",
            Self::ClipHotkey => "save clip hotkey shortcut manual keybind",
            Self::ReviewEvents => return events_terms(),
            Self::ReviewReset => "reset default review events filter restore recommended",
            Self::Folder => "recordings folder location path change open directory",
            Self::StorageLimit => "storage limit disk space gb usage used delete",
            Self::Bind(action) => return format!("{} keybind shortcut hotkey key", action.label()),
            Self::Version => "version greyscreen",
            Self::Updates => "updates update check install release",
            Self::AutoUpdates => "check for updates automatically auto update",
            Self::Logs => "logs folder report problem support",
            Self::Feedback => "send feedback report bug idea suggestion contact support",
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
        "review events shown show hide filter timeline {}",
        names.join(" ")
    )
}

pub(super) fn rows_in(section: SettingsSection) -> Vec<Row> {
    match section {
        SettingsSection::Recording => vec![
            Row::RecordGames,
            Row::FullGame,
            Row::Quality,
            Row::Resolution,
            Row::FrameRate,
            Row::Audio,
            Row::Cursor,
            Row::NavStyle,
        ],
        SettingsSection::Clips => vec![
            Row::ClipsEnabled,
            Row::Moments,
            Row::ClipWindow,
            Row::ClipHotkey,
        ],
        SettingsSection::Review => vec![Row::ReviewEvents, Row::ReviewReset],
        SettingsSection::Storage => vec![Row::Folder, Row::StorageLimit],
        SettingsSection::Keybinds => Action::ALL.iter().map(|a| Row::Bind(*a)).collect(),
        SettingsSection::About => {
            let mut rows = vec![
                Row::Version,
                Row::Updates,
                Row::AutoUpdates,
                Row::Logs,
                Row::Feedback,
            ];
            if platform::CAN_START_WITH_WINDOWS {
                rows.push(Row::StartWithWindows);
            }
            rows.push(Row::RiotNotice);
            rows
        }
    }
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
