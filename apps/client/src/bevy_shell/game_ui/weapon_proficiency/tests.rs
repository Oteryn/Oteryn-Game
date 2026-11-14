use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::{
    raw_feature::GatedCommandResult,
    weapon_proficiency::{
        ProficiencyCatalogueEntry, ProficiencyCatalogueLevel, ProficiencyCataloguePage,
    },
};
use oteryn_session::CommandStatus;
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();

    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
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
    outbox.bevy_weapon_proficiency_initialize();
    Ok((app, commands, outbox))
}
fn panel(app: &App) -> Result<Panel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .weapon_proficiency_panel())
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, c)| (c.kind == kind).then_some(entity))
        .ok_or("missing control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn command(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<oteryn_client_runtime::raw_feature::FeatureCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    Ok(command)
}
fn catalogue() -> ProficiencyCataloguePage {
    ProficiencyCataloguePage {
        offset: 0,
        total: 2,
        entries: (0..2)
            .map(|i| ProficiencyCatalogueEntry {
                definition: 258 + i,
                weapon: format!("weapon{i}"),
                name: format!("Server weapon {i}"),
                experience: 15,
                thresholds: [10, 20, 30, 40, 50, 60, 70, 80, 90],
                levels: vec![ProficiencyCatalogueLevel {
                    selected: Some(0),
                    perks: vec!["Server attack".into(), "Server defence".into()],
                }],
            })
            .collect(),
    }
}
fn publish(outbox: &Outbox, payload: Vec<u8>) {
    outbox.bevy_weapon_proficiency_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    });
}
fn loaded() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (mut app, mut commands, outbox) = fixture()?;
    open(app.world().non_send::<HostOwner>());
    assert_eq!(command(&mut commands)?.kind, 660);
    publish(&outbox, catalogue().encode().ok_or("encode")?);
    app.update();
    click(&mut app, Kind::Owner(Action::Weapon(0)))?;
    assert_eq!(command(&mut commands)?.kind, 732);
    publish(
        &outbox,
        oteryn_client_runtime::weapon_proficiency_state::State {
            definition: 258,
            revision: 7,
            slots: [None, None],
            modify_dust: [250, 1000],
            rank_dust: [25; 10],
            reshape_dust: 100,
            clear_dust: 10,
            orbs: 1,
        }
        .encode()
        .ok_or("state")?,
    );
    app.update();
    Ok((app, commands, outbox))
}
#[test]
fn weapon_proficiency_window_shows_server_data_only() -> TestResult {
    let (mut app, _commands, outbox) = loaded()?;
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        text.contains(&oteryn_client_ui::weapon_proficiency::panel_text(&panel(
            &app
        )?))
    );
    assert!(
        text.contains("Server weapon 0") && text.contains("Server attack") && text.contains("250")
    );
    outbox.bevy_weapon_proficiency_progress(258, 35);
    app.update();
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("Advancement: Server weapon 0"));
    click(&mut app, Kind::Owner(Action::Close))?;
    app.update();
    click(&mut app, Kind::Owner(Action::Track(true)))?;
    assert!(panel(&app)?.advancements.is_empty());
    Ok(())
}
#[test]
fn weapon_proficiency_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, mut commands, outbox) = loaded()?;
    let before = panel(&app)?;
    click(&mut app, Kind::Review(102))?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Confirm)?;
    click(&mut app, Kind::Confirm)?;
    let sent = command(&mut commands)?;
    let mut native = before.clone();
    let expected =
        crate::play::feature_0195_weapon_proficiency::view::bound::native_request(&mut native, 102)
            .ok_or("native request")?;
    assert_eq!(sent, expected);
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&app)?.shaping, before.shaping);
    assert_eq!(panel(&app)?.catalogue, before.catalogue);
    publish(
        &outbox,
        oteryn_client_runtime::weapon_proficiency::ModificationReply {
            result: oteryn_client_runtime::weapon_proficiency::ModificationResult::Accepted,
            slot: None,
        }
        .encode(),
    );
    assert!(panel(&app)?.shaping.is_none());
    Ok(())
}
#[test]
fn weapon_proficiency_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (mut app, mut commands, outbox) = loaded()?;
    click(&mut app, Kind::Review(102))?;
    app.update();
    outbox.bevy_weapon_proficiency_progress(258, 16);
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    app.update();
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(250), Node::default()))
        .id();
    let before = panel(&app)?;
    click(&mut app, Kind::Review(102))?;
    assert_eq!(panel(&app)?, before);
    assert!(app.world().resource::<State>().confirmation.is_none());
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    click(&mut app, Kind::Review(102))?;
    app.update();
    let (link, _receiver, new_outbox) = play_channel();
    link.use_shared_frontend();
    new_outbox.bevy_weapon_proficiency_initialize();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    assert!(!panel(&app)?.pending);
    outbox.bevy_weapon_proficiency_end();
    assert!(!panel(&app)?.open);
    Ok(())
}
#[test]
fn weapon_proficiency_window_registered() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.id() == "shell.weapon_proficiency").then_some(e))
        .ok_or("toolbar entry")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(panel(&app)?.open);
    assert_eq!(command(&mut commands)?.kind, 660);
    publish(
        &outbox,
        ProficiencyCataloguePage {
            offset: 0,
            total: 0,
            entries: Vec::new(),
        }
        .encode()
        .ok_or("catalogue")?,
    );
    app.update();
    click(&mut app, Kind::Owner(Action::Close))?;
    app.update();
    assert!(!panel(&app)?.open);
    let code = oteryn_input_actions::KeyCode::new(19)?;
    let key = || oteryn_input_actions::NormalizedInputEvent::Key {
        code,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key()], false);
    app.update();
    assert!(panel(&app)?.open);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key()], false);
    app.update();
    assert!(!panel(&app)?.open);
    assert!(include_str!("../mod.rs").contains("weapon_proficiency::install(app)"));
    assert!(include_str!("../../ui.rs").contains("Control::WeaponProficiency"));
    Ok(())
}
