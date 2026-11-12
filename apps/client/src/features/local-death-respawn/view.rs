use crate::play::PlayView;

impl PlayView {
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
