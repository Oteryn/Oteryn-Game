use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn hireling_management_open(&self) {
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            native::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).hireling_management.open = true;
            self.hireling_management_request(oteryn_client_ui::hireling_management::Intent::List);
        }
    }
    pub fn hireling_management_panel(&self) -> oteryn_client_ui::hireling_management::Panel {
        lock(&self.mailbox).hireling_management.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;

/// The hireling management window: identity and outfit rows, colours, a new name and the
/// configuration purchases, each reviewed first and bought only on the Confirm press.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::play::feature_0115_mail::view::form::{Form, field};
    use oteryn_client_ui::hireling_management::{Intent, outfits, panel_text, request, select};
    Form {
        title: tr!("Hireling management"),
        labels: &[
            "Hireling number",
            "Outfit number",
            "Head colour",
            "Body colour",
            "Legs colour",
            "Feet colour",
            "New name",
        ],
        buttons: &[
            "Select hireling",
            "Save outfit",
            "Return to lamp",
            "Refresh",
            "Load prices",
            "Rename",
            "Change sex",
            "Unlock Banker",
            "Unlock Cook",
            "Unlock Steward",
            "Unlock Trader",
            "Confirm",
        ],
        snapshot: |mailbox| {
            let p = &mailbox.hireling_management;
            let mut fields = vec![String::new(); 7];
            if let Some((row, entry)) = p
                .entries
                .iter()
                .enumerate()
                .find(|(_, e)| Some(e.hireling) == p.identity)
            {
                fields[0] = (row + 1).to_string();
                fields[1] = outfits(entry)
                    .iter()
                    .position(|(_, look)| *look == entry.look_type)
                    .map_or_else(String::new, |at| (at + 1).to_string());
                for (slot, color) in fields[2..6].iter_mut().zip(entry.colors) {
                    *slot = color.to_string();
                }
                fields[6] = entry.name.clone();
            }
            let mut text = format!(
                "{}\n{}",
                panel_text(p),
                oteryn_client_ui::store::panel_text(&mailbox.store)
            );
            for (row, entry) in p.entries.iter().enumerate() {
                text.push_str(&format!("\n{}. {}", row + 1, entry.name));
                if Some(entry.hireling) == p.identity {
                    for (at, (name, _)) in outfits(entry).iter().enumerate() {
                        text.push_str(&format!("\n   {}. {name}", at + 1));
                    }
                }
            }
            if let Some(review) = &mailbox.hireling_review {
                text.push_str("\n\n");
                text.push_str(&review.text());
                text.push_str("\nPress Confirm to buy.");
            }
            (
                p.open && !mailbox.hireling_shutdown,
                p.pending.is_some() || mailbox.store.pending,
                fields,
                text,
            )
        },
        set_open: |mailbox, value| {
            mailbox.hireling_management.open = value;
            mailbox.hireling_review = None;
        },
        submit: |mailbox, commands, action, values| {
            use super::input::configuration;
            let review = mailbox.hireling_review.take();
            match action {
                4 => {
                    configuration::prices(mailbox, commands);
                    return;
                }
                5..=10 => {
                    match configuration::review(mailbox, action + 126, field(&values, 6)) {
                        Ok(review) => mailbox.hireling_review = Some(review),
                        Err(message) => mailbox.hireling_management.feedback = Some(message),
                    }
                    return;
                }
                11 => {
                    match review {
                        Some(review) => {
                            configuration::submit(mailbox, commands, &review, true);
                        }
                        None => {
                            mailbox.hireling_management.feedback =
                                Some("Choose a configuration to review first");
                        }
                    }
                    return;
                }
                _ => {}
            }
            let p = &mut mailbox.hireling_management;
            let row = field(&values, 0)
                .parse::<usize>()
                .ok()
                .and_then(|row| row.checked_sub(1));
            let intent = match action {
                0 => {
                    if let Some(row) = row {
                        select(p, row);
                    }
                    None
                }
                1 => {
                    let look = p
                        .entries
                        .iter()
                        .find(|e| Some(e.hireling) == p.identity)
                        .and_then(|e| {
                            field(&values, 1)
                                .parse::<usize>()
                                .ok()
                                .and_then(|at| at.checked_sub(1))
                                .and_then(|at| outfits(e).get(at).map(|(_, look)| *look))
                        });
                    let colors = [2, 3, 4, 5].map(|at| field(&values, at).parse::<u8>().ok());
                    match (look, colors) {
                        (Some(look_type), [Some(a), Some(b), Some(c), Some(d)]) => {
                            Some(Intent::Outfit {
                                look_type,
                                colors: [a, b, c, d],
                            })
                        }
                        _ => {
                            p.feedback = Some(tr!(
                                "Select a hireling and outfit; enter four colors from 0 to 255"
                            ));
                            None
                        }
                    }
                }
                2 => Some(Intent::Return),
                3 => Some(Intent::List),
                _ => None,
            };
            if let Some(intent) = intent
                && let Some(command) = request(p, intent)
                && commands
                    .send(crate::combat_input::PlayCommand::Feature(command))
                    .is_err()
            {
                p.pending = None;
                p.feedback = Some(tr!("The session has ended"));
            }
        },
    }
}
