use crate::play::PlayView;

impl PlayView {
    /// Server-confirmed death notice for shared native frontends.
    pub fn death_modal_notice(&self) -> Option<String> {
        self.death
            .awaiting_acknowledgement()
            .then(|| self.death_notice())
            .flatten()
    }

    pub(in crate::play) fn death_notice(&self) -> Option<String> {
        if self.death.awaiting_acknowledgement() {
            let progress = if self.death.respawned() {
                tr!("You have returned to your temple. Press Enter to acknowledge and continue.")
            } else {
                tr!("Waiting for the server to return you to your temple.")
            };
            let protection = self.death.current_protection().map_or_else(
                || tr!("Protection details are unavailable for this death.").into(),
                oteryn_client_ui::death_protection::notice,
            );
            Some(lfmt!(
                "You are dead.\n{}\n{}\nExact experience, skill and item losses are unavailable.",
                progress,
                protection
            ))
        } else {
            self.death
                .protection
                .as_ref()
                .map(oteryn_client_ui::death_protection::notice)
        }
    }
}
