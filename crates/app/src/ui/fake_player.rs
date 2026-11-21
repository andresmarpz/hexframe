use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

use rune::{El, PlayerState};

use super::review::ReviewPlayer;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Call {
    Open(PathBuf),
    Play,
    Pause,
    Seek(f64),
}

thread_local! {
    static CALLS: RefCell<Vec<Call>> = const { RefCell::new(Vec::new()) };
    static TIME: Cell<f64> = const { Cell::new(0.0) };
}

pub(crate) fn set_time(secs: f64) {
    TIME.set(secs);
}

pub(crate) fn calls() -> Vec<Call> {
    CALLS.with_borrow(Clone::clone)
}

pub(crate) fn clear_calls() {
    CALLS.with_borrow_mut(Vec::clear);
}

fn record(call: Call) {
    CALLS.with_borrow_mut(|calls| calls.push(call));
}

#[derive(Debug)]
pub(crate) struct FakePlayer;

pub(crate) fn factory(path: &Path) -> Box<dyn ReviewPlayer> {
    record(Call::Open(path.to_owned()));
    Box::new(FakePlayer)
}

impl ReviewPlayer for FakePlayer {
    fn play(&self) {
        record(Call::Play);
    }

    fn pause(&self) {
        record(Call::Pause);
    }

    fn seek(&self, secs: f64) {
        record(Call::Seek(secs));
    }

    fn time(&self) -> f64 {
        TIME.get()
    }

    fn duration(&self) -> Option<f64> {
        Some(10.0)
    }

    fn state(&self) -> PlayerState {
        PlayerState::Ready
    }

    fn show(&self, frame: El) -> El {
        frame
    }
}
