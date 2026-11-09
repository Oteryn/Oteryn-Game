//! Tests the same borrowed callbacks as the native selector and strict response path.
use crate::combat_input::PlayCommand;
use crate::play::{
    feature_0152_item_forge::input::controller::{Action, activate},
    mailbox::lock,
    play_channel,
};
use oteryn_client_runtime::{
    cui3::item_forge::{Operation, Request},
    forge_preview as wire,
    raw_feature::{CommandStatus, GatedCommandResult},
};
type Error = Box<dyn std::error::Error + Send + Sync>;
fn item(handle: u64) -> Result<wire::Item, Error> {
    Ok(wire::Item {
        handle: std::num::NonZeroU64::new(handle).ok_or("handle")?,
        definition_ref: 7,
        appearance: 3264,
        name: "Practice sword".into(),
        classification: 1,
        tier: 0,
        maximum_tier: 1,
        container: false,
    })
}
fn result(reply: wire::Reply) -> Result<GatedCommandResult, Error> {
    Ok(GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: reply.encode().ok_or("reply")?,
    })
}
#[test]
fn forge_live_callbacks_quote_generate_and_retry_exact_attempt_without_owner_drop()
-> Result<(), Error> {
    let (link, mut commands, outbox) = play_channel();
    outbox.item_forge_initialize(true);
    {
        let mut shared = lock(&link.mailbox);
        shared.imbue.selected = true;
        let panel = &mut shared.item_forge;
        panel.preview.selected = true;
        panel.preview.remember(item(2)?);
        panel.preview.remember(item(3)?);
    }
    let action = |action| activate(&mut lock(&link.mailbox), &link.commands, action);
    action(Action::Operation(Operation::Fusion));
    action(Action::First(2));
    action(Action::Second(3));
    action(Action::Cores {
        boost: true,
        protect: true,
    });
    action(Action::Forge);
    assert!(
        commands.try_recv().is_err(),
        "metadata-capable server needs a verified quote"
    );
    action(Action::Quote);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("quote command".into());
    };
    let request = wire::Request::decode(&command.payload).ok_or("request")?;
    assert_eq!((command.capability, command.kind), (152, 6116));
    let quote = wire::Quote {
        selection: request.selection,
        first: item(2)?,
        second: item(3)?,
        resources: wire::Resources {
            carried_gold: 25_000,
            dust: 100,
            dust_limit: 100,
            cores: 2,
            ..Default::default()
        },
        cost: wire::Cost {
            gold: 25_000,
            dust: 100,
            cores: 2,
        },
        target_tier: 1,
        success_percent: 65,
        protection_percent: 50,
    };
    outbox.properties_result(
        &command,
        &result(wire::Reply {
            nonce: request.nonce,
            payload: wire::Payload::Quote(quote),
        })?,
    );
    assert!(link.item_forge_panel().verified_quote());
    action(Action::Forge);
    let PlayCommand::ServerMechanic(paid) = commands.try_recv()? else {
        return Err("paid command".into());
    };
    assert_eq!(
        (paid.capability, paid.command_type, paid.payload.len()),
        (152, 181, 51)
    );
    let original = Request::decode(&paid.payload).ok_or("original")?;
    assert_eq!(original.occurrence[6] >> 4, 7);
    assert_eq!(original.occurrence[8] & 192, 128);
    action(Action::Forge);
    assert!(
        commands.try_recv().is_err(),
        "pending suppresses duplicate paid attempts"
    );
    outbox.item_forge_result(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: vec![0xff],
        },
    );
    action(Action::First(3));
    assert_eq!(
        link.item_forge_panel().first_item,
        2,
        "unresolved attempt freezes operands"
    );
    action(Action::Retry);
    let PlayCommand::ServerMechanic(retry) = commands.try_recv()? else {
        return Err("retry".into());
    };
    assert_eq!(retry.payload, paid.payload);
    assert!(
        lock(&link.mailbox).imbue.selected,
        "borrowed callback never drops owning PlayLink or resets another panel"
    );
    Ok(())
}
#[test]
fn forge_stale_quote_and_removed_candidate_are_invalidated_with_legacy_fallback()
-> Result<(), Error> {
    let (link, mut commands, outbox) = play_channel();
    outbox.item_forge_initialize(true);
    {
        let mut shared = lock(&link.mailbox);
        let panel = &mut shared.item_forge;
        panel.preview.selected = true;
        panel.preview.remember(item(2)?);
        panel.preview.remember(item(3)?);
        panel.choose_operation(Operation::Fusion);
        panel.choose_item(true, 2);
        panel.choose_item(false, 3);
    }
    activate(&mut lock(&link.mailbox), &link.commands, Action::Quote);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("quote".into());
    };
    let request = wire::Request::decode(&command.payload).ok_or("request")?;
    lock(&link.mailbox).item_forge.scope_changed();
    outbox.properties_result(
        &command,
        &result(wire::Reply {
            nonce: request.nonce,
            payload: wire::Payload::Refused,
        })?,
    );
    assert!(
        link.item_forge_panel().preview.feedback.is_none(),
        "old in-flight reply is ignored after scope invalidation"
    );
    {
        let mut shared = lock(&link.mailbox);
        let panel = &mut shared.item_forge;
        panel.preview.selected = false;
        panel.observe(vec![
            oteryn_client_runtime::depot::ItemEntry {
                handle: item(2)?.handle,
                item_definition_ref: std::num::NonZeroU32::new(7).ok_or("ref")?,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            },
            oteryn_client_runtime::depot::ItemEntry {
                handle: item(3)?.handle,
                item_definition_ref: std::num::NonZeroU32::new(7).ok_or("ref")?,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            },
        ]);
        panel.choose_operation(Operation::Fusion);
        assert!(panel.choose_item(true, 2));
        assert!(panel.choose_item(false, 3));
    }
    activate(&mut lock(&link.mailbox), &link.commands, Action::Forge);
    assert!(
        matches!(commands.try_recv(), Ok(PlayCommand::ServerMechanic(_))),
        "legacy cap152 remains usable without optional metadata"
    );
    {
        let mut shared = lock(&link.mailbox);
        let panel = &mut shared.item_forge;
        panel.observe(vec![]);
        assert_eq!(panel.first_item, 0);
        assert_eq!(panel.second_item, 0);
        assert!(panel.preview.quote.is_none());
    }
    Ok(())
}
