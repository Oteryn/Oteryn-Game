use super::*;
use crate::bevy_shell::{login, ui};
use bevy::asset::AssetPlugin;
use std::sync::atomic::{AtomicU64, Ordering};
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
fn answer(app: &mut App, action: Action) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, candidate)| (*candidate == action).then_some(entity))
        .ok_or("missing survey answer")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
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
    let path = temp.0.join("approved-survey.txt");
    assert!(!path.exists());
    answer(&mut app, Action::Yes)?;
    assert!(!shown(&mut app)?);
    let queued = std::fs::read_to_string(&path)?;
    assert!(review.starts_with(queued.as_str()));
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .login
            .form
            .message()
            .contains("Survey response queued")
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
    std::fs::write(&path, b"earlier response")?;
    open(&mut app)?;
    answer(&mut app, Action::Yes)?;
    answer(&mut app, Action::Yes)?;
    answer(&mut app, Action::Yes)?;
    assert_eq!(std::fs::read(&path)?, b"earlier response");
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .login
            .form
            .message()
            .contains("Cannot save the response")
    );
    Ok(())
}
