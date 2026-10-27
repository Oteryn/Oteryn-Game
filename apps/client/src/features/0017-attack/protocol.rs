use crate::combat_input::CombatReport;
use crate::play::PlayView;

impl PlayView {
    /// Shows the server's answer to a combat command.
    pub fn apply_combat(&mut self, report: CombatReport) {
        self.combat_line = report.text();
        self.logout_line = None;
    }
}
