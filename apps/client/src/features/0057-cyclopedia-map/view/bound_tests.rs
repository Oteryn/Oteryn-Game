use super::bound::*;
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_ui::cyclopedia_map::Intent;
#[test]
fn cyclopedia_map_window_shows_server_data_only() {
    let (link, _, _) = play_channel();
    let snapshot = link.cyclopedia_map_snapshot();
    assert!(snapshot.panel.map.is_none());
    assert!(snapshot.panel.atlas.is_none());
    assert!(snapshot.panel.geometry.areas.is_empty());
    lock(&link.mailbox).properties.cyclopedia_map.summary = "server discovery".into();
    let snapshot = link.cyclopedia_map_snapshot();
    assert!(snapshot.text.contains("server discovery"));
}
#[test]
fn cyclopedia_map_actions_are_bound_and_confirmed() {
    let (link, mut commands, _) = play_channel();
    {
        let mut m = lock(&link.mailbox);
        m.properties.cyclopedia_map.selected = true;
        m.properties.cyclopedia_map.open = true;
        m.properties.cyclopedia_map.fields[0] = "100".into();
    }
    let snapshot = link.cyclopedia_map_snapshot();
    assert!(!link.cyclopedia_map_dispatch_bound(
        &snapshot,
        MapControl::Request(Intent::Donate),
        false
    ));
    assert!(commands.try_recv().is_err());
    assert!(link.cyclopedia_map_dispatch_bound(
        &snapshot,
        MapControl::Request(Intent::Donate),
        true
    ));
    assert!(matches!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::Feature(_))
    ));
    assert!(commands.try_recv().is_err());
    assert!(link.cyclopedia_map_panel().map.is_none());
    assert!(!link.cyclopedia_map_dispatch_bound(
        &snapshot,
        MapControl::Request(Intent::Donate),
        true
    ));
}
#[test]
fn cyclopedia_map_rejected_when_session_replaced_or_target_changed() {
    let (link, mut commands, _) = play_channel();
    {
        let mut m = lock(&link.mailbox);
        m.properties.cyclopedia_map.selected = true;
        m.properties.cyclopedia_map.open = true;
    }
    let snapshot = link.cyclopedia_map_snapshot();
    let (other, _, _) = play_channel();
    assert!(!other.cyclopedia_map_dispatch_bound(
        &snapshot,
        MapControl::Request(Intent::Query),
        false
    ));
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.cyclopedia_map_dispatch_bound(
        &snapshot,
        MapControl::Request(Intent::Query),
        false
    ));
    assert_eq!(snapshot, link.cyclopedia_map_snapshot());
    drop(modal);
    lock(&link.mailbox)
        .properties
        .cyclopedia_map
        .geometry
        .selected = Some(42);
    assert!(!link.cyclopedia_map_dispatch_bound(
        &snapshot,
        MapControl::Request(Intent::Query),
        false
    ));
    assert!(commands.try_recv().is_err());
}

#[test]
fn cyclopedia_map_atlas_reply_and_stale_session() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::{
        cyclopedia_map::geometry as wire,
        raw_feature::{CommandStatus, GatedCommandResult},
    };
    let (link, mut commands, outbox) = play_channel();
    {
        let mut m = lock(&link.mailbox);
        m.properties.cyclopedia_map.open = true;
        m.properties.cyclopedia_map.selected = true;
    }
    let before = link.cyclopedia_map_snapshot();
    assert!(link.cyclopedia_map_dispatch_bound(&before, MapControl::Geometry(0), false));
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("atlas request absent".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        wire::Request {
            area: 0,
            floor: 1,
            after: 0
        }
    );
    assert!(link.cyclopedia_map_panel().geometry.areas.is_empty());
    let reply = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: wire::encode_reply(&wire::Reply::Areas(wire::AreaPage {
            parent: 0,
            after: 0,
            next: None,
            areas: vec![wire::Area {
                id: 23,
                parent: 0,
                name: "Confirmed server region".into(),
            }],
        }))
        .map_err(|e| format!("{e:?}"))?,
    };
    outbox.properties_result(&command, &reply);
    let after = link.cyclopedia_map_snapshot();
    assert_eq!(after.panel.geometry.areas.len(), 1);
    assert_eq!(
        after.panel.geometry.areas[0].name,
        "Confirmed server region"
    );
    let unchanged = after.clone();
    outbox.properties_result(&command, &reply);
    assert_eq!(link.cyclopedia_map_snapshot(), unchanged);
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    let ended = link.cyclopedia_map_snapshot();
    assert!(!link.cyclopedia_map_dispatch_bound(&after, MapControl::Geometry(5), false));
    outbox.properties_result(&command, &reply);
    assert_eq!(link.cyclopedia_map_snapshot(), ended);
    Ok(())
}

#[test]
fn cyclopedia_map_all_intents_use_existing_dispatch() -> Result<(), Box<dyn std::error::Error>> {
    for intent in [
        Intent::Query,
        Intent::Atlas,
        Intent::RegionAtlas,
        Intent::Start,
        Intent::Shuffle,
        Intent::Find,
        Intent::Donate,
    ] {
        let (link, mut commands, _) = play_channel();
        {
            let mut mailbox = lock(&link.mailbox);
            let panel = &mut mailbox.properties.cyclopedia_map;
            panel.open = true;
            panel.selected = true;
            panel.fields[0] = "100".into();
        }
        let before = link.cyclopedia_map_snapshot();
        let expected = super::super::protocol::command(&before.panel, intent)?;
        let action = MapControl::Request(intent);
        if action.needs_confirmation() {
            assert!(!link.cyclopedia_map_dispatch_bound(&before, action.clone(), false));
            assert!(commands.try_recv().is_err());
        }
        assert!(link.cyclopedia_map_dispatch_bound(&before, action.clone(), true));
        let crate::combat_input::PlayCommand::Feature(actual) = commands.try_recv()? else {
            return Err("feature command absent".into());
        };
        assert_eq!(actual, expected);
        assert!(!link.cyclopedia_map_dispatch_bound(&before, action, true));
        assert!(link.cyclopedia_map_panel().map.is_none());
        if matches!(
            intent,
            Intent::Query | Intent::Atlas | Intent::Start | Intent::Shuffle | Intent::Find
        ) {
            let crate::combat_input::PlayCommand::Feature(speed) = commands.try_recv()? else {
                return Err("speed query absent".into());
            };
            assert_eq!(
                speed.kind,
                oteryn_client_runtime::cyclopedia_map::COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY
            );
        }
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
