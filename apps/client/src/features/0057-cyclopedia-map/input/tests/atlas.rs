use crate::{
    combat_input::PlayCommand,
    play::{mailbox::lock, play_channel},
};
use oteryn_client_runtime::{
    cyclopedia_map::{self, geometry as wire},
    raw_feature::{CommandStatus, GatedCommandResult},
};

/// Takes the next gameplay command and answers the speed query that follows an atlas intent.
fn next_command(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &crate::play::mailbox::Outbox,
) -> Result<PlayCommand, Box<dyn std::error::Error>> {
    let command = receiver.try_recv()?;
    if let Ok(PlayCommand::Feature(speed)) = receiver.try_recv() {
        assert_eq!(
            (speed.capability, speed.kind),
            (57, cyclopedia_map::COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY)
        );
        outbox.properties_result(
            &speed,
            &GatedCommandResult {
                command_id: 100,
                server_sequence: 100,
                status: CommandStatus::Accepted,
                payload: 0u16.to_le_bytes().to_vec(),
            },
        );
    }
    Ok(command)
}

#[test]
fn atlas_geometry_pages_layers_and_floor_filters_use_real_queue()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = play_channel();
    lock(&link.mailbox).properties.cyclopedia_map.selected = true;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    assert!(
        !link.cyclopedia_atlas_input(&[NormalizedInputEvent::Key {
            code: KeyCode::new(68)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
            repeat: false,
        }])
    );
    assert!(receiver.try_recv().is_err());
    assert!(
        link.cyclopedia_atlas_input(&[NormalizedInputEvent::Key {
            code: KeyCode::new(66)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
            repeat: false,
        }])
    );
    let PlayCommand::Feature(command) = next_command(&mut receiver, &outbox)? else {
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
    let PlayCommand::Feature(command) = next_command(&mut receiver, &outbox)? else {
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
    let PlayCommand::Feature(command) = next_command(&mut receiver, &outbox)? else {
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
    let PlayCommand::Feature(command) = next_command(&mut receiver, &outbox)? else {
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
    let PlayCommand::Feature(command) = next_command(&mut receiver, &outbox)? else {
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

#[test]
fn atlas_navigation_waits_for_speed_before_mutating_confirmed_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, _) = play_channel();
    {
        let mut m = lock(&link.mailbox);
        let p = &mut m.properties.cyclopedia_map;
        p.selected = true;
        p.speed_pending = true;
        p.geometry.selected = Some(1);
        p.geometry.runs = vec![wire::Run {
            y: 1,
            begin: 1,
            end: 3,
            subarea: 1,
        }];
        p.geometry.bounds = Some([1, 1, 2, 1]);
    }
    let before = link.cyclopedia_map_panel();
    link.cyclopedia_atlas_action(6);
    assert_eq!(link.cyclopedia_map_panel().geometry, before.geometry);
    let mut native = link.cyclopedia_map_panel();
    let query = native.atlas_query;
    assert!(!oteryn_client_ui::cyclopedia_map::atlas_action(
        &mut native,
        5
    ));
    assert_eq!(native.atlas_query, query);
    assert!(receiver.try_recv().is_err());
    Ok(())
}

#[test]
fn atlas_floor_controls_clamp_at_supported_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, _) = play_channel();
    {
        let mut m = lock(&link.mailbox);
        let p = &mut m.properties.cyclopedia_map;
        p.selected = true;
        p.geometry.selected = Some(1);
        p.geometry.floor = 0;
    }
    link.cyclopedia_atlas_action(6);
    assert_eq!(link.cyclopedia_map_panel().geometry.floor, 0);
    assert!(receiver.try_recv().is_err());
    lock(&link.mailbox).properties.cyclopedia_map.geometry.floor = -15;
    link.cyclopedia_atlas_action(7);
    assert_eq!(link.cyclopedia_map_panel().geometry.floor, -15);
    assert!(receiver.try_recv().is_err());
    let mut p = link.cyclopedia_map_panel();
    let query = cyclopedia_map::atlas::Query {
        floor: 0,
        x: 1,
        y: 1,
        zoom: 0,
        page: 0,
    };
    p.atlas_query = query;
    p.atlas = Some(cyclopedia_map::atlas::Atlas {
        query,
        digest: [1; 32],
        floors: u16::MAX,
        progress: None,
        cells: vec![0; 64 * 40],
        areas: vec![],
        markers: vec![],
        marker_total: 0,
    });
    assert!(!oteryn_client_ui::cyclopedia_map::atlas_action(&mut p, 2));
    assert_eq!(p.atlas_query.floor, 0);
    p.atlas.as_mut().ok_or("atlas")?.query.floor = -15;
    p.atlas_query.floor = -15;
    assert!(!oteryn_client_ui::cyclopedia_map::atlas_action(&mut p, 3));
    assert_eq!(p.atlas_query.floor, -15);
    p.map = Some(cyclopedia_map::MapResult {
        code: 0,
        revision: 1,
        discovered: false,
        found: 0,
        area_percent: 0,
        pool: 0,
        epoch: 0,
        improved: false,
        visible: vec![(1, 1, -15), (1, 1, 0)],
    });
    p.floor = 0;
    p.select_floor(false);
    assert_eq!(p.floor, 0);
    p.floor = -15;
    p.select_floor(true);
    assert_eq!(p.floor, -15);
    Ok(())
}

#[test]
fn atlas_drawing_marks_authoritative_player_on_the_matching_floor() {
    use oteryn_session::{ActorPosition, AppliedDelta, SessionEvent, WorldSpatialObservation};
    let (link, _receiver, outbox) = play_channel();
    {
        let mut m = lock(&link.mailbox);
        let p = &mut m.properties.cyclopedia_map;
        p.geometry.bounds = Some([100, 200, 110, 210]);
        p.geometry.floor = -7;
        p.hide_pois = true;
    }
    for floor in [-7, -8] {
        outbox.push(vec![SessionEvent::WorldSpatial(AppliedDelta {
            server_sequence: 1,
            base_revision: 1,
            new_revision: 2,
            value: WorldSpatialObservation {
                content_generation: [0; 32],
                actor_position: ActorPosition {
                    x: 105,
                    y: 205,
                    floor,
                },
            },
        })]);
        let p = link.cyclopedia_map_panel();
        let drawing = oteryn_client_ui::cyclopedia_map::atlas_view::project(&p);
        assert_eq!(drawing.player.is_some(), floor == -7);
        if floor == -7 {
            assert_eq!(drawing.player, Some([297, 247]));
        }
    }
}
