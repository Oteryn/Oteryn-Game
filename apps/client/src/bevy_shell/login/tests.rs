use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
use crate::play::screenshot::TestDir as Temp;
fn settings(temp: &Temp) -> Result<LoginSettings, Box<dyn std::error::Error>> {
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let path = temp.join("root.der");
    std::fs::write(&path, certificate.cert.der())?;
    Ok(LoginSettings {
        platform_url: "http://127.0.0.1:18564".into(),
        gateway_url: "http://127.0.0.1:18565".into(),
        oauth_client_id: "native-test".into(),
        dev_root: path.to_string_lossy().into_owned(),
        ..Default::default()
    })
}
#[test]
fn shared_login_request_exposes_form_on_every_native_platform() -> TestResult {
    let mut host = HostOwner::new(None)?;
    host.request_login();
    assert!(host.login.visible);
    assert!(host.modal_active());
    assert!(!host.status.contains("admission is unavailable"));
    host.close();
    assert!(!host.login.visible);
    assert!(!host.modal_active());
    Ok(())
}
#[test]
fn malformed_login_settings_and_invalid_submit_remain_visible_without_admission() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    std::fs::write(&path, b"not-json")?;
    let mut host = HostOwner::new(None)?;
    host.configure_login(Some(path.clone()), |_| None);
    host.request_login();
    assert!(host.login.form.message().contains("invalid"));
    host.submit_login();
    assert_eq!(host.phase, super::super::owner::Phase::Offline);
    assert!(!host.login.form.pending());
    assert!(host.login.visible);
    assert_eq!(std::fs::read(path)?, b"not-json");
    Ok(())
}
#[test]
fn existing_login_owner_saves_public_configuration_and_rejects_blocked_save() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let mut state = LoginState::default();
    state.load(Some(path.clone()), |_| None);
    state.form.settings = settings(&temp)?;
    let expected = state.form.settings.clone();
    let config = state.submit()?;
    assert_eq!(config.platform_url, expected.platform_url);
    assert!(state.form.pending());
    assert!(LoginSettings::load(&path)? == expected);
    assert!(state.submit().is_err());
    state
        .form
        .finish(Err(crate::GameplayEntryError::NativeProtocolUnavailable));
    std::fs::remove_file(&path)?;
    std::fs::create_dir(&path)?;
    assert!(state.submit().is_err());
    assert!(!state.form.pending());
    assert!(state.form.message().contains("Cannot save"));
    Ok(())
}
#[test]
fn text_input_values_and_native_activate_reach_existing_validation() -> TestResult {
    use bevy::asset::AssetPlugin;
    let temp = Temp::new()?;
    let mut host = HostOwner::new(None)?;
    host.configure_login(Some(temp.join("login.json")), |_| None);
    host.request_login();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .insert_non_send(host)
    .add_systems(Startup, (super::super::ui::setup, setup).chain())
    .add_systems(Update, project);
    app.update();
    for (field, mut input) in app
        .world_mut()
        .query::<(&LoginField, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 0 {
            *input = EditableText::new("https://example.invalid?token=refused");
        }
    }
    let submit = app
        .world_mut()
        .query::<(Entity, &LoginControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| matches!(control, LoginControl::Submit).then_some(entity))
        .ok_or("submit")?;
    app.world_mut().trigger(Activate { entity: submit });
    app.update();
    let host = app.world().non_send::<HostOwner>();
    assert_eq!(host.phase, super::super::owner::Phase::Offline);
    assert!(!host.login.form.pending());
    assert!(host.login.visible);
    assert_eq!(
        host.login.form.settings.platform_url,
        "https://example.invalid?token=refused"
    );
    assert!(!temp.join("login.json").exists());
    Ok(())
}
#[test]
fn keyboard_dispatch_reaches_editable_render_node_without_image_measure_owner() -> TestResult {
    use bevy::{
        asset::AssetPlugin,
        input::{
            InputPlugin,
            keyboard::{Key, KeyboardInput},
        },
        input_focus::InputDispatchPlugin,
        text::{TextLayoutInfo, TextPlugin},
        ui_widgets::TextInputPlugin,
        window::{PrimaryWindow, WindowPlugin},
    };
    let mut host = HostOwner::new(None)?;
    host.request_login();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        InputPlugin,
        InputDispatchPlugin,
        WindowPlugin::default(),
        TextPlugin,
        TextInputPlugin,
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .init_resource::<bevy::ui::UiScale>()
    .init_resource::<bevy::picking::events::PointerState>()
    .add_message::<bevy::picking::events::PointerRelease>()
    .insert_non_send(host)
    .add_systems(Startup, (super::super::ui::setup, setup).chain())
    .add_systems(Update, project);
    app.update();
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())?;
    let input = app
        .world_mut()
        .query::<(Entity, &LoginField)>()
        .iter(app.world())
        .find_map(|(entity, field)| (field.0 == 0).then_some(entity))
        .ok_or("platform input")?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(input, FocusCause::Navigated);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyN,
        logical_key: Key::Character("n".into()),
        state: bevy::input::ButtonState::Pressed,
        text: Some("n".into()),
        repeat: false,
        window,
    });
    app.update();
    assert_eq!(
        app.world()
            .entity(input)
            .get::<EditableText>()
            .ok_or("editable")?
            .value()
            .to_string(),
        "n"
    );
    assert!(
        app.world().entity(input).contains::<TextLayoutInfo>(),
        "actual input has text render layout"
    );
    assert!(
        !app.world().entity(input).contains::<ImageNode>(),
        "text render node must not share the image intrinsic-size writer"
    );
    Ok(())
}
#[test]
fn opening_and_reopening_form_preserves_upstream_editor_styles() -> TestResult {
    use bevy::{
        asset::AssetPlugin,
        text::{FontSource, RemSize},
    };
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .init_resource::<RemSize>()
    .insert_non_send(HostOwner::new(None)?)
    .add_systems(Startup, (super::super::ui::setup, setup).chain())
    .add_systems(Update, project)
    .add_systems(PostUpdate, bevy::ui::widget::update_editable_text_styles);
    app.update();
    let input = app
        .world_mut()
        .query::<(Entity, &LoginField)>()
        .iter(app.world())
        .find_map(|(entity, field)| (field.0 == 0).then_some(entity))
        .ok_or("platform input")?;
    app.world_mut()
        .entity_mut(input)
        .get_mut::<TextFont>()
        .ok_or("font")?
        .font = FontSource::Family("OterynRegressionFamily".into());
    app.update();
    let configured = app
        .world()
        .entity(input)
        .get::<EditableText>()
        .ok_or("editor")?
        .editor()
        .get_styles()
        .inner()
        .clone();
    assert!(
        configured.len() > 3,
        "upstream styled editor before form opened"
    );
    for value in ["http://127.0.0.1:18564", "https://example.invalid"] {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.login.form.settings.platform_url = value.into();
        owner.request_login();
        app.update();
        let entity = app.world().entity(input);
        let text = entity.get::<EditableText>().ok_or("editor")?;
        assert_eq!(text.value().to_string(), value);
        assert_eq!(
            text.editor().get_styles().inner(),
            &configured,
            "form open retains upstream family, size and shaping styles"
        );
        app.world_mut().non_send_mut::<HostOwner>().login.visible = false;
        app.update();
    }
    Ok(())
}

#[test]
fn exit_confirmation_fences_login_keyboard_and_native_activate() -> TestResult {
    use bevy::{
        asset::AssetPlugin,
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        window::{PrimaryWindow, WindowCloseRequested},
    };
    let mut host = HostOwner::new(None)?;
    host.settings.preferences.miscellaneous.confirm_exit = true;
    host.request_login();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .add_message::<KeyboardInput>()
        .insert_non_send(host)
        .add_systems(Startup, (super::super::ui::setup, setup).chain())
        .add_systems(Update, (keyboard, project).chain());
    super::super::game_ui::modal::install(&mut app);
    super::super::exit::install(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    app.update();
    let cancel = app
        .world_mut()
        .query::<(Entity, &LoginControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| matches!(control, LoginControl::Cancel).then_some(entity))
        .ok_or("login cancel")?;
    app.world_mut()
        .write_message(WindowCloseRequested { window });
    app.update();
    app.world_mut().trigger(Activate { entity: cancel });
    assert!(
        app.world().non_send::<HostOwner>().login.visible,
        "direct Activate behind quit confirmation must be fenced"
    );
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: Key::Escape,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert!(
        app.world().non_send::<HostOwner>().login.visible,
        "one Escape cancels quit, preserving the underlying login form"
    );
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: Key::Escape,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert!(
        !app.world().non_send::<HostOwner>().login.visible,
        "next Escape reaches the exposed login form"
    );
    Ok(())
}

mod text_input;

fn login_app(host: HostOwner) -> App {
    use bevy::asset::AssetPlugin;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(host)
        .add_systems(Startup, (super::super::ui::setup, setup).chain())
        .add_systems(Update, (keyboard, project).chain());
    super::super::game_ui::modal::install(&mut app);
    app.update();
    app
}
fn named(app: &mut App, name: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, candidate)| (candidate.as_str() == name).then_some(entity))
        .ok_or_else(|| format!("{name} absent").into())
}
fn control(app: &mut App, wanted: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &LoginControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == wanted).then_some(entity))
        .ok_or_else(|| format!("{wanted} absent").into())
}
/// Button captions directly under `parent`, in order.
fn captions(app: &mut App, parent: Entity) -> Vec<String> {
    let world = app.world();
    world
        .get::<Children>(parent)
        .into_iter()
        .flat_map(|children| children.iter())
        .filter_map(|button| world.get::<Children>(button))
        .flat_map(|children| children.iter())
        .filter_map(|caption| world.get::<Text>(caption).map(|text| text.0.clone()))
        .collect()
}
#[test]
fn login_window_is_the_280_mask_with_advanced_fields_hidden() -> TestResult {
    let mut host = HostOwner::new(None)?;
    host.request_login();
    let mut app = login_app(host);
    let window = named(&mut app, "login.window")?;
    assert_eq!(
        app.world().get::<Node>(window).ok_or("node")?.width,
        px(280.)
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Login")
    );
    let fields = named(&mut app, "login.advanced_fields")?;
    assert_eq!(
        app.world().get::<Node>(fields).ok_or("fields")?.display,
        Display::None
    );
    let advanced = control(&mut app, "login.advanced")?;
    app.world_mut().trigger(Activate { entity: advanced });
    app.update();
    assert!(app.world().non_send::<HostOwner>().login.advanced);
    assert_eq!(
        app.world().get::<Node>(fields).ok_or("fields")?.display,
        Display::Flex
    );
    // The main row is Login and Cancel; secondary actions sit in the entry top bar.
    let submit = control(&mut app, "login.submit")?;
    let footer = app.world().get::<ChildOf>(submit).ok_or("footer")?.parent();
    assert_eq!(captions(&mut app, footer), ["Login", "Cancel"]);
    let top = named(&mut app, "login.top_bar")?;
    assert_eq!(
        captions(&mut app, top),
        [
            "News",
            "About",
            "Survey",
            "Support",
            "Cookie policy",
            "Sign out device"
        ]
    );
    assert!(
        app.world_mut()
            .query::<&super::super::ui::ArtSlot>()
            .iter(app.world())
            .any(|slot| *slot == super::super::ui::ArtSlot::LoginBackground)
    );
    Ok(())
}
#[test]
fn login_errors_open_a_titled_message_box_instead_of_dialog_prose() -> TestResult {
    let mut host = HostOwner::new(None)?;
    host.request_login();
    let mut app = login_app(host);
    let alert = app
        .world_mut()
        .query_filtered::<Entity, With<LoginAlert>>()
        .single(app.world())?;
    assert_eq!(
        app.world().get::<Node>(alert).ok_or("alert")?.display,
        Display::None
    );
    let message = app
        .world()
        .non_send::<HostOwner>()
        .login
        .form
        .message()
        .to_owned();
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == message),
        "guidance prose is not printed in the dialog"
    );
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .login
        .form
        .finish(Err(crate::GameplayEntryError::WorldUnavailable));
    app.update();
    assert_eq!(
        app.world().get::<Node>(alert).ok_or("alert")?.display,
        Display::Flex
    );
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(texts.iter().any(|text| text == "Login Error"));
    assert!(
        texts
            .iter()
            .any(|text| text.starts_with("World is absent from the Platform directory"))
    );
    let ok = control(&mut app, "login.alert.ok")?;
    app.world_mut().trigger(Activate { entity: ok });
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .login
            .form
            .alert()
            .is_none()
    );
    assert_eq!(
        app.world().get::<Node>(alert).ok_or("alert")?.display,
        Display::None
    );
    Ok(())
}

#[test]
fn review_3376_sign_in_uses_the_visible_platform() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let visible = settings(&temp)?;
    visible.save(&path)?;
    let mut state = LoginState::default();
    state.load(Some(path.clone()), |_| None);
    let mut another = visible.clone();
    another.platform_url = "http://127.0.0.1:18566".into();
    another.save(&path)?;
    assert_eq!(state.submit()?.platform_url, visible.platform_url);
    Ok(())
}
