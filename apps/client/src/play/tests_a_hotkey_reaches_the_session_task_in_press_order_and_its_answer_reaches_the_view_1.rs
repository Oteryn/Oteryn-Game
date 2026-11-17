#[test]
fn a_hotkey_reaches_the_session_task_in_press_order_and_its_answer_reaches_the_view()
-> Result<(), String> {
    let serves = Arc::new(AtomicUsize::new(0));
    let combats = Arc::new(Mutex::new(Vec::new()));
    let (link, commands, events) = play_channel();
    let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
    let fake = Fake {
        serves,
        fail_step: false,
        fail_serve_after: usize::MAX,
        queued: Vec::new(),
        drained: Arc::new(AtomicUsize::new(0)),
        combats: Arc::clone(&combats),
        logouts: Arc::default(),
    };
    runtime
        .spawn(run_session(fake, commands, events))
        .map_err(|error| error.to_string())?;
    let mut view = view().map_err(|error| format!("{error:?}"))?;
    let spell = NonZeroU32::new(2).ok_or("spell")?;
    let first = NonZeroU32::new(1).ok_or("spell")?;
    view.cast(spell);
    view.cast(first);
    // Past the bar: ignored.
    view.cast(NonZeroU32::new(9).ok_or("spell")?);
    assert_eq!(view.tick(&link), Ok(()));
    for _ in 0..200 {
        assert_eq!(view.tick(&link), Ok(()));
        if lock_combats(&combats).len() == 2 && view.combat_status().ends_with("Not enough mana") {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        *lock_combats(&combats),
        [CombatCommand::Cast(spell), CombatCommand::Cast(first)]
    );
    assert_eq!(
        view.combat_status(),
        "spells 1-4 Spell cast".replace("Spell cast", "Not enough mana")
    );
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}
