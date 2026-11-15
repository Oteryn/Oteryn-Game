use crate::{
    combat_input::PlayCommand,
    play::{mailbox::lock, play_channel},
};
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_map::{
        MapResult,
        atlas::{Area, Atlas, HEIGHT, Marker, Query, WIDTH},
    },
    raw_feature::GatedCommandResult,
};
use oteryn_client_ui::cyclopedia_map::{Intent, atlas_action, atlas_color, atlas_detail};
#[test]
fn atlas_real_playlink_projects_regions_markers_and_navigation()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    lock(&link.mailbox).properties.cyclopedia_map.selected = true;
    link.cyclopedia_map_request(Intent::Atlas);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("atlas command".into());
    };
    assert_eq!(command.kind, 72);
    assert_eq!(command.capability, 57);
    assert_eq!(
        Query::decode(&command.payload).map_err(|_| "query")?,
        Query::default()
    );
    let query = Query {
        floor: -7,
        x: 100,
        y: 200,
        zoom: 0,
        page: 0,
    };
    let atlas = Atlas {
        query,
        digest: [7; 32],
        floors: 1 << 7 | 1 << 8,
        progress: Some(MapResult {
            code: 0,
            revision: 1,
            discovered: false,
            found: 0,
            area_percent: 0,
            pool: 0,
            epoch: 0,
            improved: false,
            visible: vec![(102, 202, -7)],
        }),
        cells: vec![2; WIDTH * HEIGHT],
        areas: vec![
            Area {
                id: 1,
                parent: 0,
                name: "Harbour".into(),
            },
            Area {
                id: 2,
                parent: 1,
                name: "Quays".into(),
            },
        ],
        markers: vec![
            Marker {
                kind: 1,
                x: 101,
                y: 201,
                label: "oteryn:npc.guide".into(),
            },
            Marker {
                kind: 2,
                x: 102,
                y: 201,
                label: "Floor passage".into(),
            },
            Marker {
                kind: 3,
                x: 102,
                y: 202,
                label: "Visible exploration point".into(),
            },
        ],
        marker_total: 3,
    };
    // The production result adapter applies bytes for the real PlayLink request.
    {
        let mut m = lock(&link.mailbox);
        super::super::protocol::apply(
            &mut m.properties.cyclopedia_map,
            &command,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: atlas.encode().map_err(|_| "encode")?,
            },
        );
    }
    let PlayCommand::Feature(speed) = commands.try_recv()? else {
        return Err("speed query".into());
    };
    assert_eq!(
        speed.kind,
        oteryn_client_runtime::cyclopedia_map::COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY
    );
    outbox.properties_result(
        &speed,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: 0u16.to_le_bytes().to_vec(),
        },
    );
    let mut p = link.cyclopedia_map_panel();
    p.atlas_layers = 15;
    p.player_position = Some((105, 205, -7));
    assert_eq!(
        oteryn_client_ui::cyclopedia_map::atlas_player_cell(&p),
        Some([5, 5])
    );
    p.player_position = Some((105, 205, -8));
    assert_eq!(
        oteryn_client_ui::cyclopedia_map::atlas_player_cell(&p),
        None
    );
    p.player_position = None;
    assert!(!p.pending);
    assert_eq!(p.atlas, Some(atlas));
    let detail = atlas_detail(&p);
    assert!(detail.contains("Harbour"));
    assert!(oteryn_client_ui::cyclopedia_map::panel_text(&p).contains("Harbour"));
    assert!(detail.contains("Quays"));
    assert_ne!(atlas_color(&p, 2), atlas_color(&p, 0));
    assert!(atlas_action(&mut p, 9));
    assert_eq!(p.atlas_query.x, 132);
    assert!(atlas_action(&mut p, 5));
    assert_eq!(p.atlas_query.zoom, 1);
    assert!(!atlas_action(&mut p, 10));
    assert_eq!(p.atlas_layers & 1, 0);
    assert!(atlas_action(&mut p, 3));
    assert_eq!(p.atlas_query.floor, -8);
    p.atlas.as_mut().ok_or("atlas")?.progress = None;
    assert!(atlas_detail(&p).contains("Exploration unavailable"));
    p.pending = true;
    let old = p.atlas_query;
    assert!(!atlas_action(&mut p, 1));
    assert_eq!(p.atlas_query, old);
    // No map state is shared across the activity handoff.
    lock(&link.mailbox).cancel_activity_requests();
    assert!(link.cyclopedia_map_panel().atlas.is_none());
    Ok(())
}

#[test]
fn host_floor_and_poi_controls_follow_the_active_atlas() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    let query = Query {
        floor: -7,
        x: 100,
        y: 100,
        zoom: 0,
        page: 0,
    };
    let mut atlas = Atlas {
        query,
        digest: [7; 32],
        floors: (1 << 7) | (1 << 8),
        progress: None,
        cells: vec![0; WIDTH * HEIGHT],
        areas: vec![],
        markers: vec![],
        marker_total: 0,
    };
    {
        let mut m = lock(&link.mailbox);
        let p = &mut m.properties.cyclopedia_map;
        p.selected = true;
        p.atlas_layers = 15;
        p.atlas_query = query;
        p.atlas = Some(atlas.clone());
        (super::form().submit)(&mut m, &link.commands, 23, vec![]);
    }
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("atlas floor request missing".into());
    };
    assert_eq!(
        Query::decode(&command.payload).map_err(|_| "query")?.floor,
        -8
    );
    atlas.query.floor = -8;
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: atlas.encode().map_err(|_| "atlas reply")?,
        },
    );
    let PlayCommand::Feature(speed) = commands.try_recv()? else {
        return Err("speed query missing".into());
    };
    outbox.properties_result(
        &speed,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: 0u16.to_le_bytes().to_vec(),
        },
    );
    assert_eq!(
        link.cyclopedia_map_panel()
            .atlas
            .as_ref()
            .ok_or("active atlas")?
            .query
            .floor,
        -8
    );
    (super::form().submit)(&mut lock(&link.mailbox), &link.commands, 25, vec![]);
    assert_eq!(link.cyclopedia_map_panel().atlas_layers, 11);
    Ok(())
}

#[test]
fn shared_cyclopedia_opens_an_interactive_atlas_overview() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, mut commands, _outbox) = play_channel();
    {
        let mut m = lock(&link.mailbox);
        m.properties.cyclopedia_map.selected = true;
        m.properties.cyclopedia_map.open = true;
    }
    let form = crate::play::feature_0057_cyclopedia_map::view::form();
    assert!(form.dispatch(&link, &form.read(&link), 49, vec![]));
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("atlas request".into());
    };
    assert_eq!((command.capability, command.kind), (57, 72));
    assert_eq!(
        Query::decode(&command.payload).map_err(|_| "atlas payload")?,
        Query::default()
    );
    let forms = crate::play::feature_0115_mail::view::forms::all();
    let atlas = forms
        .iter()
        .find(|form| form.title == "Map overview")
        .ok_or("missing overview presentation")?;
    assert!(atlas.read(&link).open);
    assert!(atlas.buttons.len() >= 16);
    Ok(())
}
