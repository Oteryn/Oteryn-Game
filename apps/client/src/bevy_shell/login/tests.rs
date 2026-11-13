use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
type TestResult = Result<(), Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "bevy-login-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn settings(temp: &Temp) -> Result<LoginSettings, Box<dyn std::error::Error>> {
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let path = temp.0.join("root.der");
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
    let path = temp.0.join("login.json");
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
    let path = temp.0.join("login.json");
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
    host.configure_login(Some(temp.0.join("login.json")), |_| None);
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
    assert!(!temp.0.join("login.json").exists());
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
