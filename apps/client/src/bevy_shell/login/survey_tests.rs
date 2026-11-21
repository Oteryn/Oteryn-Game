use super::*;
use crate::bevy_shell::{login, ui};
use bevy::asset::AssetPlugin;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "bevy-survey-{}-{}",
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
fn app(temp: &Temp) -> Result<App, Box<dyn std::error::Error>> {
    let mut host = HostOwner::new(None)?;
    host.login.survey.directory = Some(temp.0.clone());
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
    super::super::super::game_ui::modal::install(&mut app);
    app.update();
    Ok(app)
}
fn open(app: &mut App) -> TestResult {
    let survey = app
        .world_mut()
        .query::<(Entity, &LoginControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == "login.survey").then_some(entity))
        .ok_or("missing login Survey button")?;
    app.world_mut().trigger(Activate { entity: survey });
    app.update();
    Ok(())
}
fn activate_answer(app: &mut App, action: Action) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, candidate)| (*candidate == action).then_some(entity))
        .ok_or("missing survey answer")?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn answer(app: &mut App, action: Action) -> TestResult {
    activate_answer(app, action)?;
    app.update();
    Ok(())
}
fn wait_for_save(app: &mut App) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app
        .world()
        .non_send::<HostOwner>()
        .login
        .survey
        .save
        .is_some()
    {
        if Instant::now() >= deadline {
            return Err("survey save did not finish".into());
        }
        app.update();
        std::thread::yield_now();
    }
    Ok(())
}
fn body(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    Ok(app
        .world_mut()
        .query_filtered::<&Text, With<Body>>()
        .single(app.world())?
        .0
        .clone())
}
fn shown(app: &mut App) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(app
        .world_mut()
        .query_filtered::<&Node, With<Dialog>>()
        .single(app.world())?
        .display
        == Display::Flex)
}

#[test]
fn login_survey_reviews_the_answer_in_client_and_queues_it_after_consent() -> TestResult {
    let temp = Temp::new()?;
    let mut app = app(&temp)?;
    open(&mut app)?;
    assert!(shown(&mut app)?);
    assert!(body(&mut app)?.contains("optional one-question"));
    assert!(app.world().non_send::<HostOwner>().modal_active());
    answer(&mut app, Action::Yes)?;
    assert!(body(&mut app)?.contains("Was browser sign-in easy?"));
    answer(&mut app, Action::No)?;
    let review = body(&mut app)?;
    assert!(review.contains("Answer: Difficult"));
    assert!(review.contains("Send exactly this response"));
    let focus = app
        .world()
        .resource::<InputFocus>()
        .get()
        .ok_or("survey focus")?;
    assert!(app.world().get::<Action>(focus) == Some(&Action::No));
    let path = temp.0.join("approved-survey.txt");
    assert!(!path.exists());
    answer(&mut app, Action::Yes)?;
    wait_for_save(&mut app)?;
    assert!(!shown(&mut app)?);
    let queued = std::fs::read_to_string(&path)?;
    assert!(review.starts_with(queued.as_str()));
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .login
            .form
            .alert()
            .is_some_and(
                |(title, text)| title == "Survey" && text.contains("Survey response queued")
            )
    );
    Ok(())
}

#[test]
fn login_survey_declined_or_refused_queues_nothing() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.0.join("approved-survey.txt");
    let mut app = app(&temp)?;
    open(&mut app)?;
    answer(&mut app, Action::No)?;
    assert!(!shown(&mut app)?);
    open(&mut app)?;
    answer(&mut app, Action::Yes)?;
    answer(&mut app, Action::Yes)?;
    assert!(body(&mut app)?.contains("Answer: Easy"));
    answer(&mut app, Action::No)?;
    assert!(!shown(&mut app)?);
    assert!(!path.exists());
    std::fs::create_dir(&path)?;
    open(&mut app)?;
    answer(&mut app, Action::Yes)?;
    answer(&mut app, Action::Yes)?;
    answer(&mut app, Action::Yes)?;
    wait_for_save(&mut app)?;
    assert!(path.is_dir());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .login
            .form
            .alert()
            .is_some_and(
                |(title, text)| title == "Survey" && text.contains("Cannot save the response")
            )
    );
    Ok(())
}

#[test]
fn login_survey_same_frame_activations_cannot_skip_consent() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.0.join("approved-survey.txt");
    let mut app = app(&temp)?;
    let writes = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&writes);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .login
        .survey
        .writer = Some(Arc::new(move |path, response| {
        count.fetch_add(1, Ordering::Relaxed);
        about::queue_survey_response(path, response)
    }));
    open(&mut app)?;
    answer(&mut app, Action::Yes)?;
    activate_answer(&mut app, Action::Yes)?;
    activate_answer(&mut app, Action::Yes)?;
    app.update();
    assert!(
        !path.exists(),
        "the response was queued before consent was displayed"
    );
    let review = body(&mut app)?;
    assert!(shown(&mut app)?);
    assert!(review.contains("Send exactly this response"));
    activate_answer(&mut app, Action::Yes)?;
    activate_answer(&mut app, Action::Yes)?;
    wait_for_save(&mut app)?;
    assert_eq!(writes.load(Ordering::Relaxed), 1);
    assert!(review.starts_with(&std::fs::read_to_string(path)?));
    assert_eq!(std::fs::read_dir(&temp.0)?.count(), 1);
    Ok(())
}

fn gated_save(reopen: bool) -> TestResult {
    let (started, writing) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let (activated, responsive) = mpsc::channel();
    let driver = std::thread::spawn(move || -> Result<(), String> {
        let run = || -> TestResult {
            let temp = Temp::new()?;
            let path = temp.0.join("approved-survey.txt");
            let mut app = app(&temp)?;
            let gate = Mutex::new(gate);
            app.world_mut()
                .non_send_mut::<HostOwner>()
                .login
                .survey
                .writer = Some(Arc::new(move |path, response| {
                started.send(()).map_err(std::io::Error::other)?;
                gate.lock()
                    .map_err(|_| std::io::Error::other("survey test gate poisoned"))?
                    .recv()
                    .map_err(std::io::Error::other)?;
                about::queue_survey_response(path, response)
            }));
            open(&mut app)?;
            answer(&mut app, Action::Yes)?;
            answer(&mut app, Action::Yes)?;
            activate_answer(&mut app, Action::Yes)?;
            app.update();
            assert!(!path.exists());
            assert!(
                app.world()
                    .non_send::<HostOwner>()
                    .login
                    .form
                    .alert()
                    .is_none()
            );
            assert!(shown(&mut app)?);
            if reopen {
                use bevy::input::keyboard::{Key, KeyboardInput};
                let window = app.world_mut().spawn_empty().id();
                app.world_mut().write_message(KeyboardInput {
                    key_code: KeyCode::Escape,
                    logical_key: Key::Escape,
                    state: bevy::input::ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                });
                app.update();
                assert!(!shown(&mut app)?);
                open(&mut app)?;
            }
            activated.send(())?;
            wait_for_save(&mut app)?;
            assert!(path.exists());
            let alert = app.world().non_send::<HostOwner>().login.form.alert();
            if reopen {
                assert!(alert.is_none());
                assert!(body(&mut app)?.contains("optional one-question"));
            } else {
                assert!(
                    alert.is_some_and(|(_, message)| message.contains("Survey response queued"))
                );
                assert!(!shown(&mut app)?);
            }
            Ok(())
        };
        run().map_err(|error| error.to_string())
    });
    let deadline = Duration::from_secs(5);
    let returned = writing
        .recv_timeout(deadline)
        .and_then(|()| responsive.recv_timeout(deadline));
    let _ = release.send(());
    let result = driver.join().map_err(|_| "survey fixture panicked");
    assert!(
        returned.is_ok(),
        "Activate or the frame waited for the blocked survey writer"
    );
    result??;
    Ok(())
}

#[test]
fn login_survey_blocked_writer_does_not_block_activation() -> TestResult {
    gated_save(false)
}

#[test]
fn login_survey_ignores_save_completion_after_close_and_reopen() -> TestResult {
    gated_save(true)
}
