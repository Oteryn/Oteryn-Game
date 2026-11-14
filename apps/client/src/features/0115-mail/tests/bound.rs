use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, play_channel},
};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn seed(outbox: &Outbox) {
    lock(&outbox.0).properties.mail = Panel {
        open: true,
        selected: true,
        fields: ["4242".into(), "9000".into(), "Alice\nThais".into()],
        summary: "Confirmed mail status".into(),
        ..Default::default()
    };
}
#[test]
fn mail_form_shows_server_bound_item() {
    let (link, _commands, outbox) = play_channel();
    seed(&outbox);
    let binding = link.mail_binding();
    assert_eq!(binding.panel, link.mail_panel());
    assert_eq!(binding.panel.fields, ["4242", "9000", "Alice\nThais"]);
    assert_eq!(binding.panel.summary, "Confirmed mail status");
}
#[test]
fn mail_send_is_item_bound_and_single() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    seed(&outbox);
    let binding = link.mail_binding();
    assert!(link.mail_dispatch_bound(&binding, binding.panel.fields.clone(), Some(Intent::Post)));
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!(command.capability, 115);
    assert_eq!(command.kind, 124);
    assert_eq!(
        oteryn_session::property_services::mail::decode_intent(&command.payload),
        Ok(oteryn_session::property_services::mail::MailIntent::Post {
            item_handle: 4242,
            mailbox_handle: 9000
        })
    );
    assert!(link.mail_panel().pending);
    assert_eq!(link.mail_panel().summary, binding.panel.summary);
    assert!(!link.mail_dispatch_bound(&binding, binding.panel.fields.clone(), Some(Intent::Post)));
    assert!(commands.try_recv().is_err());
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: vec![1],
        },
    );
    assert!(!link.mail_panel().pending);
    assert_eq!(link.mail_panel().feedback, "Mail posted");
    Ok(())
}
#[test]
fn text_form_edit_is_item_revision_bound() -> TestResult {
    // The native owner has a panel snapshot, not a wire text revision.
    let (link, mut commands, outbox) = play_channel();
    seed(&outbox);
    let stale = link.mail_binding();
    link.mail_edit(2, "newer owner text".into());
    let before = link.mail_panel();
    assert!(!link.mail_dispatch_bound(&stale, stale.panel.fields.clone(), Some(Intent::Write)));
    assert_eq!(link.mail_panel(), before);
    let current = link.mail_binding();
    let mut values = current.panel.fields.clone();
    values[2] = "edited text".into();
    assert!(link.mail_dispatch_bound(&current, values, Some(Intent::Write)));
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!(
        oteryn_session::property_services::mail::decode_intent(&command.payload),
        Ok(
            oteryn_session::property_services::mail::MailIntent::WriteText {
                item_handle: 4242,
                text: "edited text".into()
            }
        )
    );
    assert!(link.mail_panel().pending);
    assert!(link.mail_panel().feedback.is_empty());
    Ok(())
}
#[test]
fn mail_rejected_when_session_replaced_or_item_changed() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    seed(&outbox);
    lock(&outbox.0)
        .inventory
        .apply(Some(oteryn_session::CharacterInventory {
            main_backpack: None,
            entries: vec![oteryn_session::ItemEntry {
                handle: std::num::NonZeroU64::new(4242).ok_or("handle")?,
                item_definition_ref: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            }],
            equipment: Vec::new(),
        }));
    let old = link.mail_binding();
    let (replacement, replacement_commands, replacement_outbox) = play_channel();
    seed(&replacement_outbox);
    let before = replacement.mail_panel();
    assert!(!replacement.mail_dispatch_bound(&old, old.panel.fields.clone(), Some(Intent::Write)));
    assert_eq!(replacement.mail_panel(), before);
    lock(&outbox.0).inventory.apply(None);
    let before = link.mail_panel();
    assert!(!link.mail_dispatch_bound(&old, old.panel.fields.clone(), Some(Intent::Post)));
    assert_eq!(link.mail_panel(), before);
    let current = link.mail_binding();
    lock(&outbox.0).properties.mail.open = false;
    assert!(!link.mail_dispatch_bound(&current, current.panel.fields.clone(), Some(Intent::Post)));
    assert!(commands.try_recv().is_err());
    seed(&outbox);
    let current = link.mail_binding();
    let _modal = link.bevy_server_modal_fixture();
    let before = link.mail_panel();
    assert!(!link.mail_dispatch_bound(&current, current.panel.fields.clone(), Some(Intent::Post)));
    assert_eq!(link.mail_panel(), before);
    drop(replacement_commands);
    let closed = replacement.mail_binding();
    let before = replacement.mail_panel();
    assert!(!replacement.mail_dispatch_bound(
        &closed,
        closed.panel.fields.clone(),
        Some(Intent::Post)
    ));
    assert_eq!(replacement.mail_panel(), before);
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    let ended = link.mail_panel();
    assert!(!link.mail_dispatch_bound(&current, current.panel.fields.clone(), Some(Intent::Post)));
    assert_eq!(link.mail_panel(), ended);
    Ok(())
}
