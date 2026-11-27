use crossbeam_channel::Sender;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Phase {
    Ready,
    Closing,
    Copying,
    Done,
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cmd {
    Install { start_with_windows: bool },
    OpenInstalled,
    RunHere,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) enum Msg {
    Closing,
    Copying,
    Done,
    Failed(String),
}

pub(crate) struct SetupState {
    pub(super) phase: Phase,
    pub(super) start_with_windows: bool,
    pub(super) install_path: String,
    cmd: Option<Sender<Cmd>>,
}

impl SetupState {
    pub(crate) fn new(install_path: String, cmd: Option<Sender<Cmd>>) -> Self {
        Self {
            phase: Phase::Ready,
            start_with_windows: true,
            install_path,
            cmd,
        }
    }

    pub(crate) fn toggle_start_with_windows(&mut self) {
        if self.phase == Phase::Ready {
            self.start_with_windows = !self.start_with_windows;
        }
    }

    pub(crate) fn install(&mut self) {
        if matches!(self.phase, Phase::Ready | Phase::Failed(_)) {
            self.phase = Phase::Copying;
            self.send(Cmd::Install {
                start_with_windows: self.start_with_windows,
            });
        }
    }

    pub(crate) fn open_installed(&mut self) {
        if self.phase == Phase::Done {
            self.send(Cmd::OpenInstalled);
        }
    }

    pub(crate) fn run_here(&mut self) {
        if matches!(self.phase, Phase::Failed(_)) {
            self.send(Cmd::RunHere);
        }
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn apply(&mut self, msg: Msg) {
        self.phase = match (&self.phase, msg) {
            (Phase::Copying | Phase::Closing, Msg::Closing) => Phase::Closing,
            (Phase::Copying | Phase::Closing, Msg::Copying) => Phase::Copying,
            (Phase::Copying | Phase::Closing, Msg::Done) => Phase::Done,
            (_, Msg::Failed(reason)) => Phase::Failed(reason),
            _ => return,
        };
    }

    fn send(&self, cmd: Cmd) {
        if let Some(sender) = &self.cmd
            && sender.send(cmd).is_err()
        {
            tracing::warn!("setup worker stopped");
        }
    }
}
