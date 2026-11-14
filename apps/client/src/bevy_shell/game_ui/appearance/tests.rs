use super::*;
use crate::play::{
    PlayView,
    feature_0025_appearance::view::bevy_window_tests::{catalogue, publish},
    play_channel,
};
use bevy::asset::AssetPlugin;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn appearance_window_lists_server_unlocks_only() -> Result {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    publish(&outbox, catalogue());
    link.appearance_open();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    let kinds = app
        .world_mut()
        .query::<&Control>()
        .iter(app.world())
        .map(|control| control.kind.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        kinds
            .iter()
            .filter_map(|kind| if let Kind::Outfit(key) = kind {
                Some(key.as_str())
            } else {
                None
            })
            .collect::<Vec<_>>(),
        vec!["earned", "reward"]
    );
    assert_eq!(
        kinds
            .iter()
            .filter_map(|kind| if let Kind::Mount(key) = kind {
                key.as_deref()
            } else {
                None
            })
            .collect::<Vec<_>>(),
        vec!["earned-mount"]
    );
    assert_eq!(
        kinds
            .iter()
            .filter_map(|kind| if let Kind::Addon(bit) = kind {
                Some(*bit)
            } else {
                None
            })
            .collect::<Vec<_>>(),
        vec![1]
    );
    for part in 0..4 {
        assert_eq!(
            kinds
                .iter()
                .filter_map(|kind| if let Kind::Colour { part: p, index } = kind {
                    (*p == part).then_some(*index)
                } else {
                    None
                })
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Current outfit: earned"))
    );
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        1
    );
    // A higher local modal must fence an already rendered Apply button.
    let apply = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| matches!(control.kind, Kind::Apply(_)).then_some(entity))
        .ok_or("apply")?;
    let before = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .appearance_panel();
    let revision = app
        .world_mut()
        .query_filtered::<Entity, With<Revision>>()
        .single(app.world())?;
    app.world_mut()
        .entity_mut(revision)
        .insert(EditableText::new("r1"));
    let modal = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(250)))
        .id();
    app.world_mut().trigger(Activate { entity: apply });
    app.update();
    assert!(commands.try_recv().is_err());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .appearance_panel(),
        before
    );
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    app.world_mut().trigger(Activate { entity: apply });
    app.world_mut().trigger(Activate { entity: apply });
    app.update();
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("appearance command absent".into());
    };
    let selection = oteryn_client_runtime::appearance::Change::decode(&command.payload)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(selection.content_revision, "r1");
    assert_eq!(selection.prior, before.confirmed);
    assert!(commands.try_recv().is_err());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .appearance_panel()
            .confirmed,
        before.confirmed
    );
    Ok(())
}

fn fixture() -> std::result::Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    ),
    Box<dyn std::error::Error>,
> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    publish(&outbox, catalogue());
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands))
}
#[test]
fn appearance_owner_chord_opens_window_and_respects_blocking() -> Result {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, mut commands) = fixture()?;
    let chord = [NormalizedInputEvent::Key {
        code: KeyCode::new(4)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
        repeat: false,
    }];
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&chord, true);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        0
    );
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&chord, false);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        1
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn appearance_pending_edits_show_feedback_without_sending() -> Result {
    use std::sync::{Arc, Mutex};
    for paste in [false, true] {
        let (mut app, mut commands) = fixture()?;
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .appearance_open();
        app.update();
        let revision = app
            .world_mut()
            .query_filtered::<Entity, With<Revision>>()
            .single(app.world())?;
        {
            let mut field = app
                .world_mut()
                .get_mut::<EditableText>(revision)
                .ok_or("revision")?;
            if paste {
                field.pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::new(
                    Mutex::new(None),
                )));
            } else {
                field.queue_edit(bevy::text::TextEdit::Insert("r1".into()));
            }
        }
        let apply = app
            .world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find_map(|(entity, control)| matches!(control.kind, Kind::Apply(_)).then_some(entity))
            .ok_or("apply")?;
        app.world_mut().trigger(Activate { entity: apply });
        app.update();
        assert!(commands.try_recv().is_err());
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains("Finish editing"))
        );
        assert!(app.world().get_entity(revision).is_ok());
    }
    Ok(())
}
