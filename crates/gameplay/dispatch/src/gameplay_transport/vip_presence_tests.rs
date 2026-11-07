use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn id(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag,
    ]
}
fn session() -> Result<GameSessionId, Box<dyn std::error::Error>> {
    GameSessionId::decode(&id(1)).map_err(|error| format!("{error:?}").into())
}
fn row(status: Status) -> Result<VipPresenceRow, Box<dyn std::error::Error>> {
    Ok(VipPresenceRow {
        target: CharacterId::decode(&id(2)).map_err(|error| format!("{error:?}"))?,
        entry: 1,
        name: "Alice".into(),
        notify: true,
        visible: status != Status::Unconfirmed,
        status,
    })
}
fn read(
    runtime: &VipPresenceRuntime,
    at: Instant,
    status: Status,
) -> Result<PresenceBatch, Box<dyn std::error::Error>> {
    let Some(Refresh::Read(plan)) = runtime.begin(session()?, 1, true, at) else {
        return Err("expected presence read".into());
    };
    runtime
        .complete(session()?, plan, Some(vec![row(status)?]), at)
        .ok_or_else(|| "expected current snapshot".into())
}

#[test]
fn vip_presence_hint_requires_truth_and_real_transition_before_announcement() -> TestResult {
    let runtime = VipPresenceRuntime::default();
    let now = Instant::now();
    assert!(!read(&runtime, now, Status::Offline)?.entries[0].announce);
    runtime.hint(row(Status::Online)?.target, true);
    let online = read(&runtime, now, Status::Online)?;
    assert!(online.entries[0].announce);
    assert_eq!(online.entries[0].confirmed_for_ms, 10_000);
    runtime.hint(row(Status::Online)?.target, true);
    // Duplicate/replayed login hint produces no duplicate notice.
    assert!(!read(&runtime, now, Status::Online)?.entries[0].announce);
    runtime.hint(row(Status::Online)?.target, true);
    // A delayed login after an actual logout corrects truth without a stale login notice.
    let offline = read(&runtime, now, Status::Offline)?;
    assert!(!offline.entries[0].announce);
    runtime.hint(row(Status::Online)?.target, true);
    read(&runtime, now, Status::Online)?;
    runtime.hint(row(Status::Offline)?.target, false);
    assert!(read(&runtime, now, Status::Offline)?.entries[0].announce);
    Ok(())
}

#[test]
fn vip_presence_periodic_reconcile_privacy_and_listener_loss_do_not_announce() -> TestResult {
    let runtime = VipPresenceRuntime::default();
    let now = Instant::now();
    assert!(!read(&runtime, now, Status::Online)?.entries[0].announce);
    assert!(
        runtime
            .begin(
                session()?,
                1,
                true,
                now + CONFIRM - Duration::from_millis(1)
            )
            .is_none()
    );
    let hidden = read(&runtime, now + CONFIRM, Status::Offline)?;
    assert_eq!(hidden.entries[0].confirmed_for_ms, 0);
    assert!(!hidden.entries[0].announce);
    runtime.invalidate();
    read(&runtime, now + CONFIRM, Status::Online)?;
    let Some(Refresh::Send(lost)) = runtime.begin(session()?, 1, false, now + CONFIRM) else {
        return Err("listener loss must invalidate online".into());
    };
    assert_eq!(lost.entries[0].status, Status::Unconfirmed);
    assert_eq!(lost.entries[0].confirmed_for_ms, 0);
    assert!(!lost.entries[0].announce);
    runtime.hint(row(Status::Online)?.target, true);
    assert!(!read(&runtime, now + CONFIRM, Status::Online)?.entries[0].announce);
    runtime.invalidate();
    let revoked = read(&runtime, now + CONFIRM, Status::Unconfirmed)?;
    assert!(!revoked.entries[0].announce);
    Ok(())
}

#[test]
fn vip_presence_late_read_cannot_replace_new_generation_or_detached_session() -> TestResult {
    let runtime = VipPresenceRuntime::default();
    let now = Instant::now();
    let Some(Refresh::Read(old)) = runtime.begin(session()?, 1, true, now) else {
        return Err("old plan".into());
    };
    let Some(Refresh::Read(new)) = runtime.begin(session()?, 2, true, now) else {
        return Err("new plan".into());
    };
    assert!(
        runtime
            .complete(session()?, old, Some(vec![row(Status::Online)?]), now)
            .is_none()
    );
    assert!(runtime.begin(session()?, 1, true, now).is_none());
    runtime.forget(session()?);
    assert!(
        runtime
            .complete(session()?, new, Some(vec![row(Status::Online)?]), now)
            .is_none()
    );
    Ok(())
}

#[test]
fn vip_presence_failed_read_unconfirms_and_unwatched_hints_allocate_nothing() -> TestResult {
    let runtime = VipPresenceRuntime::default();
    let now = Instant::now();
    read(&runtime, now, Status::Online)?;
    let other = CharacterId::decode(&id(3)).map_err(|error| format!("{error:?}"))?;
    for _ in 0..1000 {
        runtime.hint(other, true);
    }
    assert!(runtime.begin(session()?, 1, true, now).is_none());
    runtime.invalidate();
    let Some(Refresh::Read(plan)) = runtime.begin(session()?, 1, true, now) else {
        return Err("failed read plan".into());
    };
    let failed = runtime
        .complete(session()?, plan, None, now)
        .ok_or("failure snapshot")?;
    assert_eq!(failed.entries[0].status, Status::Unconfirmed);
    assert!(!failed.entries[0].announce);
    Ok(())
}
