mod death;
mod dock;
mod errors;
mod export;
mod feedback;
mod games;
mod home;
mod notices;
mod review;
mod settings;
mod update;

use std::sync::OnceLock;

use greyscreen_core::encoder::{CheckOutcome, EncoderCheck};
use greyscreen_core::settings::NavStyle;
use greyscreen_engine::EngineStatus;

use super::{Screen, SettingsSection, Ui, preview};

type Scene = (&'static str, fn(&mut Ui));

pub(crate) fn all() -> &'static [Scene] {
    static ALL: OnceLock<Vec<Scene>> = OnceLock::new();
    ALL.get_or_init(|| {
        [
            home::SCENES,
            notices::SCENES,
            games::SCENES,
            dock::SCENES,
            errors::SCENES,
            export::SCENES,
            review::SCENES,
            settings::SCENES,
            update::SCENES,
            feedback::SCENES,
        ]
        .concat()
    })
}

fn idle(ui: &mut Ui) {
    ui.status = EngineStatus::Idle;
    ui.feed.clear();
}

fn home(ui: &mut Ui) {
    idle(ui);
    ui.go(Screen::Home);
}

fn first_run(ui: &mut Ui) {
    idle(ui);
    ui.recordings.clear();
}

fn game(ui: &mut Ui) {
    idle(ui);
    ui.open_review_entry(preview::review_entry());
}

fn sheet(ui: &mut Ui, section: SettingsSection) {
    game(ui);
    ui.open_settings(section);
}

fn encoder(outcome: CheckOutcome, name: &str) -> Option<EncoderCheck> {
    Some(EncoderCheck {
        key: String::new(),
        outcome,
        encoder: name.to_owned(),
        detail: String::new(),
        noticed: true,
        verdicts: Vec::new(),
    })
}

fn docked(ui: &mut Ui) {
    idle(ui);
    ui.settings.nav_style = NavStyle::Dock;
}
