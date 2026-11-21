use crate::play::PlayView;
impl PlayView {
    pub fn notification_text(&self, popup: bool) -> String {
        self.game_notifications
            .text_at(popup, std::time::Instant::now())
    }
}
