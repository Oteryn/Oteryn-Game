use super::*;
use crate::account_services::test_server::{MockPlatform, TestResult, test_thread};
use oteryn_foundation::CancellationToken;
use std::{
    thread,
    time::{Duration, Instant},
};

fn wait(app: &mut App, after: Option<u64>) -> Result<u64, &'static str> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        app.update();
        if let Some(id) = app
            .world()
            .non_send::<HostOwner>()
            .services
            .pending()
            .map(|r| r.id)
            && Some(id) != after
        {
            return Ok(id);
        }
        thread::yield_now();
    }
    Err("account presenter did not reach Bevy")
}
fn click(app: &mut App, button: &str) -> TestResult {
    let id = app
        .world()
        .non_send::<HostOwner>()
        .services
        .pending()
        .ok_or("request")?
        .id;
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, c)| c.request == id && c.button == button)
        .map(|(e, _)| e)
        .ok_or("button")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
#[test]
fn premium_bevy_buttons_fetch_refresh_report_errors_and_return_to_account_menu() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let authority = mock.authority.clone();
        let owner = HostOwner::new(None)?;
        let mut config = crate::NativeLoginConfig::from_env(|key| match key {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some(mock.base.clone()),
            "OTERYN_OAUTH_CLIENT_ID" => Some("test-client".into()),
            "OTERYN_DEV_ROOT" => Some("/tmp/unused-premium-test-root".into()),
            _ => None,
        })?
        .ok_or("config")?;
        owner.services.configure(&mut config);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Font>()
            .init_resource::<InputFocus>()
            .add_message::<bevy::input::keyboard::KeyboardInput>()
            .insert_non_send(owner)
            .add_systems(Startup, super::super::ui::setup);
        install(&mut app);
        super::super::game_ui::modal::install(&mut app);
        let worker = thread::Builder::new().stack_size(64 << 20).spawn(move || {
            mock.runtime
                .block_on(crate::account_services::run(
                    &client,
                    &token,
                    &config,
                    CancellationToken::new(),
                ))
                .map_err(|e| format!("{e:?}"))
        })?;
        let menu = wait(&mut app, None)?;
        click(&mut app, "premium")?;
        let premium = wait(&mut app, Some(menu))?;
        let text = app
            .world_mut()
            .query_filtered::<&Text, With<Body>>()
            .single(app.world())?;
        assert!(text.0.contains("Private channels"));
        assert!(text.0.contains("Expiry:"));
        assert!(text.0.contains("[available]"));
        authority.lock().map_err(|_| "lock")?.premium_http_status = Some(503);
        click(&mut app, "refresh")?;
        let failed = wait(&mut app, Some(premium))?;
        let text = app
            .world_mut()
            .query_filtered::<&Text, With<Body>>()
            .single(app.world())?;
        assert!(text.0.contains("Premium service unavailable"));
        assert!(!text.0.contains("[available]"));
        authority.lock().map_err(|_| "lock")?.premium_http_status = None;
        click(&mut app, "refresh")?;
        let refreshed = wait(&mut app, Some(failed))?;
        // A stale button's request must not dismiss a refreshed view.
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .services
            .respond(
                premium,
                Reply {
                    button: "back",
                    selected: Vec::new(),
                    fields: Vec::new(),
                },
            );
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .services
                .pending()
                .map(|r| r.id),
            Some(refreshed)
        );
        click(&mut app, "back")?;
        wait(&mut app, Some(refreshed))?;
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
        worker.join().map_err(|_| "worker")??;
        app.update();
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .services
                .pending()
                .is_none()
        );
        let state = authority.lock().map_err(|_| "lock")?;
        assert_eq!(
            state
                .requests
                .iter()
                .filter(|r| r.starts_with("GET /api/v1/native-account/premium "))
                .count(),
            3
        );
        assert_eq!(state.effects, 0);
        Ok(())
    })
}
