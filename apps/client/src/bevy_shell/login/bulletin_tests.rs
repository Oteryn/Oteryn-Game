use super::*;
use crate::bevy_shell::{login, ui};
use bevy::asset::AssetPlugin;
type TestResult = Result<(), Box<dyn std::error::Error>>;
const DOCUMENT: &str = r#"{"news":[{"title":"Operator notice","body":"Published after the client release"}],"events":[{"year":2027,"month":1,"day":12,"title":"Winter gathering","body":"Meet at the town square"}]}"#;
fn app() -> Result<App, Box<dyn std::error::Error>> {
    let mut host = HostOwner::new(None)?;
    host.request_login();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(host)
        .add_systems(Startup, (ui::setup, login::setup).chain())
        .add_systems(Update, login::project);
    login::install(&mut app);
    app.update();
    Ok(app)
}
fn body(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    Ok(app
        .world_mut()
        .query_filtered::<&Text, With<Body>>()
        .single(app.world())?
        .0
        .clone())
}
fn activate_action(app: &mut App, action: Action) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, candidate)| (*candidate == action).then_some(entity))
        .ok_or("missing bulletin action")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn login_news_button_opens_readable_bulletin() -> TestResult {
    let mut app = app()?;
    let news = app
        .world_mut()
        .query::<(Entity, &LoginControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == "login.news").then_some(entity))
        .ok_or("missing login News button")?;
    app.world_mut().trigger(Activate { entity: news });
    app.update();
    assert!(body(&mut app)?.contains("No bulletin has been received"));
    assert_eq!(
        app.world_mut()
            .query_filtered::<&Node, With<Dialog>>()
            .single(app.world())?
            .display,
        Display::Flex
    );
    assert!(app.world().non_send::<HostOwner>().modal_active());
    Ok(())
}
#[test]
fn retained_bulletin_projects_news_calendar_and_survives_dismissal_and_invalid_update() -> TestResult
{
    let mut app = app()?;
    {
        let mut host = app.world_mut().non_send_mut::<HostOwner>();
        host.login.visible = false;
        host.login.bulletin.receive(DOCUMENT)?;
    }
    app.update();
    assert!(body(&mut app)?.contains("Operator notice\r\nPublished after the client release"));
    assert!(app.world().non_send::<HostOwner>().modal_active());
    assert!(app.world().resource::<InputFocus>().get().is_some());
    // Fix only the calendar cursor after verifying the retained publication was rendered.
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .login
        .bulletin
        .browser = Some(crate::bulletin::Browser::new(2027, 1, DOCUMENT)?);
    activate_action(&mut app, Action::Calendar)?;
    assert!(body(&mut app)?.contains("12: Winter gathering — Meet at the town square"));
    activate_action(&mut app, Action::Next)?;
    assert!(body(&mut app)?.contains("2027-02"));
    activate_action(&mut app, Action::Previous)?;
    assert!(body(&mut app)?.contains("2027-01"));
    activate_action(&mut app, Action::News)?;
    assert!(body(&mut app)?.contains("Published after the client release"));
    assert!(
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .login
            .bulletin
            .receive("invalid JSON")
            .is_err()
    );
    activate_action(&mut app, Action::Close)?;
    assert!(!app.world().non_send::<HostOwner>().modal_active());
    assert!(app.world().resource::<InputFocus>().get().is_none());
    use oteryn_input_actions::{
        ButtonState, KeyCode as InputKey, Modifier, Modifiers, NormalizedInputEvent,
    };
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[NormalizedInputEvent::Key {
            code: InputKey::new(15)?,
            state: ButtonState::Pressed,
            repeat: false,
            modifiers: Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
        }],
        false,
    );
    app.update();
    assert!(app.world().non_send::<HostOwner>().login.bulletin.visible);
    assert!(body(&mut app)?.contains("Published after the client release"));
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.update();
    assert!(!app.world().non_send::<HostOwner>().modal_active());
    Ok(())
}

#[test]
fn bulletin_escape_respects_frontmost_modal_and_keeps_login_open() -> TestResult {
    use bevy::input::keyboard::{Key, KeyboardInput};
    let mut app = app()?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .login
        .bulletin
        .receive(DOCUMENT)?;
    app.update();
    let blocker = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    let window = app.world_mut().spawn_empty().id();
    let escape = KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: Key::Escape,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    };
    app.world_mut().write_message(escape.clone());
    app.update();
    assert!(app.world().non_send::<HostOwner>().login.bulletin.visible);
    app.world_mut().despawn(blocker);
    app.world_mut().write_message(escape);
    app.update();
    let host = app.world().non_send::<HostOwner>();
    assert!(!host.login.bulletin.visible);
    assert!(host.login.visible);
    Ok(())
}
#[test]
fn login_sign_out_device_turns_remember_device_off() -> TestResult {
    let mut app = app()?;
    {
        let mut host = app.world_mut().non_send_mut::<HostOwner>();
        let misc = &mut host.settings.preferences.miscellaneous;
        misc.stay_logged_in = true;
        misc.remember_device = true;
        misc.quick_login = true;
    }
    let button = app
        .world_mut()
        .query::<(Entity, &LoginControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == "login.forget_device").then_some(entity))
        .ok_or("missing login sign-out button")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    let host = app.world().non_send::<HostOwner>();
    let misc = host.settings.preferences.miscellaneous;
    assert!(!misc.remember_device);
    assert!(
        misc.stay_logged_in && misc.quick_login,
        "only the Win32 choice changes"
    );
    // The fixture has no connection settings, so the Platform part cannot run yet.
    assert!(
        host.login
            .form
            .message()
            .contains("Correct the connection settings")
    );
    assert!(
        host.login.visible,
        "signing out keeps the login window open"
    );
    Ok(())
}
#[test]
fn login_about_shows_release_build_and_license() -> TestResult {
    let mut app = app()?;
    let button = app
        .world_mut()
        .query::<(Entity, &LoginControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == "login.about").then_some(entity))
        .ok_or("missing login About button")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    let host = app.world().non_send::<HostOwner>();
    let (title, message) = host.login.form.alert().ok_or("missing About alert")?;
    assert_eq!(title, "About");
    assert!(message.starts_with(&format!(
        "Oteryn Game Client\nRelease: {}\nBuild: {}\n",
        crate::RELEASE_ID,
        crate::CLIENT_BUILD,
    )));
    assert!(message.contains("License: MPL-2.0") && !message.contains('\r'));
    Ok(())
}
#[test]
fn login_support_link_needs_connection_settings_before_opening_a_browser() -> TestResult {
    let mut app = app()?;
    let button = app
        .world_mut()
        .query::<(Entity, &LoginControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == "login.support").then_some(entity))
        .ok_or("missing login Support button")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    let host = app.world().non_send::<HostOwner>();
    assert!(
        host.login
            .form
            .message()
            .contains("Correct the connection settings to open Platform pages")
    );
    Ok(())
}

#[test]
fn login_news_projects_today_and_tomorrow_without_moving_the_calendar_cursor() -> TestResult {
    let mut app = app()?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .login
        .bulletin
        .receive(DOCUMENT)?;
    app.update();
    let news = body(&mut app)?;
    assert!(news.contains("Today") && news.contains("Tomorrow"));
    activate_action(&mut app, Action::Next)?;
    activate_action(&mut app, Action::News)?;
    assert_eq!(body(&mut app)?, news);
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let state = &mut owner.login.bulletin;
        state.news_day = Some(0);
        state.body = "previous UTC day".into();
    }
    app.update();
    assert_eq!(body(&mut app)?, news);
    Ok(())
}

#[test]
fn login_shows_a_gameplay_hint_with_its_matching_control_icon() -> TestResult {
    use crate::bevy_shell::game_ui::art::ArtSlot;
    let mut app = app()?;
    let hint = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(id, name)| (name.as_str() == "login.featured_hint").then_some(id))
        .ok_or("featured login hint")?;
    let children = app.world().get::<Children>(hint).ok_or("hint contents")?;
    let text = children
        .iter()
        .find_map(|id| app.world().get::<Text>(id))
        .ok_or("hint description")?;
    let slot = children
        .iter()
        .find_map(|id| app.world().get::<ArtSlot>(id))
        .ok_or("hint illustration")?;
    assert!(
        matches!((slot.key.as_str(), text.0.as_str()), ("control-battle", t) if t.contains("Battle"))
            || matches!((slot.key.as_str(), text.0.as_str()), ("control-spellbook", t) if t.contains("Spellbook"))
    );
    let parent = app
        .world()
        .get::<ChildOf>(hint)
        .ok_or("login parent")?
        .parent();
    assert!(app.world().get::<LoginDialog>(parent).is_some());
    Ok(())
}

#[test]
fn review_login_menu_handle_hides_and_restores_the_account_actions() -> TestResult {
    let mut app = app()?;
    let find = |app: &mut App, key: &str| -> Result<Entity, Box<dyn std::error::Error>> {
        app.world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find_map(|(id, name)| (name.as_str() == key).then_some(id))
            .ok_or_else(|| key.to_owned().into())
    };
    let bar = find(&mut app, "login.top_bar")?;
    let toggle = find(&mut app, "login.toggle_top_bar")?;
    for expected in [Display::None, Display::Flex] {
        app.world_mut().trigger(Activate { entity: toggle });
        app.update();
        assert_eq!(app.world().get::<Node>(bar).ok_or("bar")?.display, expected);
    }
    Ok(())
}
