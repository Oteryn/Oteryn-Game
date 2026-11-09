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
    let (link, mut commands, _outbox) = play_channel();
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
    let mut p = link.cyclopedia_map_panel();
    p.atlas_layers = 15;
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
