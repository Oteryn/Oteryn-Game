use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::{
    house::house_sale as wire,
    raw_feature::{CommandStatus, GatedCommandResult},
};
#[test]
fn house_ownership_menu_and_confirmations_reach_the_real_play_queue()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut queue, outbox) = play_channel();
    crate::play::mailbox::lock(&link.mailbox)
        .properties
        .house
        .initialize(true);
    link.house_move_out_open();
    let PlayCommand::Feature(query) = queue.try_recv()? else {
        return Err("house catalogue was not requested".into());
    };
    assert_eq!(
        (query.capability, query.kind),
        (wire::CAPABILITY, wire::COMMAND)
    );
    assert_eq!(
        wire::decode_request(&query.payload)
            .map_err(|e| format!("{e:?}"))?
            .action,
        0
    );
    let now = 1_800_000_000_000;
    let reply = wire::HouseSaleReply {
        now_unix_ms: now,
        has_more: false,
        rows: vec![wire::HouseSaleRow {
            house: "oteryn:content.house.admiral_s_avenue_1".into(),
            name: "Harbour Home".into(),
            revision: 7,
            owner: "Elara Vale".into(),
            recipient: String::new(),
            transfer_state: 0,
            due_at_unix_ms: 0,
            price: 0,
            rent: 50,
            allowed: (1 << 1) | (1 << 3),
            move_out_state: 0,
        }],
    };
    let result = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: wire::encode_reply(&reply).map_err(|e| format!("{e:?}"))?,
    };
    outbox.properties_result(&query, &result);
    assert_eq!(
        link.house_ownership_panel()
            .model
            .row()
            .map(|r| r.name.as_str()),
        Some("Harbour Home")
    );
    let fields = [
        "Mira Vale".into(),
        oteryn_client_ui::house::format_ownership_date(now + 172_800_000),
        "200".into(),
    ];
    let review = link.house_ownership_review(3, fields.clone())?;
    assert!(
        queue.try_recv().is_err(),
        "reviewing alone emits no mutation"
    );
    link.house_ownership_confirm(review, false);
    assert!(queue.try_recv().is_err());
    let stale = link.house_ownership_review(3, fields.clone())?;
    link.house_ownership_select(0)?;
    link.house_ownership_confirm(stale, true);
    assert!(queue.try_recv().is_err());
    let review = link.house_ownership_review(3, fields)?;
    link.house_ownership_confirm(review, true);
    let PlayCommand::Feature(command) = queue.try_recv()? else {
        return Err("confirmed transfer was not queued".into());
    };
    let offer = wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (
            offer.action,
            offer.house.as_str(),
            offer.expected_revision,
            offer.price
        ),
        (3, reply.rows[0].house.as_str(), 7, 200)
    );
    assert_eq!(offer.due_at_unix_ms, now + 172_800_000);
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Rejected,
            payload: vec![],
        },
    );
    assert!(!link.house_ownership_panel().model.pending());
    assert!(link.house_ownership_panel().model.data().is_none());
    crate::play::mailbox::lock(&link.mailbox).properties.end();
    link.house_ownership_page(0);
    assert!(queue.try_recv().is_err());
    Ok(())
}

#[test]
fn ownership_refresh_resynchronizes_combo_with_unchanged_row_keys()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut queue, outbox) = play_channel();
    crate::play::mailbox::lock(&link.mailbox)
        .properties
        .house
        .initialize(true);
    link.house_ownership_open();
    let row = |name: &str| wire::HouseSaleRow {
        house: format!("oteryn:content.house.{name}"),
        name: name.into(),
        revision: 1,
        owner: "Alice".into(),
        recipient: String::new(),
        transfer_state: 0,
        due_at_unix_ms: 0,
        price: 0,
        rent: 1,
        allowed: 2,
        move_out_state: 0,
    };
    let reply = wire::HouseSaleReply {
        now_unix_ms: 1_800_000_000_000,
        has_more: false,
        rows: vec![row("a"), row("b")],
    };
    let payload = wire::encode_reply(&reply).map_err(|e| format!("{e:?}"))?;
    let PlayCommand::Feature(query) = queue.try_recv()? else {
        return Err("query".into());
    };
    outbox.properties_result(
        &query,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: payload.clone(),
        },
    );
    link.house_ownership_select(1)?;
    link.house_ownership_page(0);
    let PlayCommand::Feature(query) = queue.try_recv()? else {
        return Err("refresh".into());
    };
    outbox.properties_result(
        &query,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload,
        },
    );
    let current = link.house_ownership_panel();
    assert_eq!(current.model.index(), 0);
    assert_eq!(
        crate::play::feature_0049_house::view::ownership_selection(&current, Some(1), false, false),
        Some(0)
    );
    assert_eq!(
        crate::play::feature_0049_house::view::ownership_selection(&current, Some(0), false, false),
        None
    );
    assert_eq!(
        crate::play::feature_0049_house::view::ownership_selection(&current, Some(1), false, true),
        None,
        "a queued user selection must be read before projection resets the control"
    );
    Ok(())
}
