use super::*;
use crate::play::{PlayLink, PlayView, play_channel};
use bevy::asset::AssetPlugin;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn app() -> std::result::Result<(App, PlayLink), Box<dyn std::error::Error>> {
    let (link, receiver, _) = play_channel();
    link.use_shared_frontend();
    link.bevy_item_details_selected(true);
    // Keep the command channel alive for session admission.
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view,
        link: link.bevy_item_details_link(),
    });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .insert_non_send(owner)
    .insert_resource(Channel(receiver))
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    Ok((app, link))
}
#[derive(Resource)]
struct Channel(tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>);
#[test]
fn item_catalogue_widgets_use_owner_projection_and_all_controls() -> Result {
    let (mut app, link) = app()?;
    open(&app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let snapshot = link.item_catalogue_snapshot();
    let mut text = app
        .world_mut()
        .query_filtered::<&Text, With<CatalogueText>>();
    assert_eq!(text.single(app.world())?.0, snapshot.text);
    let mut fields = app.world_mut().query::<(&Field, &EditableText)>();
    let values = snapshot.fields();
    assert_eq!(fields.iter(app.world()).count(), 3);
    for (field, value) in fields.iter(app.world()) {
        assert_eq!(value.editor.text().to_string(), values[field.0]);
    }
    let mut controls = app.world_mut().query::<&Control>();
    let actions: Vec<_> = controls.iter(app.world()).map(|c| c.action).collect();
    for action in 0..10 {
        assert!(actions.contains(&Some(action)));
    }
    assert!(actions.contains(&None));
    assert!(
        app.world_mut()
            .resource_mut::<Channel>()
            .0
            .try_recv()
            .is_ok()
    );
    Ok(())
}
#[test]
fn item_catalogue_close_works_pending_unavailable_and_blocked_by_modal() -> Result {
    let (mut app, link) = app()?;
    open(&app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let mut controls = app.world_mut().query::<(Entity, &Control)>();
    let (entity, close) = controls
        .iter(app.world())
        .find(|(_, c)| c.action.is_none())
        .map(|(e, c)| (e, c.clone()))
        .ok_or("close")?;
    link.bevy_item_details_selected(false);
    app.world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()));
    app.world_mut().trigger(Activate { entity });
    assert!(!link.inspection_panel().catalogue_open);
    assert!(link.inspection_panel().pending);
    assert!(!perform(
        &app.world_mut().non_send_mut::<HostOwner>(),
        &Control {
            action: Some(0),
            ..close
        },
        [String::new(), String::new(), String::new()]
    ));
    app.update();
    open(&app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    assert!(link.inspection_panel().catalogue_open);
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(!link.inspection_panel().catalogue_open);
    assert!(link.inspection_panel().pending);
    Ok(())
}

fn chord()
-> std::result::Result<oteryn_input_actions::NormalizedInputEvent, oteryn_input_actions::InputError>
{
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    Ok(NormalizedInputEvent::Key {
        code: KeyCode::new(12)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
        repeat: false,
    })
}
#[test]
fn item_catalogue_window_registered() -> Result {
    let (mut app, link) = app()?;
    app.add_systems(Update, crate::bevy_shell::ui::project_toolbar);
    app.update();
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control, crate::bevy_shell::ui::Control::ItemCatalogue).then_some(entity)
        })
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(link.inspection_panel().catalogue_open);
    assert_eq!(
        app.world().get::<Node>(toolbar).ok_or("node")?.display,
        Display::Flex
    );
    let close = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| control.action.is_none().then_some(entity))
        .ok_or("close")?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(!link.inspection_panel().catalogue_open);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[chord()?], false);
    app.update();
    assert!(link.inspection_panel().catalogue_open);
    assert!(link.inspection_panel().pending);
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.update();
    assert_eq!(
        app.world().get::<Node>(toolbar).ok_or("node")?.display,
        Display::None
    );
    Ok(())
}
#[test]
fn item_catalogue_shortcut_respects_chat_and_modal_input_context() -> Result {
    let (mut app, link) = app()?;
    link.open_chat();
    for _ in 0..3 {
        if link.preference_chat_open() {
            break;
        }
        link.chat_console_mode();
    }
    assert!(link.preference_chat_open());
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[chord()?], false);
    assert!(!link.inspection_panel().catalogue_open);
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let arrow = NormalizedInputEvent::Key {
        code: KeyCode::ARROW_UP,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    let before = link.inspection_panel();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[arrow], false);
    assert_eq!(link.inspection_panel(), before);
    let _modal = link.bevy_server_modal_fixture();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[chord()?], false);
    assert!(!link.inspection_panel().catalogue_open);
    Ok(())
}
