use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::vip::{Intent, request};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn account_friends_open(&self) {
        submit(&mut lock(&self.mailbox), &self.commands, 0);
        #[cfg(windows)]
        open(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).vip.account_open = true;
        }
    }
    pub fn account_friends_control(&self, action: usize, fields: [String; 2]) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.vip.pending {
            return;
        }
        mailbox.vip.account_fields = fields;
        submit(&mut mailbox, &self.commands, action);
    }
}
fn submit(mailbox: &mut Mailbox, commands: &UnboundedSender<PlayCommand>, action: usize) {
    let fields = &mailbox.vip.account_fields;
    let handle = fields[1].trim().parse::<u64>().ok().filter(|n| *n > 0);
    let intent = match action {
        0 => Some(Intent::FriendsQuery(0)),
        1 => Some(Intent::MainCharacter),
        2 => Some(Intent::FriendInvite(fields[0].clone())),
        3 => handle.map(Intent::FriendAccept),
        4 => handle.map(Intent::FriendRevoke),
        5 => handle.map(Intent::FriendReject),
        6 => mailbox
            .vip
            .account_view
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| value["next_friend"].as_u64())
            .filter(|n| *n > 0)
            .map(Intent::FriendsQuery),
        _ => None,
    };
    if let Some(intent) = intent {
        if let Some(command) = request(&mut mailbox.vip, intent)
            && commands.send(PlayCommand::Vip(command)).is_err()
        {
            mailbox.vip.pending = false;
            mailbox.vip.feedback = Some("Session ended".into());
        }
    } else {
        mailbox.vip.feedback = Some("Select a valid friend handle or page".into());
    }
}
#[cfg(any(windows, test))]
fn text(mailbox: &Mailbox) -> String {
    let mut text = String::from(
        "Account Friends — current World\r\nAccepting shares your chosen main character with this account.\r\n",
    );
    if let Some(value) = mailbox
        .vip
        .account_view
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
    {
        text.push_str(&format!(
            "My main character: {}\r\n",
            value["main_character"].as_str().unwrap_or("not selected")
        ));
        for row in value["account_friends"].as_array().into_iter().flatten() {
            text.push_str(&format!(
                "#{} {} — {} — main: {}\r\n",
                row["handle"],
                row["name"].as_str().unwrap_or(""),
                row["state"].as_str().unwrap_or(""),
                row["main_character"].as_str().unwrap_or("not shared")
            ));
        }
    }
    if let Some(feedback) = &mailbox.vip.feedback {
        text.push_str(feedback);
    }
    text
}
#[cfg(windows)]
fn open(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Account Friends — Ctrl+Alt+F",
            labels: &["Character name to invite", "Session friend handle"],
            buttons: &[
                "Refresh",
                "Use this character as main",
                "Invite account",
                "Accept account friendship",
                "Withdraw / revoke",
                "Reject account invitation",
                "Next page",
            ],
            snapshot: |mailbox| {
                (
                    mailbox.vip.account_open,
                    mailbox.vip.pending,
                    mailbox.vip.account_fields.to_vec(),
                    text(mailbox),
                )
            },
            set_open: |mailbox, value| mailbox.vip.account_open = value,
            submit: |mailbox, commands, action, values| {
                if mailbox.vip.pending {
                    return;
                }
                for (slot, value) in mailbox.vip.account_fields.iter_mut().zip(values) {
                    *slot = value;
                }
                submit(mailbox, commands, action);
            },
        },
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn account_friends_caller_sends_explicit_consent_and_main_selection_without_optimistic_state()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, mut commands, outbox) = crate::play::play_channel();
        outbox.vip_initialize(true);
        link.account_friends_open();
        let PlayCommand::Vip(query) = commands.try_recv()? else {
            return Err("query".into());
        };
        assert_eq!(
            oteryn_client_ui::vip::vip::decode_request(query.kind, &query.payload)
                .map_err(|_| "query wire")?
                .operation,
            oteryn_client_ui::vip::vip::Operation::FriendsQuery
        );
        lock(&outbox.0).vip.pending = false;
        link.account_friends_control(1, [String::new(), String::new()]);
        let PlayCommand::Vip(main) = commands.try_recv()? else {
            return Err("main".into());
        };
        assert_eq!(main.payload, [8, 13]);
        assert!(lock(&outbox.0).vip.account_view.is_none());
        lock(&outbox.0).vip.pending = false;
        link.account_friends_control(3, [String::new(), "7".into()]);
        let PlayCommand::Vip(accept) = commands.try_recv()? else {
            return Err("accept".into());
        };
        let request = oteryn_client_ui::vip::vip::decode_request(accept.kind, &accept.payload)
            .map_err(|_| "accept wire")?;
        assert_eq!(
            request.operation,
            oteryn_client_ui::vip::vip::Operation::FriendAccept
        );
        assert_eq!(request.number, 7);
        assert!(text(&lock(&outbox.0)).contains("Accepting shares"));
        let response = oteryn_client_runtime::vip::VipResponse {
            status: oteryn_session::CommandStatus::Accepted,
            result: oteryn_client_ui::vip::vip::ResultView {
                code: 1,
                revision: 0,
                number: 0,
                wait: 0,
                view: r#"{"account_friends":[],"main_character":"Alice","next_friend":0}"#.into(),
            },
        };
        let refresh = crate::play::feature_0043_vip::protocol::refresh_after(&accept, &response)
            .ok_or("account refresh")?;
        assert_eq!(
            oteryn_client_ui::vip::vip::decode_request(refresh.kind, &refresh.payload)
                .map_err(|_| "refresh wire")?
                .operation,
            oteryn_client_ui::vip::vip::Operation::FriendsQuery
        );
        outbox.vip_result(&response);
        assert!(lock(&outbox.0).vip.view.is_none());
        assert!(text(&lock(&outbox.0)).contains("My main character: Alice"));

        Ok(())
    }
}
