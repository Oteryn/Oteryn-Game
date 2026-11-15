use super::*;
use crate::{
    bevy_shell::owner::{ActivePlay, HostOwner, Phase},
    play::{PlayView, play_channel},
};
fn fixture(link: crate::play::PlayLink) -> Result<App, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    owner.phase = Phase::Playing;
    owner.play = Some(ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    Ok(app)
}
fn has_text(app: &mut App, text: &str) -> bool {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|t| t.0.contains(text))
}
fn has_window_title(app: &mut App) -> bool {
    app.world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .any(|(text, parent)| {
            text.0 == "Cyclopedia Map" && app.world().get::<GameModal>(parent.parent()).is_some()
        })
}
#[test]
fn cyclopedia_map_opens_in_bevy_and_closes_on_session_end() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, _receiver, _outbox) = play_channel();
    link.use_shared_frontend();
    link.cyclopedia_atlas_open();
    let mut app = fixture(link)?;
    assert!(has_window_title(&mut app));
    assert!(has_text(&mut app, "Unavailable in this session"));
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert!(!has_window_title(&mut app));
    Ok(())
}

#[test]
fn cyclopedia_map_bevy_browses_subareas_and_submits_poi_exploration()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{combat_input::PlayCommand, play::Outbox};
    use oteryn_client_runtime::{
        cyclopedia_map::{self as map, geometry as wire},
        raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
    };
    fn receive(
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    ) -> Result<FeatureCommand, Box<dyn std::error::Error>> {
        let PlayCommand::Feature(command) = rx.try_recv()? else {
            return Err("feature command".into());
        };
        assert_eq!(command.capability, 57);
        Ok(command)
    }
    fn reply(outbox: &Outbox, command: &FeatureCommand, payload: Vec<u8>) {
        outbox.bevy_cyclopedia_map_reply(
            command,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload,
            },
        );
    }
    fn click(app: &mut App, kind: Kind) -> Result<(), Box<dyn std::error::Error>> {
        let entity = app
            .world_mut()
            .query::<(Entity, &Action)>()
            .iter(app.world())
            .find_map(|(e, a)| (a.kind == kind).then_some(e))
            .ok_or("missing control")?;
        app.world_mut().trigger(Activate { entity });
        app.update();
        Ok(())
    }
    let (link, mut rx, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_cyclopedia_map_selected();
    link.cyclopedia_atlas_open();
    let mut app = fixture(link)?;
    let regions = receive(&mut rx)?;
    assert_eq!(
        wire::decode_request(&regions.payload)
            .map_err(|e| format!("{e:?}"))?
            .area,
        0
    );
    reply(
        &outbox,
        &regions,
        wire::encode_reply(&wire::Reply::Areas(wire::AreaPage {
            parent: 0,
            after: 0,
            next: None,
            areas: vec![wire::Area {
                id: 1,
                parent: 0,
                name: "Harbour".into(),
            }],
        }))
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Atlas(3))?;
    let subareas = receive(&mut rx)?;
    assert_eq!(
        wire::decode_request(&subareas.payload)
            .map_err(|e| format!("{e:?}"))?
            .area,
        1
    );
    reply(
        &outbox,
        &subareas,
        wire::encode_reply(&wire::Reply::Areas(wire::AreaPage {
            parent: 1,
            after: 0,
            next: None,
            areas: vec![wire::Area {
                id: 2,
                parent: 1,
                name: "Quays".into(),
            }],
        }))
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert!(has_text(&mut app, "Quays"));
    click(&mut app, Kind::Atlas(5))?;
    let geometry = receive(&mut rx)?;
    let request = wire::decode_request(&geometry.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((request.area, request.floor), (2, -7));
    reply(
        &outbox,
        &geometry,
        wire::encode_reply(&wire::Reply::Geometry(wire::GeometryPage {
            area: 2,
            floor: -7,
            page: 0,
            next: None,
            total: 1,
            bounds: [100, 100, 102, 100],
            runs: vec![wire::Run {
                y: 100,
                begin: 100,
                end: 103,
                subarea: 2,
            }],
        }))
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Explore(Intent::Start))?;
    let start = receive(&mut rx)?;
    assert_eq!(
        (start.kind, &start.payload),
        (
            map::request(1, 0).map_err(|e| format!("{e:?}"))?.0,
            &map::request(1, 0).map_err(|e| format!("{e:?}"))?.1
        )
    );
    let speed = receive(&mut rx)?;
    reply(&outbox, &speed, 0u16.to_le_bytes().to_vec());
    reply(
        &outbox,
        &start,
        map::encode_result(&map::MapResult {
            code: 0,
            revision: 1,
            discovered: false,
            found: 1,
            area_percent: 20,
            pool: 0,
            epoch: 1,
            improved: false,
            visible: vec![(101, 100, -7)],
        })
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert!(has_text(&mut app, "◆"));
    assert!(has_text(&mut app, "Found 1/7"));
    click(&mut app, Kind::Explore(Intent::Find))?;
    let find = receive(&mut rx)?;
    assert_eq!(
        find.payload,
        map::request(3, 0).map_err(|e| format!("{e:?}"))?.1
    );
    click(&mut app, Kind::Close)?;
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .cyclopedia_map_panel()
            .geometry
            .open
    );
    Ok(())
}
