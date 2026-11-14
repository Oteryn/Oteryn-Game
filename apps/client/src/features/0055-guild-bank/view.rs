use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn guild_bank_panel(&self) -> oteryn_client_ui::guild_bank::Panel {
        lock(&self.mailbox).properties.guild_bank.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Guild bank",
            labels: &["Guild UUID", "Amount"],
            buttons: &[
                "Refresh",
                "Deposit",
                "Withdraw",
                "Claims",
                "Claim",
                "Older",
                "MoreClaims",
            ],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.guild_bank;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::guild_bank::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.guild_bank.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.guild_bank;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::guild_bank::Intent::Refresh,
                    1 => oteryn_client_ui::guild_bank::Intent::Deposit,
                    2 => oteryn_client_ui::guild_bank::Intent::Withdraw,
                    3 => oteryn_client_ui::guild_bank::Intent::Claims,
                    4 => oteryn_client_ui::guild_bank::Intent::Claim,
                    5 => oteryn_client_ui::guild_bank::Intent::Older,
                    6 => oteryn_client_ui::guild_bank::Intent::MoreClaims,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[cfg(feature = "bevy-shell")]
#[path = "view/bound.rs"]
mod bound;
#[cfg(feature = "bevy-shell")]
pub(crate) use bound::{GuildBankControl, GuildBankView};
