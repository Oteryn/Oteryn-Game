use super::*;
use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_ui::wheel_allocation::{atelier, wheel};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn ready() -> (
    crate::play::PlayLink,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
) {
    let (link, commands, outbox) = play_channel();
    link.wheel_test_update(|panel, _| {
        *panel = WheelPanel {
            selected: true,
            synchronized: true,
            open: true,
            confirmed_wheel: Some(wheel::WheelAllocationReply {
                refused: false,
                revision: 7,
                points: [0; 36],
                vessels: None,
            }),
            confirmed_atelier: Some(atelier::AtelierReply {
                refused: false,
                revision: 11,
                gems: vec![atelier::GemView {
                    id: [17; 16],
                    quality: 0,
                    domain: 0,
                    locked: false,
                    vessel: false,
                    modifiers: vec![1],
                }],
                grades: vec![],
                grade_quotes: vec![],
                quality_quotes: vec![atelier::QualityQuote {
                    quality: 0,
                    reveal_gold: 10,
                    switch_gold: 10,
                    fragment_kind: 0,
                    fragment_min: 1,
                    fragment_max: 2,
                }],
            }),
            ..Default::default()
        };
    });
    (link, commands, outbox)
}
#[test]
fn wheel_window_shows_server_allocation_only() -> Result {
    let (link, _, _) = ready();
    let p = link.wheel_panel();
    assert!(link.wheel_dispatch_bound(
        &p,
        Kind::Points {
            slot: 14,
            points: 1
        }
    ));
    link.wheel_test_update(|p, _| {
        use oteryn_client_runtime::wheel_presentation::{Reply, Request};
        let request = Request {
            nonce: 1,
            wheel_revision: 7,
            atelier_revision: 11,
            points: p.points,
            vessels: p.vessels,
        };
        p.diagram.selected = true;
        p.diagram.last = Some(request.clone());
        p.diagram.reply = Some(Reply {
            request,
            catalogue: "test owner".into(),
            vocation: "Knight".into(),
            available: 123,
            unlock_points: 123,
            removal_permitted: true,
            eligible: true,
            draft_valid: true,
            reason: String::new(),
            nodes: vec![],
            revelations: vec![],
            current: vec![],
            draft: vec![],
        });
    });
    let p = link.wheel_panel();
    let text = summary(&p);
    assert!(text.contains("Available points: 123"));
    assert!(text.contains("Revision: 7"));
    assert!(text.contains("Allocated points: 0"));
    assert!(text.contains("Edited points: 1"));
    assert_eq!(p.confirmed_wheel.as_ref().ok_or("wheel")?.points, [0; 36]);
    let mut app = window(link)?;
    let texts = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect::<Vec<_>>();
    assert!(texts.iter().any(|text| text.contains("Revision: 7")));
    assert_eq!(
        texts
            .iter()
            .filter(|text| text.contains(" · allocated "))
            .count(),
        36
    );
    Ok(())
}
#[test]
fn wheel_allocation_dispatches_revision_bound() -> Result {
    let (link, mut commands, outbox) = ready();
    assert!(link.wheel_dispatch_bound(
        &link.wheel_panel(),
        Kind::Points {
            slot: 14,
            points: 1
        }
    ));
    let p = link.wheel_panel();
    assert!(link.wheel_dispatch_bound(&p, Kind::Request(Intent::Save)));
    assert!(!link.wheel_dispatch_bound(&p, Kind::Request(Intent::Save)));
    let PlayCommand::Wheel(c) = commands.try_recv()? else {
        return Err("command".into());
    };
    let r = wheel::decode_wheel_request(&c.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(r.expected_revision, 7);
    assert_eq!(r.points[14], 1);
    assert_eq!(
        link.wheel_panel().confirmed_wheel.ok_or("wheel")?.points[14],
        0
    );
    assert!(commands.try_recv().is_err());
    let mut server = link.wheel_panel().confirmed_wheel.ok_or("wheel")?;
    server.revision = 8;
    // The authoritative response may differ from the proposed allocation.
    server.points[14] = 2;
    let wheel_bytes = wheel::encode_wheel_reply(&server).map_err(|e| format!("{e:?}"))?;
    let atelier_bytes =
        atelier::encode_reply(&link.wheel_panel().confirmed_atelier.ok_or("atelier")?)
            .map_err(|e| format!("{e:?}"))?;
    outbox.wheel_test_result(&oteryn_client_ui::wheel_allocation::WheelResponse {
        kind: c.kind,
        status: oteryn_client_ui::wheel_allocation::CommandStatus::Accepted,
        payload: wheel_bytes.clone(),
        wheel: Some(wheel_bytes),
        atelier: Some(atelier_bytes),
        resources: None,
    });
    assert_eq!(link.wheel_panel().confirmed_wheel.ok_or("wheel")?, server);
    assert_eq!(link.wheel_panel().points[14], 2);
    assert!(link.wheel_dispatch_bound(
        &link.wheel_panel(),
        Kind::Points {
            slot: 14,
            points: 1
        }
    ));
    assert!(link.wheel_dispatch_bound(&link.wheel_panel(), Kind::Request(Intent::Save)));
    let PlayCommand::Wheel(c) = commands.try_recv()? else {
        return Err("command".into());
    };
    let r = wheel::decode_wheel_request(&c.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((r.expected_revision, r.points[14]), (8, 1));
    assert_eq!(
        link.wheel_panel().confirmed_wheel.ok_or("wheel")?.points[14],
        2
    );
    Ok(())
}
#[test]
fn wheel_gem_action_requires_confirm_and_revision() -> Result {
    let (link, mut commands, _) = ready();
    assert!(link.wheel_dispatch_bound(&link.wheel_panel(), Kind::Gem([17; 16])));
    assert!(link.wheel_dispatch_bound(&link.wheel_panel(), Kind::Socket));
    assert_eq!(link.wheel_panel().vessels[0], Some([17; 16]));
    assert!(
        !link
            .wheel_panel()
            .confirmed_atelier
            .as_ref()
            .ok_or("atelier")?
            .gems[0]
            .vessel
    );
    assert!(link.wheel_dispatch_bound(&link.wheel_panel(), Kind::Clear));
    assert_eq!(link.wheel_panel().vessels[0], None);
    assert!(link.wheel_dispatch_bound(&link.wheel_panel(), Kind::Request(Intent::Dismantle)));
    assert!(commands.try_recv().is_err());
    let p = link.wheel_panel();
    assert!(p.gems.review.is_some());
    assert!(!link.wheel_dispatch_bound(&p, Kind::Request(Intent::Save)));
    assert!(link.wheel_dispatch_bound(&p, Kind::Confirm));
    assert!(!link.wheel_dispatch_bound(&p, Kind::Confirm));
    let PlayCommand::Wheel(c) = commands.try_recv()? else {
        return Err("command".into());
    };
    let r = atelier::decode_mutate(&c.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(r.expected_revision, 11);
    assert_eq!(
        link.wheel_panel()
            .confirmed_atelier
            .ok_or("atelier")?
            .gems
            .len(),
        1
    );
    Ok(())
}
#[test]
fn wheel_presets_reuse_existing_owner() -> Result {
    let (link, _, _) = ready();
    let path = std::env::temp_dir().join(format!("uni6-presets-{}.json", std::process::id()));
    assert!(link.wheel_preset_bound(&link.wheel_panel(), 0, "uni6", &path));
    assert!(link.wheel_dispatch_bound(
        &link.wheel_panel(),
        Kind::Points {
            slot: 14,
            points: 1
        }
    ));
    assert!(link.wheel_preset_bound(&link.wheel_panel(), 1, "uni6", &path));
    assert_eq!(link.wheel_panel().points[14], 0);
    let book = oteryn_client_ui::wheel_allocation::presets::PresetBook::load(&path)?;
    assert_eq!(book, link.wheel_panel().preset_book);
    std::fs::remove_file(path)?;
    Ok(())
}
#[test]
fn wheel_actions_rejected_when_session_replaced_or_stale_revision() -> Result {
    let (link, mut commands, _) = ready();
    let p = link.wheel_panel();
    link.wheel_test_update(|p, _| {
        if let Some(w) = &mut p.confirmed_wheel {
            w.revision += 1;
        }
    });
    let actions = [
        Kind::Close,
        Kind::Metadata,
        Kind::Socket,
        Kind::Clear,
        Kind::Vessel(0),
        Kind::Preset {
            action: 0,
            name: "blocked".into(),
        },
        Kind::Points {
            slot: 14,
            points: 1,
        },
        Kind::Request(Intent::Save),
        Kind::Gem([17; 16]),
        Kind::Confirm,
        Kind::Cancel,
    ];
    for action in &actions {
        let before = link.wheel_panel();
        assert!(!link.wheel_dispatch_bound(&p, action.clone()));
        assert_eq!(link.wheel_panel(), before);
    }
    let p = link.wheel_panel();
    link.wheel_test_update(|_, shutdown| *shutdown = true);
    for action in actions {
        assert!(!link.wheel_dispatch_bound(&p, action));
        assert_eq!(link.wheel_panel(), p);
    }
    assert!(commands.try_recv().is_err());
    Ok(())
}
fn window(link: crate::play::PlayLink) -> std::result::Result<App, Box<dyn std::error::Error>> {
    use bevy::{asset::AssetPlugin, input_focus::InputFocus};
    link.use_shared_frontend();
    let mut owner = HostOwner::new(None)?;
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok(app)
}
#[test]
fn wheel_modal_rejects_controls_and_replacement_session_callbacks() -> Result {
    let (link, mut commands, outbox) = ready();
    let mut app = window(link)?;
    let controls = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .map(|(e, c)| (e, c.clone()))
        .collect::<Vec<_>>();
    assert!(!controls.is_empty());
    let before = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .wheel_panel();
    let modal = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    for (entity, _) in &controls {
        app.world_mut().trigger(Activate { entity: *entity });
    }
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .wheel_panel(),
        before
    );
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(modal);
    app.update();
    let (link, mut replacement_commands, _) = ready();
    outbox.wheel_test_end();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let generation = owner.generation();
        owner.poll([960, 640]);
        assert!(owner.generation() > generation);
        let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
    }
    let owner = app.world().non_send::<HostOwner>();
    let state = app.world().resource::<State>();
    for (_, control) in controls {
        assert!(!perform(owner, state, &control));
    }
    assert!(replacement_commands.try_recv().is_err());
    Ok(())
}
#[test]
fn wheel_v2_equips_and_unequips_through_revision_bound_owner() -> Result {
    for equipped in [false, true] {
        let (link, mut commands, _) = ready();
        link.wheel_test_update(|p, _| {
            p.atelier_v2 = true;
            p.gem = Some([17; 16]);
            p.vessel_board = Some(oteryn_client_ui::gem_atelier::VesselBoard {
                wheel_revision: 7,
                in_temple: true,
                premium: true,
                slots: (0..12)
                    .map(|slot| atelier::v2::VesselSlotView {
                        quality: slot / 4,
                        open: true,
                        gem: if equipped && slot == 0 {
                            Some([17; 16])
                        } else {
                            None
                        },
                    })
                    .collect(),
                ..Default::default()
            });
        });
        let p = link.wheel_panel();
        assert!(link.wheel_dispatch_bound(&p, if equipped { Kind::Clear } else { Kind::Socket }));
        assert!(!link.wheel_dispatch_bound(&p, if equipped { Kind::Clear } else { Kind::Socket }));
        let PlayCommand::Wheel(c) = commands.try_recv()? else {
            return Err("command".into());
        };
        let request =
            atelier::v2::decode_replace_vessel(&c.payload).map_err(|e| format!("{e:?}"))?;
        assert_eq!(request.expected_wheel_revision, 7);
        assert_eq!(request.gem, if equipped { None } else { Some([17; 16]) });
        assert_eq!(link.wheel_panel().vessel_board, p.vessel_board);
    }
    Ok(())
}
#[test]
fn wheel_presets_owner_open_flag_projects_and_closes() -> Result {
    let (link, _, _) = ready();
    link.wheel_test_update(|p, _| {
        p.open = false;
        p.presets_open = true;
    });
    let mut app = window(link)?;
    assert!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .next()
            .is_some()
    );
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    link.wheel_test_update(|p, _| p.selected = false);
    assert!(link.wheel_dispatch_bound(&link.wheel_panel(), Kind::Close));
    assert!(!link.wheel_panel().open);
    assert!(!link.wheel_panel().presets_open);
    Ok(())
}
