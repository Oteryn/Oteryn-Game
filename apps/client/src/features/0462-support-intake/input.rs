use crate::play::PlayLink;
impl PlayLink {
    /// Queue only evidence previously approved in the native consent dialogs.
    pub fn queue_support_at(&self, directory: &std::path::Path) {
        crate::support_delivery::queue_support_at(&self.commands, directory);
    }
}
