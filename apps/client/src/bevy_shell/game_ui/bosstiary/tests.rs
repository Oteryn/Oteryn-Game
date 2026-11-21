use super::*;
#[test]
fn bosstiary_activation_is_fenced_by_foreground_map_modal() -> Result<(), Box<dyn std::error::Error>>
{
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    outbox.bevy_bosstiary_initialize();
    let generation = owner.generation();
    owner.phase = Phase::Playing;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    let mut app = App::new();
    app.insert_non_send(owner);
    let entity = app
        .world_mut()
        .spawn((
            Action {
                generation,
                kind: Kind::Request(Intent::Catalogue),
            },
            Node::default(),
        ))
        .observe(activate)
        .id();
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(250)))
        .id();
    app.world_mut().trigger(Activate { entity });
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(modal);
    app.world_mut().trigger(Activate { entity });
    assert!(commands.try_recv().is_ok());
    Ok(())
}
#[test]
fn bosstiary_toolbar_requests_catalogue_and_projects_window()
-> Result<(), Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.bevy_bosstiary_initialize();
    let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    crate::bevy_shell::ui::toolbar::open_toolbar(
        &mut owner,
        crate::bevy_shell::ui::Control::Panel("bosstiary"),
    );
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Feature(c) if c.kind == 133 && c.payload.starts_with(b"BCT1"))
    );
    let page = oteryn_client_runtime::boss_encounter::catalogue::Page {
        next: None,
        points: 10,
        entries: vec![oteryn_client_runtime::boss_encounter::catalogue::Entry {
            key: "test:boss".into(),
            name: "Confirmed boss".into(),
            category: 1,
            kills: 5,
            thresholds: [5, 10, 20],
            points: [10, 30, 60],
            look: 1,
            object: false,
        }],
    };
    outbox.bevy_bosstiary_result(&oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: page.encode().ok_or("catalogue")?,
    });
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<Window>>()
            .iter(app.world())
            .count(),
        1
    );
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(texts.iter().any(|text| text.contains("Confirmed boss")
        && text.contains("Prowess: 5/5")
        && text.contains("Mastery: 5/20")));
    Ok(())
}
#[test]
fn bosstiary_hazard_buttons_query_and_raise_the_server_hazard_level()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::raw_feature::GatedCommandResult;
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.bevy_bosstiary_initialize();
    let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    crate::bevy_shell::ui::toolbar::open_toolbar(
        &mut owner,
        crate::bevy_shell::ui::Control::Panel("bosstiary"),
    );
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 133));
    let page = oteryn_client_runtime::boss_encounter::catalogue::Page {
        next: None,
        points: 0,
        entries: vec![],
    };
    outbox.bevy_bosstiary_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: page.encode().ok_or("catalogue")?,
    });
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    app.init_resource::<bevy::input_focus::InputFocus>();
    crate::bevy_shell::ui::window_layout::install(&mut app);
    super::super::bestiary::install(&mut app);
    install(&mut app);
    app.update();
    let press = |app: &mut App, intent: Intent| -> Result<(), Box<dyn std::error::Error>> {
        let entity = app
            .world_mut()
            .query::<(Entity, &Action)>()
            .iter(app.world())
            .find_map(|(entity, action)| (action.kind == Kind::Request(intent)).then_some(entity))
            .ok_or("hazard button missing")?;
        app.world_mut().trigger(Activate { entity });
        Ok(())
    };
    let layers = |app: &mut App, id| {
        app.world_mut()
            .query::<(Entity, &crate::bevy_shell::ui::window_layout::WindowLayer)>()
            .iter(app.world())
            .find_map(|(entity, layer)| (layer.0 == id).then_some(entity))
            .ok_or("window missing")
    };
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .charm_progression_open();
    app.update();
    let bestiary = layers(&mut app, "bestiary")?;
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(bestiary, bevy::input_focus::FocusCause::Navigated);
    app.update();
    press(&mut app, Intent::HazardQuery)?;
    assert!(commands.try_recv().is_err());
    let boss = layers(&mut app, "bosstiary")?;
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(boss, bevy::input_focus::FocusCause::Navigated);
    app.update();
    press(&mut app, Intent::HazardQuery)?;
    let PlayCommand::Feature(query) = commands.try_recv()? else {
        return Err("hazard query missing".into());
    };
    assert_eq!((query.capability, query.kind), (117, 169));
    assert!(query.payload.is_empty());
    outbox.bevy_bosstiary_result(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: vec![5, 3],
    });
    app.update();
    let shown = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0.contains("Hazard level: 3 (unlocked 5)"));
    assert!(shown, "the server hazard level is not shown");
    press(&mut app, Intent::HazardRaise)?;
    let PlayCommand::Feature(raise) = commands.try_recv()? else {
        return Err("hazard raise missing".into());
    };
    assert_eq!((raise.kind, raise.payload), (169, vec![4]));
    Ok(())
}
fn click(app: &mut App, kind: Kind) -> Result<(), Box<dyn std::error::Error>> {
    app.update();
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, action)| action.kind == kind)
        .map(|(entity, _)| entity)
        .ok_or("bosstiary control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn window_text(app: &mut App) -> String {
    app.update();
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect::<Vec<_>>()
        .join("\n")
}
fn accepted(payload: Vec<u8>) -> oteryn_client_runtime::raw_feature::GatedCommandResult {
    oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload,
    }
}
/// A UUIDv7-shaped handoff, as the boss activity reply requires.
const HANDOFF: [u8; 16] = [7, 7, 7, 7, 7, 7, 0x77, 7, 0x87, 7, 7, 7, 7, 7, 7, 7];
#[test]
fn bosstiary_window_pulls_boss_lever_and_equips_boss_slot() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::boss_encounter::{
        BossActivityReply,
        bosstiary_slots::{BossSlotReply, BossSlotRequest, SlotDetails},
        encode_boss_activity_reply, encode_boss_key,
    };
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.bevy_bosstiary_initialize();
    let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    crate::bevy_shell::ui::toolbar::open_toolbar(
        &mut owner,
        crate::bevy_shell::ui::Control::Panel("bosstiary"),
    );
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 133));
    let page = oteryn_client_runtime::boss_encounter::catalogue::Page {
        next: None,
        points: 10,
        entries: vec![oteryn_client_runtime::boss_encounter::catalogue::Entry {
            key: "oteryn:creature.test_boss".into(),
            name: "Confirmed boss".into(),
            category: 1,
            kills: 5,
            thresholds: [5, 10, 20],
            points: [10, 30, 60],
            look: 1,
            object: false,
        }],
    };
    outbox.bevy_bosstiary_result(&accepted(page.encode().ok_or("catalogue")?));
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    assert!(window_text(&mut app).contains("Selected boss: Confirmed boss"));

    click(&mut app, Kind::Request(Intent::Enter))?;
    let PlayCommand::Feature(lever) = commands.try_recv()? else {
        return Err("boss lever command".into());
    };
    assert_eq!(
        (lever.capability, lever.kind, lever.payload),
        (
            117,
            178,
            encode_boss_key("oteryn:creature.test_boss").map_err(|e| format!("{e:?}"))?
        )
    );
    outbox.bevy_bosstiary_result(&accepted(
        encode_boss_activity_reply(&BossActivityReply::Accepted {
            handoff: HANDOFF,
            deadline_unix_ms: Some(90_000),
        })
        .map_err(|e| format!("{e:?}"))?,
    ));
    assert!(window_text(&mut app).contains("Boss entry accepted. Deadline: 90000"));
    for (kind, intent) in [(179, Intent::Defeated), (180, Intent::Timeout)] {
        click(&mut app, Kind::Request(intent))?;
        assert!(
            matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == kind && c.payload == encode_boss_key("oteryn:creature.test_boss").map_err(|e| format!("{e:?}"))?)
        );
        outbox.bevy_bosstiary_result(&accepted(
            encode_boss_activity_reply(&BossActivityReply::Accepted {
                handoff: HANDOFF,
                deadline_unix_ms: None,
            })
            .map_err(|e| format!("{e:?}"))?,
        ));
        assert!(window_text(&mut app).contains("Boss return accepted"));
    }

    let slots = |bosses: [&str; 2]| BossSlotReply {
        details: Some(SlotDetails {
            base_bonus: 4,
            bonuses: [4, 0],
            boosted: String::new(),
            now: 10,
        }),
        status: 0,
        unlocked: 1,
        bonus: 4,
        points: 10,
        remove_fee: 0,
        ready_at_ms: 0,
        bosses: bosses.map(str::to_owned),
    };
    click(&mut app, Kind::Request(Intent::SlotsDetailed))?;
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 491 && Some(c.payload.clone()) == BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)).encode())
    );
    outbox.bevy_bosstiary_result(&accepted(slots(["", ""]).encode().ok_or("slots")?));
    assert!(window_text(&mut app).contains("Slot 1: Empty"));
    click(&mut app, Kind::Slot)?;
    click(&mut app, Kind::SlotBoss(false))?;
    click(&mut app, Kind::Request(Intent::SlotEquipSelected))?;
    let PlayCommand::Feature(equip) = commands.try_recv()? else {
        return Err("boss slot command".into());
    };
    assert_eq!(equip.kind, 491);
    assert_eq!(
        Some(equip.payload),
        BossSlotRequest::Detailed(Box::new(BossSlotRequest::Equip {
            slot: 1,
            boss: "oteryn:creature.test_boss".into(),
        }))
        .encode()
    );
    outbox.bevy_bosstiary_result(&accepted(
        slots(["oteryn:creature.test_boss", ""])
            .encode()
            .ok_or("slots")?,
    ));
    assert!(window_text(&mut app).contains("> Slot 1: Confirmed boss | +4% equipment loot"));
    Ok(())
}
#[test]
fn bosstiary_entry_track_kills_toggle_adds_and_removes_the_boss_on_the_tracker()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::cyclopedia_tracker::{
        TrackerAction, TrackerEntry, TrackerRequest, TrackerSnapshot, encode_domain_snapshot,
        encode_request, encode_snapshot,
    };
    let tracker = |keys: &[&str]| TrackerSnapshot {
        boss: true,
        entries: keys
            .iter()
            .map(|key| TrackerEntry {
                creature_key: (*key).into(),
                kill_count: 1,
                thresholds: [5, 10, 20],
                completed: false,
            })
            .collect(),
    };
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.bevy_bosstiary_initialize();
    outbox.bevy_cyclopedia_tracker_initialize();
    let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    crate::bevy_shell::ui::toolbar::open_toolbar(
        &mut owner,
        crate::bevy_shell::ui::Control::Panel("bosstiary"),
    );
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 133));
    let page = two_boss_page();
    outbox.bevy_bosstiary_result(&accepted(page.encode().ok_or("catalogue")?));
    // The login join snapshot of domain 98: nothing tracked yet.
    outbox.bevy_cyclopedia_tracker_push(
        encode_domain_snapshot(
            &TrackerSnapshot {
                boss: false,
                entries: vec![],
            },
            &tracker(&[]),
        )
        .ok_or("domain snapshot")?,
    );
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    let toggle =
        |app: &mut App, key: &str| -> Result<(Entity, bool, bool), Box<dyn std::error::Error>> {
            app.update();
            let (entity, track) = app
                .world_mut()
                .query::<(Entity, &TrackToggle)>()
                .iter(app.world())
                .find_map(|(entity, toggle)| (toggle.key == key).then_some((entity, toggle.track)))
                .ok_or("track toggle")?;
            let disabled = app
                .world()
                .get::<bevy::ui::InteractionDisabled>(entity)
                .is_some();
            Ok((entity, track, disabled))
        };
    // No kill of Morgaroth yet: the server would refuse, so the toggle is disabled.
    assert!(toggle(&mut app, "oteryn:creature.morgaroth")?.2);
    let (entity, track, disabled) = toggle(&mut app, "oteryn:creature.ferumbras")?;
    assert!(track && !disabled);
    assert!(window_text(&mut app).contains("Track kills"));
    app.world_mut().trigger(Activate { entity });
    let PlayCommand::Feature(add) = commands.try_recv()? else {
        return Err("tracker add".into());
    };
    assert_eq!((add.capability, add.kind), (197, 384));
    assert_eq!(
        Some(add.payload),
        encode_request(&TrackerRequest::Change {
            action: TrackerAction::Add,
            creature_key: "oteryn:creature.ferumbras".into(),
        })
    );
    outbox.bevy_cyclopedia_tracker_result(&accepted(
        encode_snapshot(&tracker(&["oteryn:creature.ferumbras"])).ok_or("snapshot")?,
    ));
    let (entity, track, _) = toggle(&mut app, "oteryn:creature.ferumbras")?;
    assert!(!track);
    assert!(window_text(&mut app).contains("Stop tracking kills"));
    app.world_mut().trigger(Activate { entity });
    let PlayCommand::Feature(remove) = commands.try_recv()? else {
        return Err("tracker remove".into());
    };
    assert_eq!(
        Some(remove.payload),
        encode_request(&TrackerRequest::Change {
            action: TrackerAction::Remove,
            creature_key: "oteryn:creature.ferumbras".into(),
        })
    );
    outbox.bevy_cyclopedia_tracker_result(
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: oteryn_session::CommandStatus::Rejected,
            payload: Vec::new(),
        },
    );
    assert!(window_text(&mut app).contains("The server refused the tracker action"));
    Ok(())
}

fn two_boss_page() -> oteryn_client_runtime::boss_encounter::catalogue::Page {
    let entry =
        |key: &str, name: &str, kills| oteryn_client_runtime::boss_encounter::catalogue::Entry {
            key: key.into(),
            name: name.into(),
            category: 1,
            kills,
            thresholds: [5, 10, 20],
            points: [10, 30, 60],
            look: 1,
            object: false,
        };
    oteryn_client_runtime::boss_encounter::catalogue::Page {
        next: None,
        points: 10,
        entries: vec![
            entry("oteryn:creature.ferumbras", "Ferumbras", 1),
            entry("oteryn:creature.morgaroth", "Morgaroth", 0),
        ],
    }
}

#[test]
fn catalogue_page_controls_preserve_selected_boss() -> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.bevy_bosstiary_initialize();
    let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    crate::bevy_shell::ui::toolbar::open_toolbar(
        &mut owner,
        crate::bevy_shell::ui::Control::Panel("bosstiary"),
    );
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 133));
    let page = two_boss_page();
    outbox.bevy_bosstiary_result(&accepted(page.encode().ok_or("catalogue")?));
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    for intent in [Intent::CatalogueNext, Intent::CatalogueRefresh] {
        click(&mut app, Kind::Request(intent))?;
        assert!(window_text(&mut app).contains("Selected boss: Ferumbras"));
    }
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(c) if c.kind == 133));
    outbox.bevy_bosstiary_result(&accepted(page.encode().ok_or("catalogue")?));
    click(&mut app, Kind::Request(Intent::Enter))?;
    let PlayCommand::Feature(enter) = commands.try_recv()? else {
        return Err("enter boss room".into());
    };
    assert_eq!(
        enter.payload,
        oteryn_client_runtime::boss_encounter::encode_boss_key("oteryn:creature.ferumbras")
            .map_err(|e| format!("{e:?}"))?
    );
    Ok(())
}
