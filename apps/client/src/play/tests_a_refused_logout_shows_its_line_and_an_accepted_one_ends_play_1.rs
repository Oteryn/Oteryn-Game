#[test]
fn a_refused_logout_shows_its_line_and_an_accepted_one_ends_play() -> Result<(), String> {
    let logouts = Arc::new(Mutex::new(VecDeque::from([
        Some(LogoutResult::InFight {
            retry_after_ms: 2_001,
        }),
        None,
        Some(LogoutResult::Accepted),
    ])));
    let (link, commands, events) = play_channel();
    let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
    let fake = Fake {
        serves: Arc::new(AtomicUsize::new(0)),
        fail_step: false,
        fail_serve_after: usize::MAX,
        queued: Vec::new(),
        drained: Arc::new(AtomicUsize::new(0)),
        combats: Arc::new(Mutex::new(Vec::new())),
        logouts: Arc::clone(&logouts),
    };
    runtime
        .spawn(run_session(fake, commands, events))
        .map_err(|error| error.to_string())?;
    let mut view = view().map_err(|error| format!("{error:?}"))?;
    let tick_until = |view: &mut PlayView, done: &dyn Fn(&PlayView) -> bool| {
        for _ in 0..200 {
            let ticked = view.tick(&link);
            if ticked.is_err() || done(view) {
                return ticked;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    };
    view.logout();
    // A second press before the tick is the same logout.
    view.logout();
    assert_eq!(
        tick_until(&mut view, &|view| view.combat_status().contains("fight")),
        Ok(())
    );
    assert_eq!(
        view.combat_status(),
        "spells 1-4 You may not logout during a fight (3 s)"
    );
    assert_eq!(lock_logouts(&logouts).len(), 2);
    view.logout();
    assert_eq!(
        tick_until(&mut view, &|view| view.combat_status().contains("again")),
        Ok(())
    );
    view.logout();
    assert_eq!(tick_until(&mut view, &|_| false), Err(PlayEnd::LoggedOut));
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}
