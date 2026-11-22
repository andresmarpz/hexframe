use crate::ui::Ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Screen {
    #[default]
    Home,
    Games,
    Clips,
}

impl Ui {
    pub(crate) fn go(&mut self, screen: Screen) {
        if self.in_theater() {
            return;
        }
        let entering_games = screen == Screen::Games && self.screen != Screen::Games;
        if screen != Screen::Games {
            self.pause_review();
        }
        self.screen = screen;
        if entering_games
            && self.review.is_none()
            && !self.live_selected
            && let Some(index) = self.visible_games().first().copied()
        {
            let entry = self.recordings[index].clone();
            self.open_review_entry(entry);
        }
    }

    pub(crate) fn clip_count(&self) -> usize {
        self.recordings.iter().map(|rec| rec.meta.clips.len()).sum()
    }

    fn pause_review(&mut self) {
        if let Some(review) = &mut self.review {
            review.pause();
        }
    }
}
