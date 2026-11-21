use super::Ui;

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
        if screen != Screen::Games {
            self.pause_review();
        }
        self.screen = screen;
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
