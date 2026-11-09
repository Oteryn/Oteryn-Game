use super::*;

#[test]
fn a_step_error_ends_the_task_with_its_public_class() -> Result<(), String> {
    let (link, _serves, runtime) = task(true, usize::MAX)?;
    link.request(StepDir::North);
    assert!(matches!(
        wait_for(&link),
        Some(PlayEvent::Ended(PublicClass::SessionUnavailable))
    ));
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

#[test]
fn a_liveness_error_while_idle_returns_to_login() -> Result<(), String> {
    let (link, _serves, runtime) = task(false, 2)?;
    let mut view = view().map_err(|error| format!("{error:?}"))?;
    let mut ended = None;
    for _ in 0..200 {
        if let Err(end) = view.tick(&link) {
            ended = Some(end);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(ended, Some(PlayEnd::Ended(PublicClass::SessionUnavailable)));
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

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

#[test]
fn pushed_events_are_drained_so_the_queue_never_overflows() -> Result<(), String> {
    let (link, commands, events) = play_channel();
    let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
    let pushing = Pushing {
        queue: Vec::new(),
        sequence: 0,
    };
    runtime
        .spawn(run_session(pushing, commands, events))
        .map_err(|error| error.to_string())?;
    let mut view = view().map_err(|error| format!("{error:?}"))?;
    let mut pushed = 0;
    for _ in 0..400 {
        while let Some(event) = link.poll() {
            match event {
                PlayEvent::Pushed(batch) => {
                    pushed += 1;
                    view.apply_pushed(&batch)
                        .map_err(|error| format!("{error:?}"))?;
                }
                PlayEvent::Ended(class) => return Err(format!("session ended: {class:?}")),
                PlayEvent::LoggedOut => return Err("logged out".to_owned()),
                PlayEvent::Stepped(_)
                | PlayEvent::Combat(_)
                | PlayEvent::LogoutRefused(_)
                | PlayEvent::ActionFeedback(_) => {}
            }
        }
        if pushed > 4 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(pushed > 4);
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

#[test]
fn an_undrained_mailbox_stays_bounded_and_keeps_the_latest_state() {
    let (link, _commands, outbox) = play_channel();
    for n in 0..5_000u64 {
        let events = vec![
            spatial_event(n, i32::try_from(n).unwrap_or(0)),
            overlay_event(n, n.to_be_bytes().to_vec()),
            overlay_event(n, b"same".to_vec()),
        ];
        outbox.push(events);
    }
    let event = link.poll();
    assert!(matches!(event, Some(PlayEvent::Pushed(_))));
    let Some(PlayEvent::Pushed(pushed)) = event else {
        return;
    };
    assert_eq!(pushed.own.map(|position| position.x), Some(4_999));
    assert!(pushed.markers.len() <= MAX_PUSHED_MARKERS);
    assert!(link.poll().is_none());
}

#[test]
fn pushes_older_than_a_step_outcome_are_delivered_before_it() {
    let (link, _commands, outbox) = play_channel();
    outbox.push(vec![spatial_event(1, 50)]);
    outbox.stepped(moved_to(100, 199, 0));
    outbox.push(vec![spatial_event(3, 60)]);
    assert!(matches!(
        link.poll(),
        Some(PlayEvent::Pushed(pushed)) if pushed.own.is_some_and(|position| position.x == 50)
    ));
    assert!(matches!(link.poll(), Some(PlayEvent::Stepped(_))));
    assert!(matches!(
        link.poll(),
        Some(PlayEvent::Pushed(pushed)) if pushed.own.is_some_and(|position| position.x == 60)
    ));
    assert!(link.poll().is_none());
}
