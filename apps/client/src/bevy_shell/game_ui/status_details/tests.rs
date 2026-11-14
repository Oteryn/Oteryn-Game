use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_ui::{mechanics_status, party_effects, potion_durations};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.status_details_test_selected();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn dispatch(app: &App, action: Action) -> bool {
    let owner = app.world().non_send::<HostOwner>();
    let Some(play) = &owner.play else {
        return false;
    };
    let expected = play.link.status_details_panel();
    play.link.status_details_dispatch_bound(&expected, action)
}
fn query(commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>, tab: Tab) -> Result {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature request missing".into());
    };
    let expected = match tab {
        Tab::Party => (280, 496),
        Tab::Potions => (381, 662),
        Tab::Mechanics => (384, 669),
    };
    assert_eq!((command.capability, command.kind), expected);
    assert!(command.payload.is_empty());
    assert!(commands.try_recv().is_err());
    Ok(())
}
fn publish(outbox: &Outbox, tab: Tab) -> Result {
    let payload = match tab {
        Tab::Party => party_effects::PartyEffects {
            in_party: true,
            shared_enabled: true,
            shared_active: false,
            eligible: false,
            bonuses: [6, 12, 6, 12, 50],
            party_spell_remaining_ms: [0, 120000, 0, 0, 0],
        }
        .encode(),
        Tab::Potions => potion_durations::PotionDurations {
            active: vec![potion_durations::PotionDuration {
                source: 3,
                item_id: 0,
                percent: 50,
                remaining_ms: 120000,
            }],
        }
        .encode(),
        Tab::Mechanics => mechanics_status::MechanicsStatus {
            observed_unix_ms: 100000,
            dust: 17,
            dust_limit: 100,
            deaths: vec![],
            timers: vec![],
            members: vec![mechanics_status::MemberEffects {
                character: [1; 16],
                name: "Server member".into(),
                conditions: None,
                bonuses: [0; 5],
                party_spell_remaining_ms: [0; 5],
            }],
        }
        .encode(),
    }
    .ok_or("encode")?;
    outbox.status_details_test_reply(tab, true, &payload);
    Ok(())
}
fn shown(app: &mut App) -> String {
    app.world_mut()
        .query_filtered::<&Text, With<Body>>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn status_details_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    for tab in [Tab::Party, Tab::Potions, Tab::Mechanics] {
        assert!(dispatch(&app, Action::Open(tab)));
        query(&mut commands, tab)?;
        app.update();
        assert!(shown(&mut app).contains("Loading"));
        publish(&outbox, tab)?;
        app.update();
        let owner = app.world().non_send::<HostOwner>();
        let snapshot = owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .status_details_panel();
        let expected = snapshot
            .text_at(tab, std::time::Instant::now())
            .replace("\r\n", "\n");
        assert_eq!(shown(&mut app), expected);
        match tab {
            Tab::Party => assert!(expected.contains("Protect Party: 120s")),
            Tab::Potions => assert!(expected.contains("Store XP boost +50%: 02:00")),
            Tab::Mechanics => {
                assert!(expected.contains("Dust: 17 / 100"));
                assert!(expected.contains("Server member"));
                assert!(expected.contains("Effects not observed in this Channel"));
            }
        }
    }
    Ok(())
}
#[test]
fn status_details_actions_are_bound_and_confirmed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    for tab in [Tab::Party, Tab::Potions, Tab::Mechanics] {
        assert!(dispatch(&app, Action::Open(tab)));
        query(&mut commands, tab)?;
        assert!(!dispatch(&app, Action::Refresh(tab)));
        publish(&outbox, tab)?;
        app.update();
        let before = shown(&mut app);
        let refresh = control(&mut app, Action::Refresh(tab))?;
        app.world_mut().trigger(Activate { entity: refresh });
        query(&mut commands, tab)?;
        app.world_mut().trigger(Activate { entity: refresh });
        assert!(commands.try_recv().is_err());
        app.update();
        assert_eq!(shown(&mut app), before);
        outbox.status_details_test_reply(tab, false, &[]);
        app.update();
        assert!(shown(&mut app).contains("Refresh"));
        assert!(dispatch(&app, Action::Close(tab)));
        app.update();
    }
    Ok(())
}
#[test]
fn status_details_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    assert!(dispatch(&app, Action::Open(Tab::Party)));
    query(&mut commands, Tab::Party)?;
    publish(&outbox, Tab::Party)?;
    app.update();
    let old = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .status_details_panel();
    assert!(dispatch(&app, Action::Open(Tab::Potions)));
    query(&mut commands, Tab::Potions)?;
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    let before = link.status_details_panel();
    assert!(!link.status_details_dispatch_bound(&old, Action::Refresh(Tab::Party)));
    assert_eq!(link.status_details_panel(), before);
    let (link, mut replacement_commands, replacement_outbox) = play_channel();
    link.use_shared_frontend();
    replacement_outbox.status_details_test_selected();
    assert!(
        link.status_details_dispatch_bound(&link.status_details_panel(), Action::Open(Tab::Party))
    );
    query(&mut replacement_commands, Tab::Party)?;
    publish(&replacement_outbox, Tab::Party)?;
    let replacement = link.status_details_panel();
    // Identical visible values in a different session still reject the old binding.
    assert!(!link.status_details_dispatch_bound(&old, Action::Refresh(Tab::Party)));
    assert_eq!(link.status_details_panel(), replacement);
    assert!(replacement_commands.try_recv().is_err());
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    publish(&outbox, Tab::Potions)?;
    app.update();
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .status_details_panel(),
        replacement
    );
    assert!(shown(&mut app).contains("Protect Party: 120s"));
    assert!(!shown(&mut app).contains("Store XP boost"));
    assert!(dispatch(&app, Action::Open(Tab::Mechanics)));
    query(&mut replacement_commands, Tab::Mechanics)?;
    replacement_outbox.status_details_test_end();
    assert!(!dispatch(&app, Action::Refresh(Tab::Mechanics)));
    assert!(replacement_commands.try_recv().is_err());
    app.update();
    assert!(shown(&mut app).is_empty());
    Ok(())
}
#[test]
fn status_details_window_registered() -> Result {
    assert!(include_str!("../mod.rs").contains("status_details::install(app)"));
    assert!(include_str!("../../ui.rs").contains("Control::StatusDetails"));
    let (mut app, mut commands, _) = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::StatusDetails).then_some(e))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    query(&mut commands, Tab::Party)?;
    app.update();
    assert!(!shown(&mut app).is_empty());
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(shown(&mut app).is_empty());
    Ok(())
}

fn control(
    app: &mut App,
    action: Action,
) -> std::result::Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.action == action).then_some(e))
        .ok_or("control".into())
}
fn key(key_code: KeyCode, repeat: bool) -> bevy::input::keyboard::KeyboardInput {
    bevy::input::keyboard::KeyboardInput {
        key_code,
        logical_key: bevy::input::keyboard::Key::Unidentified(
            bevy::input::keyboard::NativeKey::Unidentified,
        ),
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat,
        window: Entity::PLACEHOLDER,
    }
}
#[test]
fn status_details_refresh_and_shortcut_respect_other_modals() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    app.init_resource::<ButtonInput<KeyCode>>();
    for modifier in [KeyCode::ControlLeft, KeyCode::AltLeft, KeyCode::ShiftLeft] {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(modifier);
    }
    app.world_mut().write_message(key(KeyCode::F11, false));
    app.update();
    query(&mut commands, Tab::Party)?;
    publish(&outbox, Tab::Party)?;
    app.update();
    let root = app
        .world()
        .resource::<State>()
        .widgets
        .as_ref()
        .ok_or("window")?
        .root;
    let refresh = control(&mut app, Action::Refresh(Tab::Party))?;
    let before = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .status_details_panel();
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(modal, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().trigger(Activate { entity: refresh });
    app.world_mut().resource_mut::<State>().next_refresh = Some(Instant::now());
    app.world_mut().write_message(key(KeyCode::F11, false));
    app.update();
    let after = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .status_details_panel();
    assert_eq!(after, before);
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(modal);
    app.update();
    app.world_mut().resource_mut::<State>().next_refresh = Some(Instant::now());
    app.update();
    query(&mut commands, Tab::Party)?;
    assert_eq!(
        app.world()
            .resource::<State>()
            .widgets
            .as_ref()
            .ok_or("window")?
            .root,
        root
    );
    publish(&outbox, Tab::Party)?;
    app.update();
    app.world_mut().write_message(key(KeyCode::F11, true));
    app.update();
    assert!(!shown(&mut app).is_empty());
    app.world_mut().write_message(key(KeyCode::F11, false));
    app.update();
    assert!(shown(&mut app).is_empty());
    Ok(())
}

#[test]
fn status_details_refresh_preserves_scroll() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    assert!(dispatch(&app, Action::Open(Tab::Mechanics)));
    query(&mut commands, Tab::Mechanics)?;
    publish(&outbox, Tab::Mechanics)?;
    app.update();
    let scroll = |app: &mut App| -> std::result::Result<Entity, Box<dyn std::error::Error>> {
        app.world_mut()
            .query_filtered::<&ChildOf, With<Body>>()
            .iter(app.world())
            .next()
            .map(ChildOf::parent)
            .ok_or("scroll area".into())
    };
    let area = scroll(&mut app)?;
    let position = Vec2::new(0., 135.);
    app.world_mut()
        .entity_mut(area)
        .insert(bevy::ui::ScrollPosition(position));
    assert!(dispatch(&app, Action::Refresh(Tab::Mechanics)));
    query(&mut commands, Tab::Mechanics)?;
    app.update();
    let area = scroll(&mut app)?;
    assert_eq!(
        app.world()
            .get::<bevy::ui::ScrollPosition>(area)
            .map(|value| value.0),
        Some(position)
    );
    publish(&outbox, Tab::Mechanics)?;
    app.update();
    let area = scroll(&mut app)?;
    assert_eq!(
        app.world()
            .get::<bevy::ui::ScrollPosition>(area)
            .map(|value| value.0),
        Some(position)
    );
    Ok(())
}
