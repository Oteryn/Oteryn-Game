use super::*;
use std::{
    sync::mpsc::Sender,
    time::{Duration, Instant},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

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
            stage: Stage::Working,
            pending: Some((Action::Check, rx)),
            report: "Checking the signed stable release…".into(),
            outcome: outcome.clone(),
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
    let directory = std::env::temp_dir().join(format!("oteryn-maintenance-{}", std::process::id()));
    let (mut app, check, outcome) = app(directory);
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

    // Verification runs on the real maintenance caller; this build has no signing root.
    click(&mut app, Choice::Verify)?;
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.world().non_send::<Maintenance>().stage == Stage::Working {
        if Instant::now() >= deadline {
            return Err("verification did not finish".into());
        }
        std::thread::sleep(Duration::from_millis(10));
        app.update();
    }
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
