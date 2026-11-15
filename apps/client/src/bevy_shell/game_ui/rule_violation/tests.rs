use super::*;
use oteryn_client_runtime::player_report as wire;
#[test]
fn rule_violation_reasons_from_protocol() {
    let reasons = reasons();
    assert_eq!(reasons.len(), usize::from(wire::MAX_REPORT_REASON) + 1);
    for (id, text) in reasons {
        assert!(text.contains(&format!("Reason {id}:")));
        assert!(text.contains("Evidence:"));
        assert!(!text.contains("Choose a report reason"));
    }
    assert_eq!(actions().len(), usize::from(wire::MAX_REPORT_ACTION) + 1);
}
#[test]
fn rule_violation_window_widgets_send_existing_intent() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        combat_input::PlayCommand,
        play::{PlayView, play_channel},
    };
    let (link, mut receiver, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_report_selected(true);
    link.rule_violation_open("Other Player");
    let mut owner = HostOwner::new(None)?;
    owner.phase = Phase::Playing;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
    });
    let mut app = App::new();
    app.insert_non_send(owner)
        .init_resource::<InputFocus>()
        .insert_resource(Skins {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            checked: Handle::default(),
            unchecked: Handle::default(),
            frame: Handle::default(),
            inset: Handle::default(),
            font: Handle::default(),
        });
    install(&mut app);
    app.update();
    let control = |app: &mut App, predicate: fn(Kind) -> bool| {
        app.world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find(|(_, control)| predicate(control.kind))
            .map(|(entity, _)| entity)
    };
    let send = control(&mut app, |kind| matches!(kind, Kind::Send)).ok_or("send")?;
    assert!(app.world().get::<InteractionDisabled>(send).is_some());
    for predicate in [
        (|kind| matches!(kind, Kind::Reason(6))) as fn(Kind) -> bool,
        |kind| matches!(kind, Kind::Action(1)),
    ] {
        let entity = control(&mut app, predicate).ok_or("choice")?;
        app.world_mut().trigger(Activate { entity });
    }
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 4 {
            *text = EditableText::new("Observed repeated messages");
        }
    }
    app.update();
    assert!(app.world().get::<InteractionDisabled>(send).is_none());
    app.world_mut().trigger(Activate { entity: send });
    app.update();
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("report".into());
    };
    let report =
        wire::decode_player_report_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((report.reason, report.action), (6, 1));
    assert_eq!(report.target, "Other Player");
    assert_eq!(report.comment, "Observed repeated messages");
    assert!(app.world().get::<InteractionDisabled>(send).is_some());
    Ok(())
}
#[test]
fn deferred_report_revalidates_foreground_modal_and_original_fields()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PlayView, play_channel};
    let (link, mut receiver, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_report_selected(true);
    link.rule_violation_open("Other Player");
    let mut owner = HostOwner::new(None)?;
    owner.phase = Phase::Playing;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
    });
    let mut app = App::new();
    app.insert_non_send(owner)
        .init_resource::<InputFocus>()
        .insert_resource(Skins {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            checked: Handle::default(),
            unchecked: Handle::default(),
            frame: Handle::default(),
            inset: Handle::default(),
            font: Handle::default(),
        });
    install(&mut app);
    app.update();
    let control = |app: &mut App, predicate: fn(Kind) -> bool| {
        app.world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find(|(_, control)| predicate(control.kind))
            .map(|(entity, _)| entity)
    };
    let send = control(&mut app, |kind| matches!(kind, Kind::Send)).ok_or("send")?;
    assert!(app.world().get::<InteractionDisabled>(send).is_some());
    for predicate in [
        (|kind| matches!(kind, Kind::Reason(6))) as fn(Kind) -> bool,
        |kind| matches!(kind, Kind::Action(1)),
    ] {
        let entity = control(&mut app, predicate).ok_or("choice")?;
        app.world_mut().trigger(Activate { entity });
    }
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 4 {
            *text = EditableText::new("Observed repeated messages");
        }
    }
    app.update();
    assert!(app.world().get::<InteractionDisabled>(send).is_none());

    let comment = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| (field.0 == 4).then_some(entity))
        .ok_or("comment")?;
    app.world_mut()
        .get_mut::<EditableText>(comment)
        .ok_or("comment editor")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    ));
    app.world_mut().trigger(Activate { entity: send });
    app.update();
    assert!(receiver.try_recv().is_err(), "paste is still pending");
    app.world_mut()
        .spawn((GameModal, GlobalZIndex(300), Node::default()));
    {
        let mut editor = app
            .world_mut()
            .get_mut::<EditableText>(comment)
            .ok_or("comment editor")?;
        editor.pending_paste = None;
        editor.editor_mut().set_text("Pasted evidence");
    }
    app.update();
    assert!(
        receiver.try_recv().is_err(),
        "deferred report must be cancelled behind the foreground modal"
    );
    Ok(())
}
#[test]
fn report_blocking_dialog_escape_respects_top_modal() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    outbox.bevy_report_selected(true);
    link.rule_violation_open("Other Player");
    let mut owner = HostOwner::new(None)?;
    owner.phase = Phase::Playing;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: crate::play::PlayView::new(TileCoord::new(100, 100), 7, &[])?,
    });
    let mut app = App::new();
    app.insert_non_send(owner)
        .init_resource::<InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_resource(Skins {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            checked: Handle::default(),
            unchecked: Handle::default(),
            frame: Handle::default(),
            inset: Handle::default(),
            font: Handle::default(),
        });
    install(&mut app);
    app.update();
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(300), Node::default()))
        .id();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_report_panel()
            .open
    );
    app.world_mut().despawn(higher);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .player_report_panel()
            .open
    );
    Ok(())
}
