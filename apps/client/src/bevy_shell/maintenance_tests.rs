use super::*;
use std::sync::mpsc::Sender;
type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Every operation a test window started, in order; tests never reach the network.
static STARTED: Mutex<Vec<(PathBuf, String, Action)>> = Mutex::new(Vec::new());

/// A deterministic operation: verification reports a mismatch, anything else succeeds.
fn scripted(directory: PathBuf, channel: &str, action: Action) -> Receiver<Outcome> {
    let (tx, rx) = mpsc::channel();
    let result = match action {
        Action::Verify => Err(UpdateError::Verification),
        _ => Ok(format!("{action:?} finished")),
    };
    let _ = tx.send(result);
    if let Ok(mut started) = STARTED.lock() {
        started.push((directory, channel.to_owned(), action));
    }
    rx
}

fn app(directory: PathBuf) -> (App, Sender<Outcome>, Arc<Mutex<Option<Outcome>>>) {
    let (tx, rx) = mpsc::channel();
    let outcome = Arc::new(Mutex::new(None));
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>();
    install(
        &mut app,
        Maintenance {
            directory,
            channel: "beta".into(),
            stage: Stage::Working,
            pending: Some((Action::Check, rx)),
            report: "Checking the signed beta release…".into(),
            outcome: outcome.clone(),
            start: scripted,
        },
    );
    app.update();
    (app, tx, outcome)
}
fn report(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    Ok(app
        .world_mut()
        .query_filtered::<&Text, With<Report>>()
        .single(app.world())?
        .0
        .clone())
}
fn shown(app: &mut App) -> Vec<Choice> {
    app.world_mut()
        .query::<(&Choice, &Node)>()
        .iter(app.world())
        .filter(|(_, node)| node.display != Display::None)
        .map(|(choice, _)| *choice)
        .collect()
}
fn click(app: &mut App, choice: Choice) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Choice)>()
        .iter(app.world())
        .find(|(_, candidate)| **candidate == choice)
        .map(|(entity, _)| entity)
        .ok_or("maintenance choice missing")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}

#[test]
fn maintenance_window_checks_then_verifies_offers_repair_and_closes() -> TestResult {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let directory = fixture.path().to_path_buf();
    let (mut app, check, outcome) = app(directory.clone());
    assert!(
        shown(&mut app).is_empty(),
        "nothing is offered while checking"
    );
    app.world_mut().write_message(WindowCloseRequested {
        window: Entity::PLACEHOLDER,
    });
    app.update();
    assert_eq!(
        app.should_exit(),
        None,
        "a running check keeps the window open"
    );

    check.send(Ok("Version: 2.0\nRelease: r2".into()))?;
    app.update();
    assert!(report(&mut app)?.starts_with("Version: 2.0"));
    assert_eq!(
        shown(&mut app),
        vec![Choice::Update, Choice::Verify, Choice::Close]
    );
    click(&mut app, Choice::Repair)?;
    assert!(
        report(&mut app)?.starts_with("Version: 2.0"),
        "a hidden choice does nothing"
    );

    // The Verify button starts the injected operation, never the signed-release transport.
    click(&mut app, Choice::Verify)?;
    assert_eq!(
        STARTED.lock().map_err(|_| "started")?.as_slice(),
        [(directory.clone(), "beta".into(), Action::Verify)]
    );
    app.update();
    assert!(report(&mut app)?.starts_with("Verification failed"));
    assert_eq!(shown(&mut app), vec![Choice::Repair, Choice::Close]);
    assert!(matches!(
        outcome.lock().map_err(|_| "outcome")?.as_ref(),
        Some(Err(_))
    ));

    click(&mut app, Choice::Close)?;
    assert_eq!(app.should_exit(), Some(AppExit::Success));
    Ok(())
}
