// Ported from OBS Studio plugins/win-capture/window-capture.c (the previously_failed flag).
// Copyright (C) OBS Studio contributors. Licensed under GPL-2.0-or-later.

#[cfg(test)]
mod tests;

#[derive(Debug, Default, Clone, Copy)]
pub struct LogOnce {
    failing: bool,
}

impl LogOnce {
    pub const fn new() -> Self {
        Self { failing: false }
    }

    pub fn failure(&mut self) -> bool {
        !std::mem::replace(&mut self.failing, true)
    }

    pub fn success(&mut self) {
        self.failing = false;
    }

    pub fn is_failing(&self) -> bool {
        self.failing
    }
}
