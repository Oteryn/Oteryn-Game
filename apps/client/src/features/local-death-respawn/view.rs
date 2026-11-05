use crate::play::PlayView;

impl PlayView {
    pub(in crate::play) fn death_notice(&self) -> Option<&'static str> {
        self.death
            .dead
            .then_some("You are dead. Waiting for automatic respawn at your temple.")
    }
}
