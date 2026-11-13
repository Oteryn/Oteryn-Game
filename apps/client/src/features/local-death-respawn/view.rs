use crate::play::PlayView;

impl PlayView {
    /// Server-confirmed death notice for shared native frontends.
    pub fn death_modal_notice(&self) -> Option<String> {
        self.death.dead.then(|| self.death_notice()).flatten()
    }

    pub(in crate::play) fn death_notice(&self) -> Option<String> {
        if self.death.dead {
            Some(tr!("You are dead. Waiting for automatic respawn at your temple.").into())
        } else {
            self.death
                .protection
                .as_ref()
                .map(oteryn_client_ui::death_protection::notice)
        }
    }
}
