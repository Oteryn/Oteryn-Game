use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn exiva_panel(&self) -> oteryn_client_ui::exiva_settings::ExivaPanel {
        lock(&self.mailbox).exiva.clone()
    }
}

/// Exiva permissions for the shared frontend; a mode button toggles that one permission.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::combat_input::PlayCommand;
    use oteryn_client_ui::exiva_settings::{panel_text, request};
    const MODES: [&str; 5] = [
        "Allow everyone",
        "Allow player list",
        "Allow own guild",
        "Allow own party",
        "Allow VIP list",
    ];
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Exiva permissions"),
        labels: &["Player name to add", "Player row to remove (from 1)"],
        buttons: &[
            "Add name",
            "Remove selected",
            "Refresh",
            "Allow everyone",
            "Allow player list",
            "Allow own guild",
            "Allow own party",
            "Allow VIP list",
        ],
        snapshot: |mailbox| {
            let panel = &mailbox.exiva;
            let mut text = String::new();
            if let Some(list) = &panel.list {
                for (index, (_, name)) in list.players.iter().enumerate() {
                    text.push_str(&format!("{}. {name}\n", index + 1));
                }
                for (bit, mode) in MODES.iter().enumerate() {
                    let mark = if list.modes & (1 << bit) != 0 { "[x]" } else { "[ ]" };
                    text.push_str(&format!("{mark} {mode}\n"));
                }
            }
            text.push_str(&panel_text(panel));
            (
                panel.open && !mailbox.quest_shutdown,
                // Refresh stays usable after a refused read; the other buttons need the list.
                panel.pending || !panel.selected,
                vec![String::new(); 2],
                text,
            )
        },
        set_open: |mailbox, value| mailbox.exiva.open = value,
        submit: |mailbox, commands, action, values| {
            use crate::play::feature_0115_mail::view::form::field;
            if action != 2 && mailbox.exiva.list.is_none() {
                mailbox.exiva.error = Some(tr!("Exiva request refused; refresh the list"));
                return;
            }
            let payload = match action {
                0 => {
                    let mut payload = vec![6];
                    let name: String = field(&values, 0).chars().take(29).collect();
                    payload.extend(name.as_bytes());
                    Some(payload)
                }
                1 => {
                    let payload = field(&values, 1)
                        .parse::<usize>()
                        .ok()
                        .and_then(|row| row.checked_sub(1))
                        .and_then(|row| mailbox.exiva.list.as_ref()?.players.get(row))
                        .map(|(id, _)| {
                            let mut bytes = vec![2];
                            bytes.extend(id);
                            bytes
                        });
                    if payload.is_none() {
                        mailbox.exiva.error = Some(tr!("Enter a player row from the list"));
                    }
                    payload
                }
                2 => Some(vec![5]),
                3..=7 => mailbox
                    .exiva
                    .list
                    .as_ref()
                    .map(|list| vec![0, list.modes ^ (1 << (action - 3))]),
                _ => None,
            };
            if let Some(payload) = payload
                && let Some(payload) = request(&mut mailbox.exiva, payload)
                && commands.send(PlayCommand::Exiva(payload)).is_err()
            {
                mailbox.exiva.pending = false;
                mailbox.exiva.error = Some(tr!("Session ended"));
            }
        },
    }
}
