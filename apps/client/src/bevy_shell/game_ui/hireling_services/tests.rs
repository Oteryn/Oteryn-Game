use crate::{
    bevy_shell::{
        owner::{ActivePlay, HostOwner, Phase},
        ui::{self, Control},
    },
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::hireling::{HirelingPanel, Intent, panel_text};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.hireling_services_test_initialize();
    link.hireling_target(7, 9, -1);
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .insert_non_send(owner)
        .add_systems(Startup, ui::setup);
    super::super::modal::install(&mut app);
    super::install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, caption: &str) -> Result {
    let entity = app
        .world_mut()
        .query_filtered::<(Entity, &Children), With<bevy::ui_widgets::Button>>()
        .iter(app.world())
        .find_map(|(entity, children)| {
            children
                .iter()
                .any(|child| {
                    app.world()
                        .get::<Text>(child)
                        .is_some_and(|t| t.0 == caption)
                })
                .then_some(entity)
        })
        .ok_or_else(|| format!("missing hireling control: {caption}"))?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn panel(app: &App) -> std::result::Result<HirelingPanel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .hireling_panel())
}
fn open(app: &mut App) -> Result {
    click(app, "Hireling services (Ctrl+Shift+H)")?;
    app.update();
    Ok(())
}
fn request(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> std::result::Result<oteryn_client_ui::hireling::hireling::Request, Box<dyn std::error::Error>>
{
    let PlayCommand::Hireling(command) = commands.try_recv()? else {
        return Err("wrong request".into());
    };
    Ok(command.0)
}
#[test]
fn hireling_services_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    open(&mut app)?;
    assert_eq!(
        request(&mut commands)?,
        oteryn_client_ui::hireling::hireling::Request::Query
    );
    assert!(panel(&app)?.active.is_none());
    outbox.hireling_services_test_response(true);
    app.update();
    let expected = panel_text(&panel(&app)?);
    let texts: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(texts.contains(&expected));
    assert!(texts.contains(&"Target: 7, 9, floor -1".to_owned()));
    assert!(!texts.iter().any(|text| text.contains("gold")));
    Ok(())
}
#[test]
fn hireling_services_actions_are_bound_and_confirmed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    open(&mut app)?;
    let _ = request(&mut commands)?;
    outbox.hireling_services_test_response(false);
    app.update();
    for (label, intent, active) in [
        ("Summon banker", Intent::Summon, true),
        ("Dismiss", Intent::Dismiss, false),
    ] {
        click(&mut app, label)?;
        app.update();
        assert!(commands.try_recv().is_err());
        assert_eq!(panel(&app)?.active, Some(!active));
        click(&mut app, "Confirm")?;
        click(&mut app, "Confirm")?;
        let expected = match intent {
            Intent::Summon => oteryn_client_ui::hireling::hireling::Request::Summon {
                x: 7,
                y: 9,
                floor: -1,
            },
            _ => oteryn_client_ui::hireling::hireling::Request::Dismiss,
        };
        assert_eq!(request(&mut commands)?, expected);
        assert!(commands.try_recv().is_err());
        assert_eq!(panel(&app)?.active, Some(!active));
        outbox.hireling_services_test_response(active);
        app.update();
        assert_eq!(panel(&app)?.active, Some(active));
    }
    click(&mut app, "Refresh")?;
    click(&mut app, "Refresh")?;
    assert_eq!(
        request(&mut commands)?,
        oteryn_client_ui::hireling::hireling::Request::Query
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn hireling_services_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    open(&mut app)?;
    let _ = request(&mut commands)?;
    outbox.hireling_services_test_response(false);
    app.update();
    click(&mut app, "Summon banker")?;
    app.update();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .hireling_target(8, 9, -1);
    let before = panel(&app)?;
    click(&mut app, "Confirm")?;
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.update();
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(250), Node::default()))
        .id();
    click(&mut app, "Refresh")?;
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    outbox.hireling_services_test_end();
    click(&mut app, "Refresh")?;
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().close();
    click(&mut app, "Refresh")?;
    assert!(commands.try_recv().is_err());
    let (link, mut replacement_commands, replacement_outbox) = play_channel();
    link.use_shared_frontend();
    replacement_outbox.hireling_services_test_initialize();
    link.hireling_target(8, 9, -1);
    assert!(link.hireling_services_dispatch_bound(
        &link.hireling_services_snapshot(),
        super::Action::Toggle
    ));
    let _ = request(&mut replacement_commands)?;
    replacement_outbox.hireling_services_test_response(false);
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.play = Some(ActivePlay { view, link });
        owner.phase = Phase::Playing;
    }
    // Even with matching generation and panel values, the old mailbox binding is invalid.
    click(&mut app, "Refresh")?;
    assert!(replacement_commands.try_recv().is_err());
    app.update();
    click(&mut app, "Refresh")?;
    assert_eq!(
        request(&mut replacement_commands)?,
        oteryn_client_ui::hireling::hireling::Request::Query
    );
    Ok(())
}
#[test]
fn hireling_services_window_registered() -> Result {
    let (mut app, mut commands, _) = fixture()?;
    assert!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .any(|control| control.id() == "shell.hireling_services")
    );
    open(&mut app)?;
    let _ = request(&mut commands)?;
    assert!(panel(&app)?.open);
    click(&mut app, "Hireling services (Ctrl+Shift+H)")?;
    app.update();
    assert!(!panel(&app)?.open);
    owner_chord(&mut app)?;
    app.update();
    assert!(panel(&app)?.open);
    click(&mut app, "Close")?;
    app.update();
    assert!(!panel(&app)?.open);
    Ok(())
}

#[test]
fn hireling_services_close_refusal_is_visible_without_changing_owner() -> Result {
    let (mut app, mut commands, _) = fixture()?;
    open(&mut app)?;
    let _ = request(&mut commands)?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .hireling_target(8, 9, -1);
    let before = panel(&app)?;
    click(&mut app, "Close")?;
    app.update();
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| { text.0 == "Could not close hireling services. Please try again." })
    );
    click(&mut app, "Close")?;
    app.update();
    assert!(!panel(&app)?.open);
    Ok(())
}

fn owner_chord(app: &mut App) -> Result {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(11)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
            repeat: false,
        }],
        false,
    );
    Ok(())
}
#[test]
fn hireling_services_owner_chord_opens_and_f8_is_unclaimed() -> Result {
    use bevy::input::keyboard::{Key, KeyboardInput};
    let (mut app, mut commands, _) = fixture()?;
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::F8,
        logical_key: Key::F8,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    assert!(!panel(&app)?.open);
    assert!(commands.try_recv().is_err());
    owner_chord(&mut app)?;
    app.update();
    assert!(panel(&app)?.open);
    assert_eq!(
        request(&mut commands)?,
        oteryn_client_ui::hireling::hireling::Request::Query
    );
    Ok(())
}
