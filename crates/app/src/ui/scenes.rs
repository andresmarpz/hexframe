mod death;
mod dock;
mod errors;
mod export;
mod feedback;
mod game_page;
mod games;
mod home;
mod icons;
mod notices;
mod review;
mod settings;
mod shell;
mod update;

use std::sync::OnceLock;

use greyscreen_core::encoder::{CheckOutcome, EncoderCheck};
use greyscreen_engine::EngineStatus;

use super::{Screen, SettingsSection, Ui, preview};

type Scene = (&'static str, fn(&mut Ui));

pub(crate) fn all() -> &'static [Scene] {
    static ALL: OnceLock<Vec<Scene>> = OnceLock::new();
    ALL.get_or_init(|| {
        [
            home::SCENES,
            dock::SCENES,
            icons::SCENES,
            notices::SCENES,
            games::SCENES,
            game_page::SCENES,
            errors::SCENES,
            export::SCENES,
            review::SCENES,
            settings::SCENES,
            shell::SCENES,
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
        first_failure: String::new(),
        noticed: true,
        verdicts: Vec::new(),
    })
}
