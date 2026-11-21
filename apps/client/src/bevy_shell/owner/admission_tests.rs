use super::*;
use crate::{
    account_services::test_server::{TestResult, test_thread},
    bevy_shell::{game_ui::modal, login, ui},
    native_entry::test_support::Stack,
};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_session::admission_queue::{AdmissionQueue, encode_queue_refusal};
use std::time::{Duration, Instant};

#[test]
fn login_form_shows_server_queue_position_and_cancel_stops_admission() -> TestResult {
    test_thread(|| {
        let refusal = encode_queue_refusal(AdmissionQueue {
            position: 3,
            retry_after_ms: 20_000,
        })?;
        let stack = Stack::start_with_frames(vec![], vec![refusal])?;
        let config = stack.config();
        let mut owner = HostOwner::new(None)?;
        owner.request_login();
        owner.login.form.settings = crate::native_login::LoginSettings {
            platform_url: config.platform_url.clone(),
            gateway_url: config.gateway_url.clone(),
            oauth_client_id: config.oauth_client_id.clone(),
            dev_root: config.dev_root.to_string_lossy().into_owned(),
            ..Default::default()
        };
        owner.login.form.submit()?;
        let entry = owner.entry.as_mut().ok_or("entry")?;
        assert!(entry.configure(config));
        assert!(entry.request());

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Font>()
            .init_resource::<InputFocus>()
            .insert_non_send(owner)
            .add_systems(Startup, (ui::setup, login::setup).chain())
            .add_systems(PreUpdate, |mut owner: NonSendMut<HostOwner>| {
                owner.poll([800, 600]);
            })
            .add_systems(Update, (login::project, ui::project).chain());
        modal::install(&mut app);

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            app.update();
            let owner = app.world().non_send::<HostOwner>();
            if owner.login.form.message().starts_with("Queue position 3.") {
                break;
            }
            if Instant::now() >= deadline {
                return Err("server queue never reached the login form".into());
            }
            std::thread::yield_now();
        }
        let assert_queue_text = |app: &mut App| {
            let world = app.world_mut();
            assert!(
                world.query::<&Text>().iter(world).any(|text| {
                    text.0.starts_with("Queue position 3.") && text.0.contains("Retrying in ")
                }),
                "queue countdown must be rendered"
            );
        };
        assert_queue_text(&mut app);

        // Reentry after a Channel switch uses the shell while the login window is hidden.
        app.world_mut().non_send_mut::<HostOwner>().login.visible = false;
        app.update();
        assert_queue_text(&mut app);
        app.world_mut().non_send_mut::<HostOwner>().login.visible = true;
        app.update();

        let world = app.world_mut();
        let cancel = world
            .query::<(Entity, &login::LoginControl)>()
            .iter(world)
            .find_map(|(entity, control)| {
                matches!(control, login::LoginControl::Cancel).then_some(entity)
            })
            .ok_or("cancel button")?;
        world.trigger(Activate { entity: cancel });
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.update();
            if app.world().non_send::<HostOwner>().phase == Phase::Offline {
                break;
            }
            if Instant::now() >= deadline {
                return Err("cancel did not end queued admission".into());
            }
            std::thread::yield_now();
        }
        assert_eq!(
            stack.admissions.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert!(app.world().non_send::<HostOwner>().play.is_none());
        stack.finish();
        Ok(())
    })
}
