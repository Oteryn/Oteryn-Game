use crate::play::PlayLink;
impl PlayLink {
    pub fn discovery_action(
        &self,
        panel: u32,
        action: usize,
        fields: &[String],
        row: usize,
    ) -> bool {
        let accepted = match panel {
            98 => self.quick_loot_control(action, fields, row),
            132 => self.inspection_control(action, fields, row),
            232 => self.team_finder_control(action, fields, row),
            285 => match action {
                1 | 2
                    if self.store_panel().page
                        == oteryn_client_ui::store::StorePage::CoinHistory =>
                {
                    self.coin_history(action == 2)
                }
                1 => self.purchase_history(false),
                2 => self.purchase_history(true),
                6 => self.coin_history(false),
                _ => false,
            },
            _ => false,
        };
        if !accepted {
            let mut mailbox = crate::play::mailbox::lock(&self.mailbox);
            let feedback = match panel {
                98 => &mut mailbox.discovery.loot.feedback,
                132 => &mut mailbox.discovery.inspection.feedback,
                232 => &mut mailbox.discovery.teams.feedback,
                285 => &mut mailbox.discovery.history.feedback,
                _ => return false,
            };
            *feedback =
                tr!("Action unavailable: check selection, details, or pending request").into();
        }
        accepted
    }
}
