use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_ui::mail::Panel;
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_mail_panel(Panel {
        open: true,
        selected: true,
        fields: ["4242".into(), "9000".into(), "Alice\nThais".into()],
        ..Default::default()
    });
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
    Ok((app, commands, outbox))
}
fn button(app: &mut App, intent: Option<Intent>) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.intent == intent).then_some(entity))
        .ok_or("mail button missing".into())
}
fn panel(app: &App) -> Result<Panel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play missing")?
        .link
        .mail_panel())
}
#[test]
fn mail_form_shows_server_bound_item() -> TestResult {
    let (mut app, _commands, _outbox) = fixture()?;
    let mut query = app.world_mut().query::<(
        &Field,
        Option<&EditableText>,
        Option<&Text>,
        Has<bevy::ui_widgets::TextInput>,
    )>();
    let fields: Vec<_> = query.iter(app.world()).collect();
    assert_eq!(fields.len(), 3);
    for (index, expected) in ["4242", "9000", "Alice\nThais"].iter().enumerate() {
        let (_, editable, label, input) = fields
            .iter()
            .find(|(field, ..)| field.0 == index)
            .ok_or("field missing")?;
        if index == 0 {
            assert!(editable.is_none());
            assert!(!input);
            assert_eq!(label.ok_or("item text missing")?.0, *expected);
        } else {
            let editable = editable.ok_or("editor missing")?;
            assert!(*input);
            assert_eq!(editable.value().to_string(), *expected);
            assert_eq!(editable.allow_newlines, index == 2);
            assert_eq!(editable.max_characters, Some(MAX_TEXT_BYTES));
        }
    }
    Ok(())
}
#[test]
fn mail_send_is_item_bound_and_single() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    let post = button(&mut app, Some(Intent::Post))?;
    app.world_mut().trigger(Activate { entity: post });
    app.world_mut().trigger(Activate { entity: post });
    app.update();
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!(
        oteryn_session::property_services::mail::decode_intent(&command.payload),
        Ok(oteryn_session::property_services::mail::MailIntent::Post {
            item_handle: 4242,
            mailbox_handle: 9000
        })
    );
    assert!(commands.try_recv().is_err());
    assert!(panel(&app)?.pending);
    assert!(panel(&app)?.feedback.is_empty());
    Ok(())
}
#[test]
fn text_form_edit_is_item_revision_bound() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let write = button(&mut app, Some(Intent::Write))?;
    // An owner update between activation and dispatch invalidates the rendered snapshot.
    app.world_mut().trigger(Activate { entity: write });
    let mut newer = panel(&app)?;
    newer.fields[2] = "new owner text".into();
    outbox.bevy_mail_panel(newer.clone());
    app.update();
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&app)?, newer);
    let write = button(&mut app, Some(Intent::Write))?;
    app.world_mut().trigger(Activate { entity: write });
    app.update();
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!(
        oteryn_session::property_services::mail::decode_intent(&command.payload),
        Ok(
            oteryn_session::property_services::mail::MailIntent::WriteText {
                item_handle: 4242,
                text: "new owner text".into()
            }
        )
    );
    Ok(())
}
#[test]
fn mail_rejected_when_session_replaced_or_item_changed() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    let post = button(&mut app, Some(Intent::Post))?;
    let before = panel(&app)?;
    let blocker = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(250), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: post });
    app.update();
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&app)?, before);
    app.world_mut().despawn(blocker);
    app.update();
    let _server_modal = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bevy_server_modal_fixture();
    app.world_mut().trigger(Activate { entity: post });
    app.update();
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&app)?, before);
    Ok(())
}

#[test]
fn mail_shortcut_and_editor_use_existing_owner() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let mut closed = panel(&app)?;
    closed.open = false;
    outbox.bevy_mail_panel(closed);
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[oteryn_input_actions::NormalizedInputEvent::Key {
            code: oteryn_input_actions::KeyCode::new(16)?,
            state: oteryn_input_actions::ButtonState::Pressed,
            modifiers: oteryn_input_actions::Modifiers::one(
                oteryn_input_actions::Modifier::Control,
            )
            .with(oteryn_input_actions::Modifier::Alt),
            repeat: false,
        }],
        false,
    );
    app.update();
    assert!(panel(&app)?.open);
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 2 {
            text.editor_mut().set_text("edited letter");
        }
    }
    let write = button(&mut app, Some(Intent::Write))?;
    app.world_mut().trigger(Activate { entity: write });
    app.update();
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!(
        oteryn_session::property_services::mail::decode_intent(&command.payload),
        Ok(
            oteryn_session::property_services::mail::MailIntent::WriteText {
                item_handle: 4242,
                text: "edited letter".into()
            }
        )
    );
    Ok(())
}
