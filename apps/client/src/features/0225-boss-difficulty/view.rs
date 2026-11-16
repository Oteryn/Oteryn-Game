use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::boss_difficulty::BossDifficultyPanel;
impl PlayLink {
    #[must_use]
    pub fn boss_difficulty_panel(&self) -> BossDifficultyPanel {
        lock(&self.mailbox).boss_difficulty.clone()
    }
}
#[path = "native.rs"]
mod native;

/// Boss difficulty selection for the shared frontend; the difficulty list is a number field.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use oteryn_client_ui::boss_difficulty::{Intent, panel_text};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Boss difficulty"),
        labels: &["Boss id", "Server context (automatic)", "Difficulty"],
        buttons: &["Open", "Preview", "Start", "Cancel"],
        snapshot: |mailbox| {
            let panel = &mailbox.boss_difficulty;
            let levels = native::difficulty_levels(panel.reply.as_ref());
            let context = panel
                .context
                .map(|id| id.iter().map(|byte| format!("{byte:02x}")).collect::<String>())
                .unwrap_or_default();
            let text = format!(
                "Difficulty {}-{}\n{}",
                levels.start(),
                levels.end(),
                panel_text(panel)
            );
            (
                panel.open,
                panel.pending,
                vec![panel.boss_key.clone(), context, panel.difficulty.to_string()],
                text,
            )
        },
        set_open: |mailbox, value| mailbox.boss_difficulty.open = value,
        submit: |mailbox, commands, action, values| {
            use crate::play::feature_0115_mail::view::form::field;
            let intent = match action {
                0 => Intent::Open,
                1 => Intent::Preview,
                2 => Intent::Start,
                3 => Intent::Cancel,
                _ => return,
            };
            if let Some(command) = native::submit(
                &mut mailbox.boss_difficulty,
                intent,
                field(&values, 0),
                field(&values, 1),
                field(&values, 2),
            ) && commands
                .send(crate::combat_input::PlayCommand::Feature(command))
                .is_err()
            {
                mailbox.boss_difficulty.pending = false;
                mailbox.boss_difficulty.feedback = Some(tr!("The session has ended"));
            }
        },
    }
}
