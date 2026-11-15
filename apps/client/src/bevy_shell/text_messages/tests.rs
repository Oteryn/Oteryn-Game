use super::*;
use crate::play::{PlayView, ReferencePreferences, play_channel};
use oteryn_session::{AppliedDelta, ServerMessage, SessionEvent};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn otc_textmsg_setup_mounts_wrapping_noninteractive_areas_in_world() -> TestResult {
    let mut app = App::new();
    let world = app.world_mut().spawn_empty().id();
    app.insert_resource(GameRegions {
        vitals: world,
        minimap: world,
        equipment: world,
        containers: world,
        battle: world,
        skills: world,
        vip: world,
        analyzers: world,
        chat: world,
        bars: world,
        world,
    })
    .insert_resource(Skins {
        buttons: oteryn_client_ui_bevy::ButtonImages {
            up: default(),
            down: default(),
        },
        checked: default(),
        unchecked: default(),
        frame: default(),
        inset: default(),
        font: default(),
    })
    .add_systems(Startup, setup);
    app.update();
    let mut count = 0;
    for (_, node, layout, pickable, parent) in app
        .world_mut()
        .query::<(&ScreenArea, &Node, &TextLayout, &Pickable, &ChildOf)>()
        .iter(app.world())
    {
        assert_eq!(parent.parent(), world);
        assert_eq!(node.width, percent(80.));
        assert_eq!(layout.justify, Justify::Center);
        assert_eq!(layout.linebreak, TextLayout::default().linebreak);
        assert_eq!(*pickable, Pickable::IGNORE);
        count += 1;
    }
    assert_eq!(count, 4);
    Ok(())
}

#[test]
fn otc_textmsg_look_is_green_and_replaces_only_its_own_area() -> TestResult {
    let mut state = ScreenMessages::default();
    state.show(Area::Center, "warning", [245, 94, 94], Duration::ZERO);
    state.observe_look(
        &LookMessage {
            receipt: 1,
            text: "You see a sword.".into(),
        },
        Duration::ZERO,
    );
    let look = state
        .visible(Area::Look, Duration::ZERO)
        .ok_or("look area")?;
    assert_eq!(look.text, "You see a sword.");
    assert_eq!(look.colour, [0, 235, 0]);
    assert_eq!(
        state
            .visible(Area::Center, Duration::ZERO)
            .ok_or("warning")?
            .text,
        "warning"
    );
    state.observe_look(
        &LookMessage {
            receipt: 2,
            text: "You see a shield.".into(),
        },
        Duration::from_secs(1),
    );
    assert_eq!(
        state
            .visible(Area::Look, Duration::from_secs(1))
            .ok_or("replacement")?
            .text,
        "You see a shield."
    );
    assert!(state.visible(Area::Look, Duration::from_secs(5)).is_none());
    Ok(())
}

#[test]
fn otc_textmsg_default_bars_and_explicit_saved_choice() -> TestResult {
    let defaults = ReferencePreferences::default();
    assert!(defaults.creature_bars);
    assert_eq!(defaults.hud_names, [true; 3]);
    let mut saved = serde_json::to_value(defaults)?;
    saved
        .as_object_mut()
        .ok_or("settings object")?
        .remove("creature_bars");
    assert!(serde_json::from_value::<ReferencePreferences>(saved.clone())?.creature_bars);
    saved["creature_bars"] = false.into();
    assert!(!serde_json::from_value::<ReferencePreferences>(saved)?.creature_bars);
    Ok(())
}

#[test]
fn otc_textmsg_routes_real_server_delivery_and_keeps_chat() -> TestResult {
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    let (link, _commands, outbox) = play_channel();
    let messages = [
        (
            ServerMessage::StatusText("status".into()),
            Area::Bottom,
            [255; 3],
        ),
        (
            ServerMessage::Warning("warning".into()),
            Area::Center,
            [245, 94, 94],
        ),
        (
            ServerMessage::Advance("advance".into()),
            Area::Top,
            [255; 3],
        ),
    ];
    for (index, (value, _, _)) in messages.iter().enumerate() {
        outbox.push(vec![SessionEvent::ServerMessage(AppliedDelta {
            server_sequence: index as u64 + 1,
            base_revision: index as u64,
            new_revision: index as u64 + 1,
            value: value.clone(),
        })]);
    }
    view.tick(&link).map_err(|_| "tick")?;
    let mut state = ScreenMessages::default();
    state.observe(&view, Duration::ZERO);
    for (message, area, colour) in messages {
        let text = match message {
            ServerMessage::StatusText(text)
            | ServerMessage::Warning(text)
            | ServerMessage::Advance(text) => text,
        };
        let shown = state
            .visible(area, Duration::ZERO)
            .ok_or("routed message")?;
        assert_eq!(shown.text, text);
        assert_eq!(shown.colour, colour);
        assert!(
            link.chat_panel()
                .console
                .notices
                .text(Default::default(), Default::default())
                .contains(&text)
        );
    }
    assert_eq!(view.server_messages().count(), 3);
    assert!(view.notification_text(false).is_empty());
    assert!(view.notification_text(true).is_empty());
    Ok(())
}

#[test]
fn otc_textmsg_expiry_replacement_and_identical_new_receipt() -> TestResult {
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    let (link, _commands, outbox) = play_channel();
    let mut state = ScreenMessages::default();
    for sequence in 1..=2 {
        outbox.push(vec![SessionEvent::ServerMessage(AppliedDelta {
            server_sequence: sequence,
            base_revision: sequence - 1,
            new_revision: sequence,
            value: ServerMessage::Warning("same".into()),
        })]);
        view.tick(&link).map_err(|_| "tick")?;
        state.observe(&view, Duration::from_secs(sequence - 1));
    }
    assert_eq!(
        state
            .visible(Area::Center, Duration::from_secs(1))
            .ok_or("replacement")?
            .text,
        "same"
    );
    assert!(
        state
            .visible(Area::Center, Duration::from_millis(4999))
            .is_some()
    );
    assert!(
        state
            .visible(Area::Center, Duration::from_secs(5))
            .is_none()
    );
    state.observe(&view, Duration::from_secs(6));
    assert!(
        state
            .visible(Area::Center, Duration::from_secs(6))
            .is_none()
    );
    state.show(
        Area::Center,
        "replacement",
        [245, 94, 94],
        Duration::from_secs(6),
    );
    assert_eq!(
        state
            .visible(Area::Center, Duration::from_secs(6))
            .ok_or("replacement")?
            .text,
        "replacement"
    );
    assert_eq!(visible_time("short"), Duration::from_secs(4));
    assert_eq!(visible_time(&"é".repeat(50)), Duration::from_secs(5));
    Ok(())
}

#[test]
fn otc_textmsg_existing_notice_controls_apply_to_screen_areas() -> TestResult {
    use crate::bevy_shell::owner::ActivePlay;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut host = HostOwner::new(None)?;
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    outbox.push(vec![SessionEvent::ServerMessage(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: ServerMessage::Warning("Danger".into()),
    })]);
    view.tick(&link).map_err(|_| "tick")?;
    view.configure_notifications(false, false);
    host.play = Some(ActivePlay { view, link });
    let mut app = App::new();
    app.insert_non_send(host)
        .insert_resource(Time::<()>::default())
        .insert_resource(Time::<Real>::default());
    let text = app
        .world_mut()
        .spawn((
            ScreenArea(Area::Center),
            Text::new(""),
            TextColor::default(),
        ))
        .id();
    app.init_resource::<ScreenMessages>()
        .add_systems(Update, project);
    app.update();
    assert!(app.world().get::<Text>(text).ok_or("text")?.0.is_empty());
    {
        let host = app.world().non_send::<HostOwner>();
        host.play
            .as_ref()
            .ok_or("play")?
            .view
            .configure_notifications(true, true);
    }
    app.update();
    assert_eq!(app.world().get::<Text>(text).ok_or("text")?.0, "Danger");
    {
        let host = app.world().non_send::<HostOwner>();
        assert!(
            host.play.as_ref().ok_or("play")?.view.notification_input(&[
                NormalizedInputEvent::Key {
                    code: KeyCode::new(69)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control)
                        .with(Modifier::Alt)
                        .with(Modifier::Shift),
                    repeat: false,
                }
            ])
        );
    }
    app.update();
    assert!(app.world().get::<Text>(text).ok_or("text")?.0.is_empty());
    Ok(())
}

#[test]
fn otc_textmsg_bevy_areas_clear_on_session_end() -> TestResult {
    use crate::bevy_shell::owner::ActivePlay;
    let mut host = HostOwner::new(None)?;
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    outbox.push(vec![SessionEvent::ServerMessage(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: ServerMessage::Warning("Danger".into()),
    })]);
    view.tick(&link).map_err(|_| "tick")?;
    host.play = Some(ActivePlay { view, link });
    let mut app = App::new();
    app.insert_non_send(host)
        .insert_resource(Time::<()>::default())
        .insert_resource(Time::<Real>::default());
    for area in Area::ALL {
        app.world_mut()
            .spawn((ScreenArea(area), Text::new(""), TextColor::default()));
    }
    app.init_resource::<ScreenMessages>()
        .add_systems(Update, project);
    app.update();
    for (area, text, colour) in app
        .world_mut()
        .query::<(&ScreenArea, &Text, &TextColor)>()
        .iter(app.world())
    {
        assert_eq!(text.0, if area.0 == Area::Center { "Danger" } else { "" });
        if area.0 == Area::Center {
            assert_eq!(colour.0, Color::srgb_u8(245, 94, 94));
        }
    }
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(4));
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .all(|text| text.0.is_empty())
    );
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert_eq!(app.world().resource::<ScreenMessages>().last_sequence, None);
    Ok(())
}
