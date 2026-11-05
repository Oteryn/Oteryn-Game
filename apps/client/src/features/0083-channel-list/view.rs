use crate::play::PlayView;
impl PlayView {
    pub fn served_channel_text(&self) -> Option<String> {
        self.served_channel
            .map(oteryn_client_ui::channel_list::channel_text)
    }
}
