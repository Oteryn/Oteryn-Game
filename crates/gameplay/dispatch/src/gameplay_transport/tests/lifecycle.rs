/// PREM-1b (D476): every terminal release, abandoned or mismatched, stops the account's
/// Premium pulls on a proven `Released` or `Terminal` outcome, as grace expiry does, and
/// never on an unended or unknown one.
#[test]
fn only_a_proven_terminal_release_stops_premium() {
    assert!(ends_session::<()>(&Ok(ExpiredLossReleaseV1::Released {
        decided_at: 1
    })));
    assert!(ends_session::<()>(&Ok(ExpiredLossReleaseV1::Terminal)));
    assert!(!ends_session::<()>(&Ok(
        ExpiredLossReleaseV1::NotApplicable
    )));
    assert!(!ends_session::<()>(&Ok(ExpiredLossReleaseV1::NotExpired {
        deadline: 2,
        now: 1
    })));
    assert!(!ends_session(&Err::<ExpiredLossReleaseV1, ()>(())));
}

/// PREM-1b (D476, #1722 Codex P1 4176449967): a late or repeated terminal release of a
/// session never stops the pulls of a same-account successor admitted before it lands.
#[test]
fn a_late_release_keeps_a_successor_session_premium() {
    let account = [9; 16];
    let old = GameSessionId::decode(&uuid_v7(0x70)).expect("old");
    let successor = GameSessionId::decode(&uuid_v7(0x71)).expect("successor");
    let mut sessions = PremiumSessions::default();
    sessions.admit(account, old);
    // A resume of the same session registers once.
    sessions.admit(account, old);
    sessions.admit(account, successor);
    // The old session's Terminal outcome lands after the successor's admission.
    assert!(!sessions.release(account, old));
    // A retry of that release, and a release never admitted, stop nothing.
    assert!(!sessions.release(account, old));
    assert!(!sessions.release([8; 16], old));
    // The successor's own release is the account's last: its pulls stop.
    assert!(sessions.release(account, successor));
    assert!(!sessions.release(account, successor));
    // Grace expiry or a mismatch release of a sole session stops them at once.
    sessions.admit(account, old);
    assert!(sessions.release(account, old));
}

/// PREM-1b (D476, #1722 Codex P2 4176492358): a reconciled TERMINAL settle releases the
/// session's registration like a proven release. When a joined release of the same session
/// already released it, the second release is a no-op and a successor keeps its pulls.
#[test]
fn a_reconciled_terminal_release_stops_premium_once() {
    let account = [7; 16];
    let lost = GameSessionId::decode(&uuid_v7(0x72)).expect("lost");
    let successor = GameSessionId::decode(&uuid_v7(0x73)).expect("successor");
    let mut sessions = PremiumSessions::default();
    sessions.admit(account, lost);
    // The abandoned release's attempt is unended; its reconciliation finds TERMINAL.
    assert!(sessions.release(account, lost));
    // Grace expiry of the same session, joined on its fence, reconciles TERMINAL too.
    assert!(!sessions.release(account, lost));
    // A successor admitted afterwards is untouched by any later reconciliation.
    sessions.admit(account, successor);
    assert!(!sessions.release(account, lost));
    assert!(sessions.release(account, successor));
}

/// D449 (#1708 Codex P1 4175882774): the capability-mismatch release fences with the lost
/// epoch. Its retry and grace expiry join that fence, a compatible resume that wins the race
/// lifts it when it restores control, and no mismatch attempt can fence the slot again
/// afterwards, so an unproven mismatch release never leaves a fence behind a resumed session.
/// A slot whose bound session lost control at epoch 1.
fn lost_slot() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
    use {
        crate::foundation::{CharacterLease, NodeId},
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    };
    let world_id = WorldId::decode(&uuid_v7(0x60)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world_id,
        ChannelId::decode(&uuid_v7(0x61)).expect("channel"),
        NodeId::decode(&uuid_v7(0x62)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        ChannelContentPin::test(world_id),
    )
    .expect("channel runtime");
    let session = GameSessionId::decode(&uuid_v7(0x63)).expect("session");
    let reservation = runtime.reserve_fresh_session(session).expect("reserve");
    let actor = runtime.commit_fresh_session(reservation).expect("commit");
    let character = CharacterId::decode(&CHARACTER).expect("character");
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(character, 1).expect("lease"),
        )
        .expect("bind lease");
    runtime
        .record_control_loss(
            actor,
            session,
            ControlLossMark {
                epoch: 1,
                grace_deadline: 160,
            },
        )
        .expect("loss");
    (runtime, actor, session)
}

#[test]
fn capability_mismatch_fence_is_lifted_by_a_winning_resume() {
    let (mut runtime, actor, session) = lost_slot();
    let epoch = ControlLossEpochRefV1::new(1).expect("epoch");
    let mismatch = TerminalRelease::CapabilityMismatch(epoch)
        .fence(&mut runtime)
        .expect("mismatch token");
    mismatch
        .fence(&mut runtime, actor, session)
        .expect("mismatch fences");
    // A retry of the mismatch release and grace expiry of the same epoch join the fence.
    let retry = TerminalRelease::CapabilityMismatch(epoch)
        .fence(&mut runtime)
        .expect("retry token");
    assert_eq!(retry.fence(&mut runtime, actor, session), Ok(()));
    assert_eq!(
        TransitionFence::GraceExpiry(1).fence(&mut runtime, actor, session),
        Ok(())
    );
    // Any other transition waits on it.
    let other = TerminalRelease::Abandoned(
        AuthenticatedTransportRefV1::decode(&[7; 16]).expect("transport"),
    )
    .fence(&mut runtime)
    .expect("other token");
    assert_eq!(
        other.fence(&mut runtime, actor, session),
        Err(CarrierError::WriteFenceBusy)
    );
    // The compatible resume wins: restoring control of the epoch lifts the mismatch fence,
    // with no durable read and no lost entry needed.
    assert_eq!(runtime.restore_control(actor, session, 1), Ok(()));
    assert_eq!(mismatch.lift(&mut runtime, actor, session), Ok(false));
    assert_eq!(other.fence(&mut runtime, actor, session), Ok(()));
    assert_eq!(other.lift(&mut runtime, actor, session), Ok(true));
    // A late mismatch attempt can no longer fence the resumed slot.
    assert_eq!(
        retry.fence(&mut runtime, actor, session),
        Err(CarrierError::ControlLossConflict)
    );
    assert_eq!(other.fence(&mut runtime, actor, session), Ok(()));
}

/// D449 (#1708 Codex P1 4175947583): grace expiry and a mismatch release of one epoch join
/// its `ControlLoss` fence. A release that settles first gives back only its own hold, so the
/// fence stays while the other is still saving, and the last holder lifts it. A retry is
/// counted once, a refused release leaves the fence to the others, and a release holding
/// nothing lifts nothing.
#[test]
fn a_joined_epoch_fence_is_lifted_only_by_its_last_holder() {
    let (mut runtime, actor, session) = lost_slot();
    let holders = FenceHolders::default();
    let token = TransitionFence::GraceExpiry(1);
    let other = TransitionFence::Transition(9);
    let mut grace = FenceHold::new(session, token);
    let mut mismatch = FenceHold::new(session, token);
    assert_eq!(holders.fence(&mut runtime, actor, &mut grace), Ok(()));
    assert_eq!(holders.fence(&mut runtime, actor, &mut mismatch), Ok(()));
    // The mismatch release retries its fence: still one hold.
    assert_eq!(holders.fence(&mut runtime, actor, &mut mismatch), Ok(()));
    // The mismatch release settles NotApplicable while grace expiry is still saving.
    assert_eq!(holders.lift(&mut runtime, actor, &mut mismatch), Ok(()));
    assert_eq!(
        other.fence(&mut runtime, actor, session),
        Err(CarrierError::WriteFenceBusy)
    );
    // A second release holding nothing lifts nothing.
    assert_eq!(holders.lift(&mut runtime, actor, &mut mismatch), Ok(()));
    assert_eq!(
        other.fence(&mut runtime, actor, session),
        Err(CarrierError::WriteFenceBusy)
    );
    // A mismatch retry joins and is refused later: the fence stays with grace expiry.
    let mut retry = FenceHold::new(session, token);
    assert_eq!(holders.fence(&mut runtime, actor, &mut retry), Ok(()));
    holders.release(&mut retry);
    assert_eq!(
        other.fence(&mut runtime, actor, session),
        Err(CarrierError::WriteFenceBusy)
    );
    // Grace expiry, the last holder, settles: the fence is lifted and no count is left.
    assert_eq!(holders.lift(&mut runtime, actor, &mut grace), Ok(()));
    assert!(holders.counts().is_empty());
    assert_eq!(other.fence(&mut runtime, actor, session), Ok(()));
    assert_eq!(other.lift(&mut runtime, actor, session), Ok(true));
    // A retired session leaves no hold behind.
    assert_eq!(holders.fence(&mut runtime, actor, &mut grace), Ok(()));
    holders.forget(session);
    assert!(holders.counts().is_empty());
}

/// KILL-REWARD-COMP-1 (#1908 Codex P1 4202367021): a grace expiry that ends unknown keeps
/// its hold for the lifecycle's retry, which counts the release once, so the retry's
/// non-terminal settle lifts the fence and leaves no count.
#[test]
fn a_retried_unknown_grace_expiry_counts_its_hold_once() {
    let (mut runtime, actor, session) = lost_slot();
    let holders = FenceHolders::default();
    let token = TransitionFence::GraceExpiry(1);
    let other = TransitionFence::Transition(9);
    let mut first = FenceHold::new(session, token);
    assert_eq!(holders.fence(&mut runtime, actor, &mut first), Ok(()));
    // The expiry ends unknown holding the fence; the retry takes the hold over.
    holders.keep(first);
    let mut retry = holders.resume(session).expect("kept hold");
    assert!(holders.resume(session).is_none());
    assert_eq!(holders.fence(&mut runtime, actor, &mut retry), Ok(()));
    assert_eq!(holders.counts().get(&(session, token)), Some(&1));
    // The retry's non-terminal settle lifts the fence and no count is left.
    assert_eq!(holders.lift(&mut runtime, actor, &mut retry), Ok(()));
    assert!(holders.counts().is_empty());
    assert_eq!(other.fence(&mut runtime, actor, session), Ok(()));
    assert_eq!(other.lift(&mut runtime, actor, session), Ok(true));
    // A hold that ended is not kept, and a retired session leaves none behind.
    holders.keep(retry);
    assert!(holders.resume(session).is_none());
    let mut kept = FenceHold::new(session, token);
    assert_eq!(holders.fence(&mut runtime, actor, &mut kept), Ok(()));
    holders.keep(kept);
    holders.forget(session);
    assert!(holders.resume(session).is_none());
}

/// KILL-REWARD-COMP-1 (#1908 Codex P1 4202825532): two grace-expiry lifecycles of one
/// session that both end unknown keep both holds; their retries settle each once, so the
/// fence is lifted and no count is left.
#[test]
fn concurrent_unknown_grace_expiries_keep_every_hold() {
    let (mut runtime, actor, session) = lost_slot();
    let holders = FenceHolders::default();
    let token = TransitionFence::GraceExpiry(1);
    let other = TransitionFence::Transition(9);
    let mut first = FenceHold::new(session, token);
    let mut second = FenceHold::new(session, token);
    assert_eq!(holders.fence(&mut runtime, actor, &mut first), Ok(()));
    assert_eq!(holders.fence(&mut runtime, actor, &mut second), Ok(()));
    // Both end unknown: neither kept hold replaces the other.
    holders.keep(first);
    holders.keep(second);
    let mut retries = [
        holders.resume(session).expect("kept hold"),
        holders.resume(session).expect("kept hold"),
    ];
    assert!(holders.resume(session).is_none());
    assert_eq!(holders.counts().get(&(session, token)), Some(&2));
    // Each retry joins the fence without counting again, and settles its own hold once.
    for retry in &mut retries {
        assert_eq!(holders.fence(&mut runtime, actor, retry), Ok(()));
    }
    assert_eq!(holders.counts().get(&(session, token)), Some(&2));
    let [early, last] = &mut retries;
    assert_eq!(holders.lift(&mut runtime, actor, early), Ok(()));
    assert_eq!(
        other.fence(&mut runtime, actor, session),
        Err(CarrierError::WriteFenceBusy)
    );
    assert_eq!(holders.lift(&mut runtime, actor, last), Ok(()));
    assert!(holders.counts().is_empty());
    assert_eq!(other.fence(&mut runtime, actor, session), Ok(()));
    assert_eq!(other.lift(&mut runtime, actor, session), Ok(true));
}
