use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn analyser_panel(&self) -> oteryn_client_ui::analyser::Panel {
        lock(&self.mailbox).analyser.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    crate::play::feature_0115_mail::view::native::open(link, form());
}

/// Auxiliary windows borrow the mailbox; only the shell's PlayLink owns session teardown.
#[cfg(any(windows, test))]
pub(in crate::play) fn open_shared(
    shared: &std::sync::Arc<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) -> bool {
    let ready = super::input::begin_shared_window(&mut lock(shared), commands);
    // Unit tests exercise the same mailbox transition without creating OS windows.
    #[cfg(all(windows, not(test)))]
    if ready {
        let weak = std::sync::Arc::downgrade(shared);
        let rollback = weak.clone();
        let commands = commands.clone();
        if std::thread::Builder::new()
            .name("Analysers-panel".into())
            .spawn(move || {
                crate::play::feature_0115_mail::view::native::run(weak, commands, form());
            })
            .is_err()
            && let Some(shared) = rollback.upgrade()
        {
            let mut mailbox = lock(&shared);
            mailbox.analyser.open = false;
            mailbox.analyser.feedback = Some("Could not open the analyser window");
            return false;
        }
    }
    ready
}
#[cfg(windows)]
fn form() -> crate::play::feature_0115_mail::view::native::Form {
    use crate::play::feature_0115_mail::view::native::Form;
    Form {
        title: "Analysers",
        labels: &[
            "XP still needed for your level goal (blank disables)",
            "Item or creature race number from analyser",
            "Unit item value (gold) or kill target",
            "Session export file path",
        ],
        buttons: &[
            "Overview",
            "Hunting",
            "Experience",
            "Loot",
            "Supplies",
            "Damage",
            "Healing",
            "Impact",
            "Input",
            "Hunting session",
            "Party hunt",
            "Loot tracker",
            "Kill tracker",
            "Reset session",
            "Apply level goal",
            "Apply item price",
            "Resolve item info",
            "Export (replace file)",
            "Refresh party hunt",
            "Party supply price",
            "Track loot item",
            "Untrack loot item",
            "Tracked / all loot",
            "Set race kill target",
            "Remove race target",
            "Refresh hunting tasks",
        ],
        snapshot: |mailbox| {
            let panel = &mailbox.analyser;
            (
                panel.open,
                false,
                vec![
                    panel.goal_input.clone(),
                    panel.price_input[0].clone(),
                    panel.price_input[1].clone(),
                    panel.export_path.clone(),
                ],
                oteryn_client_ui::analyser::panel_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.analyser.open = value,

        submit: super::input::submit,
    }
}
