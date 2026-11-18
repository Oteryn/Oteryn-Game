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
        .add_plugins(bevy::text::TextPlugin)
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

#[test]
fn cyclopedia_map_bevy_resets_pois_donates_and_pages_the_region_atlas()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::{
        cyclopedia_map::{self as map, region_atlas},
        raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
    };
    let (link, mut rx, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_cyclopedia_map_selected();
    link.cyclopedia_atlas_open();
    let mut app = fixture(link)?;
    let receive = |rx: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>|
     -> Result<FeatureCommand, Box<dyn std::error::Error>> {
        let PlayCommand::Feature(command) = rx.try_recv()? else {
            return Err("feature command".into());
        };
        assert_eq!(command.capability, 57);
        Ok(command)
    };
    let reply = |command: &FeatureCommand, payload: Vec<u8>| {
        outbox.bevy_cyclopedia_map_reply(
            command,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload,
            },
        );
    };
    let click = |app: &mut App, kind: Kind| -> Result<(), Box<dyn std::error::Error>> {
        let entity = app
            .world_mut()
            .query::<(Entity, &Action)>()
            .iter(app.world())
            .find_map(|(e, a)| (a.kind == kind).then_some(e))
            .ok_or("missing control")?;
        app.world_mut().trigger(Activate { entity });
        app.update();
        Ok(())
    };
    let regions = receive(&mut rx)?;
    reply(
        &regions,
        map::geometry::encode_reply(&map::geometry::Reply::Areas(map::geometry::AreaPage {
            parent: 0,
            after: 0,
            next: None,
            areas: vec![map::geometry::Area {
                id: 1,
                parent: 0,
                name: "Harbour".into(),
            }],
        }))
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    let result = map::encode_result(&map::MapResult {
        code: 0,
        revision: 2,
        discovered: false,
        found: 0,
        area_percent: 20,
        pool: 0,
        epoch: 2,
        improved: false,
        visible: vec![],
    })
    .map_err(|e| format!("{e:?}"))?;
    click(&mut app, Kind::Explore(Intent::Shuffle))?;
    let shuffle = receive(&mut rx)?;
    assert_eq!(
        (shuffle.kind, &shuffle.payload),
        (73, &map::request(2, 0).map_err(|e| format!("{e:?}"))?.1)
    );
    let speed = receive(&mut rx)?;
    assert_eq!(speed.kind, 6178);
    reply(&speed, 0u16.to_le_bytes().to_vec());
    reply(&shuffle, result.clone());
    app.update();
    assert!(has_text(&mut app, "Found 0/7"));
    for mut gold in app
        .world_mut()
        .query_filtered::<&mut EditableText, With<Donation>>()
        .iter_mut(app.world_mut())
    {
        *gold = EditableText::new("500");
    }
    click(&mut app, Kind::Explore(Intent::Donate))?;
    let donate = receive(&mut rx)?;
    assert_eq!(
        (donate.kind, &donate.payload),
        (73, &map::request(4, 500).map_err(|e| format!("{e:?}"))?.1)
    );
    reply(&donate, result);
    app.update();
    click(&mut app, Kind::Explore(Intent::RegionAtlas))?;
    let atlas = receive(&mut rx)?;
    assert_eq!(
        (atlas.kind, &atlas.payload),
        (73, &map::request(5, 0).map_err(|e| format!("{e:?}"))?.1)
    );
    reply(
        &atlas,
        region_atlas::encode(&region_atlas::Page {
            next: None,
            areas: vec![
                region_atlas::Area {
                    id: 1,
                    parent: 0,
                    name: "Harbour".into(),
                },
                region_atlas::Area {
                    id: 2,
                    parent: 1,
                    name: "Quays".into(),
                },
            ],
            runs: vec![region_atlas::Run {
                area: 2,
                x: 100,
                end: 103,
                y: 100,
                floor: -7,
            }],
        })
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert!(has_text(&mut app, "Harbour > Quays [#2]"));
    assert!(has_text(&mut app, "(complete)"));
    Ok(())
}

type Ready = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    crate::play::Outbox,
);
/// An open, selected Cyclopedia Map window whose region list the server already answered.
fn ready() -> Result<Ready, Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_map::geometry as wire;
    let (link, mut rx, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_cyclopedia_map_selected();
    link.cyclopedia_atlas_open();
    let mut app = fixture(link)?;
    let regions = feature(&mut rx)?;
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
    Ok((app, rx, outbox))
}
fn feature(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
) -> Result<oteryn_client_runtime::raw_feature::FeatureCommand, Box<dyn std::error::Error>> {
    let crate::combat_input::PlayCommand::Feature(command) = rx.try_recv()? else {
        return Err("feature command".into());
    };
    assert_eq!(command.capability, 57);
    Ok(command)
}
fn reply(
    outbox: &crate::play::Outbox,
    command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    payload: Vec<u8>,
) {
    use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
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
fn press(app: &mut App, kind: Kind) -> Result<(), Box<dyn std::error::Error>> {
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
fn gold(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    Ok(app
        .world_mut()
        .query_filtered::<&EditableText, With<Donation>>()
        .single(app.world())?
        .value()
        .to_string())
}

include!("donation_cases.rs");
