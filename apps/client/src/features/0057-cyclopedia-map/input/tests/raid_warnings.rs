use super::*;
use oteryn_client_runtime::cyclopedia_map::{COMMAND_TYPE_CYCLOPEDIA_QUERY, raid_warnings as wire};
use std::time::Instant;

#[test]
fn raid_warning_query_sent_only_when_626_selected() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = crate::play::play_channel();
    let now = Instant::now();
    let poll = |instant| {
        raid_warnings::poll(&mut lock(&link.mailbox).properties, &link.commands, instant);
    };
    lock(&link.mailbox).properties.cyclopedia_map.selected = true;
    link.cyclopedia_map_open();
    poll(now);
    assert!(
        commands.try_recv().is_err(),
        "57 alone cannot send selector 129"
    );

    link.cyclopedia_atlas_close();
    lock(&link.mailbox)
        .properties
        .cyclopedia_map
        .raid_warnings_selected = true;
    poll(now);
    assert!(commands.try_recv().is_err(), "a closed map does not poll");
    link.cyclopedia_map_open();
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("expected a raid warning query on open".into());
    };
    assert_eq!(command.capability, 57);
    assert_eq!(command.kind, COMMAND_TYPE_CYCLOPEDIA_QUERY);
    assert_eq!(command.payload, wire::QUERY);

    let refresh = now + std::time::Duration::from_secs(61);
    poll(refresh);
    assert!(
        commands.try_recv().is_err(),
        "an outstanding query is not duplicated"
    );
    let reply = oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
        payload: wire::encode(&[]).map_err(|error| format!("{error:?}"))?,
    };
    outbox.properties_result(&command, &reply);
    poll(now + std::time::Duration::from_secs(59));
    assert!(
        commands.try_recv().is_err(),
        "the refresh interval has not elapsed"
    );
    poll(refresh);
    let PlayCommand::Feature(refreshed) = commands.try_recv()? else {
        return Err("expected a periodic raid warning query".into());
    };
    assert_eq!(refreshed.payload, wire::QUERY);
    outbox.properties_result(&refreshed, &reply);

    link.cyclopedia_atlas_close();
    poll(refresh + std::time::Duration::from_secs(120));
    assert!(commands.try_recv().is_err(), "refresh stops on close");
    Ok(())
}
