//! Atomic Bevy bindings over the retained house owners and their existing request builders.
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::house::{
    self,
    sale::{Confirmation, OwnershipPanel},
};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone, Debug)]
pub struct HouseBrowsingSnapshot {
    identity: Weak<Mutex<Mailbox>>,
    pub model: house::Panel,
    pub fields: [String; 7],
    pub ownership: OwnershipPanel,
    pub ownership_open: bool,
    pub ownership_fields: [String; 3],
    pub previous: bool,
    cursor: String,
    history: Vec<String>,
}
impl PartialEq for HouseBrowsingSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.identity.ptr_eq(&other.identity)
            && self.model == other.model
            && self.fields == other.fields
            && self.ownership == other.ownership
            && self.ownership_open == other.ownership_open
            && self.ownership_fields == other.ownership_fields
            && self.cursor == other.cursor
            && self.history == other.history
    }
}
impl Eq for HouseBrowsingSnapshot {}
impl HouseBrowsingSnapshot {
    fn capture(link: &PlayLink, mailbox: &Mailbox) -> Self {
        let panel = &mailbox.properties.house;
        Self {
            identity: Arc::downgrade(&link.mailbox),
            model: panel.model.clone(),
            fields: panel.fields.clone(),
            ownership: panel.ownership.model.clone(),
            ownership_open: panel.ownership.open,
            ownership_fields: panel.ownership.fields.clone(),
            previous: !panel.ownership.history.is_empty(),
            cursor: panel.ownership.cursor.clone(),
            history: panel.ownership.history.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HouseBrowsingAction {
    Request {
        selection: usize,
        fields: [String; 7],
    },
    Map {
        fields: [String; 7],
    },
    OwnershipOpen,
    OwnershipPage(i8),
    OwnershipSelect(usize),
    OwnershipReview {
        action: u8,
        fields: [String; 3],
    },
    OwnershipConfirm {
        review: Confirmation,
        accepted: bool,
    },
    OwnershipClose,
    Close,
}
impl PlayLink {
    pub fn house_browsing_snapshot(&self) -> HouseBrowsingSnapshot {
        HouseBrowsingSnapshot::capture(self, &lock(&self.mailbox))
    }
    /// Revalidates identity, target, revision, pending state and server modal under one lock.
    /// All outgoing packets are built by the same owners used by the native frontend.
    pub fn house_browsing_dispatch_bound(
        &self,
        expected: &HouseBrowsingSnapshot,
        action: HouseBrowsingAction,
    ) -> Result<Option<Confirmation>, String> {
        use super::super::input;
        let mut mailbox = lock(&self.mailbox);
        if matches!(
            action,
            HouseBrowsingAction::Close | HouseBrowsingAction::OwnershipClose
        ) {
            if !expected.identity.ptr_eq(&Arc::downgrade(&self.mailbox))
                || self.commands.is_closed()
                || mailbox.inventory_shutdown
                || mailbox.custom_hotkeys_ended
            {
                return Err("The session has ended".into());
            }
            if action == HouseBrowsingAction::Close {
                mailbox.properties.house.model.open = false;
            }
            mailbox.properties.house.ownership.open = false;
            return Ok(None);
        }

        if self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.activity_reset_commands
            || mailbox.inventory_shutdown
            || mailbox.server_modal.is_active()
            || HouseBrowsingSnapshot::capture(self, &mailbox) != *expected
            || !mailbox.properties.house.model.open
            || (!matches!(action, HouseBrowsingAction::Close)
                && (!mailbox.properties.house.model.selected
                    || mailbox.properties.house.model.pending.is_some()
                    || mailbox.properties.house.ownership.model.pending()))
        {
            return Err("The house selection or session changed; refresh before retrying".into());
        }
        let original = mailbox.properties.house.clone();
        let result = (|| {
            let panel = &mut mailbox.properties.house;
            match action {
                HouseBrowsingAction::Close => {
                    panel.model.open = false;
                    panel.ownership.open = false;
                    panel.ownership.model.cancel_review();
                    Ok(None)
                }
                HouseBrowsingAction::OwnershipClose => {
                    panel.ownership.open = false;
                    panel.ownership.model.cancel_review();
                    Ok(None)
                }
                HouseBrowsingAction::Request { selection, fields } => {
                    if matches!(selection,8..=10|14..) || panel.ownership.open {
                        return Err("Choose a house action".into());
                    }
                    validate_fields(&fields)?;
                    panel.fields = fields;
                    let command = input::prepare(panel, selection)?;
                    self.commands
                        .send(crate::combat_input::PlayCommand::Feature(command))
                        .map_err(|_| "The session has ended".to_owned())?;
                    Ok(None)
                }
                HouseBrowsingAction::Map { fields } => {
                    validate_fields(&fields)?;
                    if fields[..2] != panel.fields[..2] {
                        return Err("Query houses before viewing their entrances".into());
                    }
                    let index = fields[2]
                        .trim()
                        .parse::<usize>()
                        .ok()
                        .and_then(|n| n.checked_sub(1))
                        .filter(|i| *i < panel.model.page.rows.len())
                        .ok_or("Select a house row (starting at 1)")?;
                    panel.model.index = index;
                    panel.fields = fields;
                    panel.fields[6].clear();
                    Ok(None)
                }
                HouseBrowsingAction::OwnershipOpen => {
                    if panel.ownership.open || !panel.ownership.model.available() {
                        return Err("Ownership is unavailable".into());
                    }
                    let command = panel.ownership.page(0)?;
                    self.commands
                        .send(crate::combat_input::PlayCommand::Feature(command))
                        .map_err(|_| "The session has ended".to_owned())?;
                    panel.ownership.open = true;
                    Ok(None)
                }
                HouseBrowsingAction::OwnershipPage(direction) => {
                    if !panel.ownership.open || !(-1..=1).contains(&direction) {
                        return Err("Open ownership first".into());
                    }
                    let command = panel.ownership.page(direction)?;
                    self.commands
                        .send(crate::combat_input::PlayCommand::Feature(command))
                        .map_err(|_| "The session has ended".to_owned())?;
                    Ok(None)
                }
                HouseBrowsingAction::OwnershipSelect(index) => {
                    if !panel.ownership.open {
                        return Err("Open ownership first".into());
                    }
                    panel.ownership.select(index)?;
                    Ok(None)
                }
                HouseBrowsingAction::OwnershipReview { action, fields } => {
                    if !panel.ownership.open {
                        return Err("Open ownership first".into());
                    }
                    input::ownership::review(&mut mailbox, action, fields).map(Some)
                }
                HouseBrowsingAction::OwnershipConfirm { review, accepted } => {
                    if !panel.ownership.open {
                        return Err("Open ownership first".into());
                    }
                    if let Some(command) = panel.ownership.model.confirm(review, accepted)? {
                        self.commands
                            .send(crate::combat_input::PlayCommand::Feature(command))
                            .map_err(|_| "The session has ended".to_owned())?;
                    }
                    Ok(None)
                }
            }
        })();
        if result.is_err() {
            mailbox.properties.house = original;
        }
        result
    }
}
fn validate_fields(fields: &[String; 7]) -> Result<(), String> {
    if fields.iter().any(|f| f.len() > 8192 || f.contains('\0')) {
        return Err("Check the house fields".into());
    }
    Ok(())
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn house_browsing_initialize(&self) {
        lock(&self.0).properties.house.initialize(true);
    }
    pub(crate) fn house_browsing_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.properties_result(command, result);
    }
}

#[cfg(test)]
mod tests {
    use crate::play::{mailbox::lock, play_channel};
    #[test]
    fn house_browsing_rejected_when_session_replaced_or_target_changed() {
        let (link, mut queue, _) = play_channel();
        lock(&link.mailbox).properties.house.initialize(true);
        link.house_open();
        let snapshot = link.house_browsing_snapshot();
        let (other, _other_queue, _other_outbox) = play_channel();
        lock(&other.mailbox).properties.house.initialize(true);
        other.house_open();
        assert!(
            other
                .house_browsing_dispatch_bound(&snapshot, super::HouseBrowsingAction::Close)
                .is_err()
        );
        link.house_edit(1, "changed".into());
        assert!(
            link.house_browsing_dispatch_bound(
                &snapshot,
                super::HouseBrowsingAction::Map {
                    fields: snapshot.fields.clone()
                }
            )
            .is_err()
        );
        assert!(link.house_panel().model.open);
        assert!(queue.try_recv().is_err());
    }
}

#[cfg(test)]
mod action_tests {
    use super::*;
    use crate::{combat_input::PlayCommand, play::play_channel};
    use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
    use oteryn_protocol_oteryn::{house as wire, house_sale as sale};
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    fn reply(payload: Vec<u8>) -> GatedCommandResult {
        GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload,
        }
    }
    fn command(
        queue: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    ) -> Result<FeatureCommand, Box<dyn std::error::Error>> {
        let PlayCommand::Feature(command) = queue.try_recv()? else {
            return Err("wrong command".into());
        };
        Ok(command)
    }
    #[test]
    fn house_browsing_all_listing_actions_use_existing_owner() -> TestResult {
        for selection in [0, 1, 2, 3, 4, 5, 6, 7, 11, 12, 13] {
            let (link, mut queue, outbox) = play_channel();
            outbox.house_browsing_initialize();
            link.house_open();
            let mut fields: [String; 7] = Default::default();
            fields[0] = "01010101010171018101010101010101".into();
            fields[1] = "oteryn:content.area.city.thais".into();
            link.house_browsing_dispatch_bound(
                &link.house_browsing_snapshot(),
                HouseBrowsingAction::Request {
                    selection: 0,
                    fields: fields.clone(),
                },
            )?;
            let query = command(&mut queue)?;
            let row = wire::HouseSummary {
                details: None,
                house: "oteryn:content.house.test".into(),
                kind: wire::HouseKind::PrivateHouse,
                state: 1,
                rent: 50,
                revision: 4,
                acl_revision: 9,
                owned: true,
                move_out_ms: None,
                price: 100,
                ends_ms: None,
                own_maximum: None,
            };
            outbox.house_browsing_reply(
                &query,
                &reply(
                    wire::encode_house_page(&[row], Some("oteryn:content.house.test"), true)
                        .map_err(|e| format!("{e:?}"))?,
                ),
            );
            fields[2] = "1".into();
            fields[3] = "500".into();
            fields[4] = "02020202020272028202020202020202".into();
            fields[5] = "123 456 0".into();
            fields[6] = "CONFIRM".into();
            let expected = link.house_browsing_snapshot();
            let action = HouseBrowsingAction::Request { selection, fields };
            link.house_browsing_dispatch_bound(&expected, action.clone())?;
            let sent = command(&mut queue)?;
            assert_eq!(
                (sent.capability, sent.kind),
                (
                    49,
                    if !(2..11).contains(&selection) {
                        57
                    } else {
                        56
                    }
                )
            );
            if (2..=7).contains(&selection) {
                let intent =
                    wire::decode_house_intent(&sent.payload).map_err(|e| format!("{e:?}"))?;
                assert_eq!(intent.house, "oteryn:content.house.test");
                if matches!(selection, 3..=5) {
                    assert!(matches!(
                        intent.action,
                        wire::HouseAction::Acl { revision: 9, .. }
                            | wire::HouseAction::Door { revision: 9, .. }
                    ));
                }
            }
            assert!(
                link.house_browsing_dispatch_bound(&expected, action)
                    .is_err()
            );
            assert!(queue.try_recv().is_err());
            assert_eq!(link.house_panel().model.page, expected.model.page);
        }
        Ok(())
    }
    #[test]
    fn house_browsing_ownership_actions_use_exact_confirmation_and_revision() -> TestResult {
        for action in 1..=6 {
            let (link, mut queue, outbox) = play_channel();
            outbox.house_browsing_initialize();
            link.house_open();
            link.house_browsing_dispatch_bound(
                &link.house_browsing_snapshot(),
                HouseBrowsingAction::OwnershipOpen,
            )?;
            let query = command(&mut queue)?;
            let data = sale::HouseSaleReply {
                now_unix_ms: 1_800_000_000_000,
                has_more: true,
                rows: vec![sale::HouseSaleRow {
                    house: "oteryn:content.house.test".into(),
                    name: "Server home".into(),
                    revision: 7,
                    owner: "Owner".into(),
                    recipient: "Recipient".into(),
                    transfer_state: 1,
                    due_at_unix_ms: 1_800_172_800_000,
                    price: 200,
                    rent: 50,
                    allowed: 126,
                    move_out_state: 1,
                }],
            };
            outbox.house_browsing_reply(
                &query,
                &reply(sale::encode_reply(&data).map_err(|e| format!("{e:?}"))?),
            );
            let expected = link.house_browsing_snapshot();
            let review = link
                .house_browsing_dispatch_bound(
                    &expected,
                    HouseBrowsingAction::OwnershipReview {
                        action,
                        fields: [
                            "Recipient".into(),
                            house::format_ownership_date(1_800_172_800_000),
                            "200".into(),
                        ],
                    },
                )?
                .ok_or("review")?;
            assert!(queue.try_recv().is_err());
            assert!(review.text().contains("Server home"));
            let expected = link.house_browsing_snapshot();
            let confirm = HouseBrowsingAction::OwnershipConfirm {
                review,
                accepted: true,
            };
            link.house_browsing_dispatch_bound(&expected, confirm.clone())?;
            let sent = command(&mut queue)?;
            let request = sale::decode_request(&sent.payload).map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                (
                    request.action,
                    request.expected_revision,
                    request.house.as_str()
                ),
                (action, 7, "oteryn:content.house.test")
            );
            assert_eq!(link.house_ownership_panel().model.data(), Some(&data));
            assert!(
                link.house_browsing_dispatch_bound(&expected, confirm)
                    .is_err()
            );
            assert!(queue.try_recv().is_err());
        }
        Ok(())
    }
    #[test]
    fn house_browsing_owner_rejects_modal_closed_and_stale_reply() -> TestResult {
        let (link, mut queue, outbox) = play_channel();
        outbox.house_browsing_initialize();
        link.house_open();
        let expected = link.house_browsing_snapshot();
        let modal = link.bevy_server_modal_fixture();
        assert!(
            link.house_browsing_dispatch_bound(&expected, HouseBrowsingAction::OwnershipOpen)
                .is_err()
        );
        assert_eq!(link.house_browsing_snapshot(), expected);
        drop(modal);
        let mut fields = expected.fields.clone();
        fields[1] = "oteryn:content.area.city.thais".into();
        fields[0] = "01010101010171018101010101010101".into();
        link.house_browsing_dispatch_bound(
            &expected,
            HouseBrowsingAction::Request {
                selection: 0,
                fields,
            },
        )?;
        let query = command(&mut queue)?;
        outbox.house_browsing_initialize();
        let ended = link.house_browsing_snapshot();
        outbox.house_browsing_reply(
            &query,
            &reply(wire::encode_house_page(&[], None, true).map_err(|e| format!("{e:?}"))?),
        );
        assert_eq!(link.house_browsing_snapshot(), ended);
        link.house_open();
        let expected = link.house_browsing_snapshot();
        drop(queue);
        assert!(
            link.house_browsing_dispatch_bound(&expected, HouseBrowsingAction::Close)
                .is_err()
        );
        assert_eq!(link.house_browsing_snapshot(), expected);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn house_browsing_unavailable_window_can_close() -> Result<(), String> {
    let (link, _queue, _outbox) = crate::play::play_channel();
    lock(&link.mailbox).properties.house.initialize(false);
    link.house_open();
    link.house_browsing_dispatch_bound(
        &link.house_browsing_snapshot(),
        HouseBrowsingAction::Close,
    )?;
    assert!(!link.house_panel().model.open);
    Ok(())
}
