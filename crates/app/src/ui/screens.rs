use super::Ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Screen {
    #[default]
    Home,
    Games,
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

    fn pause_review(&self) {
        if let Some(review) = &self.review
            && review.is_playing()
        {
            review.toggle();
        }
    }
}
