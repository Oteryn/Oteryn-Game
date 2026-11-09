use crate::{
    combat_input::PlayCommand,
    play::{mailbox::lock, play_channel},
};
use oteryn_client_runtime::{
    cyclopedia_map::{self, geometry as wire},
    raw_feature::{CommandStatus, GatedCommandResult},
};

#[test]
fn atlas_geometry_pages_layers_and_floor_filters_use_real_queue()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = play_channel();
    lock(&link.mailbox).properties.cyclopedia_map.selected = true;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    assert!(
        link.cyclopedia_atlas_input(&[NormalizedInputEvent::Key {
            code: KeyCode::new(68)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
            repeat: false,
        }])
    );
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing region query".into());
    };
    assert_eq!((command.capability, command.kind), (57, 72));
    let request = wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((request.area, request.floor, request.after), (0, 1, 0));
    let names = wire::Reply::Areas(wire::AreaPage {
        parent: 0,
        after: 0,
        next: None,
        areas: vec![wire::Area {
            id: 1,
            parent: 0,
            name: "Harbor reaches".into(),
        }],
    });
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&names).map_err(|e| format!("{e:?}"))?,
        },
    );
    link.cyclopedia_atlas_action(5);
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing geometry query".into());
    };
    let request = wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((request.area, request.floor, request.after), (1, -7, 0));
    let geometry = wire::GeometryPage {
        area: 1,
        floor: -7,
        page: 0,
        next: Some(1),
        total: 1025,
        bounds: [10, 0, 19, 1024],
        runs: (0..1024)
            .map(|y| wire::Run {
                y,
                begin: 10,
                end: 20,
                subarea: 2,
            })
            .collect(),
    };
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&wire::Reply::Geometry(geometry))
                .map_err(|e| format!("{e:?}"))?,
        },
    );
    assert_eq!(link.cyclopedia_map_panel().geometry.runs.len(), 1024);
    link.cyclopedia_atlas_pump();
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing geometry continuation".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload)
            .map_err(|e| format!("{e:?}"))?
            .after,
        1
    );
    let geometry = wire::GeometryPage {
        area: 1,
        floor: -7,
        page: 1,
        next: None,
        total: 1025,
        bounds: [10, 0, 19, 1024],
        runs: vec![wire::Run {
            y: 1024,
            begin: 10,
            end: 20,
            subarea: 2,
        }],
    };
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 3,
            server_sequence: 3,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&wire::Reply::Geometry(geometry))
                .map_err(|e| format!("{e:?}"))?,
        },
    );
    link.cyclopedia_map_request(oteryn_client_ui::cyclopedia_map::Intent::Query);
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing live discovery query".into());
    };
    let points = cyclopedia_map::MapResult {
        code: 0,
        revision: 1,
        discovered: false,
        found: 0,
        area_percent: 0,
        pool: 0,
        epoch: 1,
        improved: false,
        visible: vec![(10, 0, -7)],
    };
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 4,
            server_sequence: 4,
            status: CommandStatus::Accepted,
            payload: cyclopedia_map::encode_result(&points).map_err(|e| format!("{e:?}"))?,
        },
    );
    let drawing =
        oteryn_client_ui::cyclopedia_map::atlas_view::project(&link.cyclopedia_map_panel());
    assert!(!drawing.rects.is_empty());
    assert_eq!(drawing.points.len(), 1);
    link.cyclopedia_atlas_action(8);
    assert!(
        oteryn_client_ui::cyclopedia_map::atlas_view::project(&link.cyclopedia_map_panel())
            .rects
            .is_empty()
    );
    link.cyclopedia_atlas_action(9);
    assert!(
        oteryn_client_ui::cyclopedia_map::atlas_view::project(&link.cyclopedia_map_panel())
            .points
            .is_empty()
    );
    link.cyclopedia_atlas_action(6);
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing floor-filter query".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload)
            .map_err(|e| format!("{e:?}"))?
            .floor,
        -6
    );
    outbox.logged_out();
    assert!(link.cyclopedia_map_panel().geometry.runs.is_empty());
    link.cyclopedia_atlas_pump();
    assert!(receiver.try_recv().is_err());
    Ok(())
}
