#![allow(clippy::expect_used)]
use super::*;
use crate::foundation::{
    ChannelId, CharacterLease, CharacterWorldEligibilityClaimV1, FreshAdmissionCommit,
    FreshAdmissionFacts, WorldId,
};
use oteryn_protocol_oteryn::{
    chat::{ChatLine, ChatRoom},
    world_spatial::{ActorPosition, WorldSpatialObservation},
};

fn uuid(value: u8) -> [u8; 16] {
    let mut bytes = [value; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}
fn session(value: u8) -> GameSessionId {
    GameSessionId::decode(&uuid(value)).expect("session")
}
fn account(value: u8) -> AccountId {
    AccountId::from_bytes(uuid(value)).expect("account")
}
fn identity(value: u8) -> ObserverIdentity {
    ObserverIdentity {
        session: session(value),
        generation: ConnectionGeneration::new(1).expect("generation"),
        account: account(value),
    }
}
fn runtime() -> SpectateRuntime {
    SpectateRuntime::new(Limits {
        node_sessions: 128,
        target_grants: 16,
        attempt_accounts: 128,
    })
    .expect("limits")
}
/// Native snapshot fixture, supplied independently of every grant under test. This qualifies the
/// process-local owner, not signed Gateway evidence or production FND durability admission.
#[allow(clippy::too_many_arguments)]
fn current(
    session_id: u8,
    generation: u64,
    lease: u64,
    owner: u64,
    channel: u8,
    active: bool,
    observable: bool,
    pvp: bool,
) -> CurrentTarget {
    let character = CharacterId::decode(&uuid(1)).expect("character");
    let world = WorldId::decode(&uuid(2)).expect("world");
    let channel = ChannelId::decode(&uuid(channel)).expect("channel");
    let facts = FreshAdmissionFacts::new([7; 32], character, world, channel, 1, 1).expect("facts");
    let commit =
        FreshAdmissionCommit::from_facts(session(session_id), facts, 8_u64).expect("commit");
    let state = if active {
        GameSessionState::Active
    } else {
        GameSessionState::Terminal
    };
    let snapshot = GameSessionAuthoritySnapshot::from_current_facts(
        commit,
        state,
        ConnectionGeneration::new(generation).expect("generation"),
        active.then_some(8_u64),
        CharacterLease::new(character, lease).expect("lease"),
        Some(CharacterWorldEligibilityClaimV1::new(character, world)),
        RuntimeScopeRefV1::channel(world, channel),
        ScopeOwnershipGeneration::new(owner).expect("owner"),
    )
    .expect("independently supplied current native facts");
    CurrentTarget::from_current(snapshot, observable, pvp)
}
fn live() -> CurrentTarget {
    current(3, 1, 1, 1, 4, true, true, false)
}
fn grant(runtime: &mut SpectateRuntime, target: &CurrentTarget) -> (GrantRevision, GrantCode) {
    let (revision, code) = runtime
        .set_grant(target, None, Audience::Code)
        .expect("grant");
    (revision, code.expect("code"))
}
fn spatial() -> WorldSpatialObservation {
    WorldSpatialObservation {
        content_generation: [1; 32],
        actor_position: ActorPosition {
            x: 4,
            y: 5,
            floor: 7,
        },
    }
}
fn flush(runtime: &mut SpectateRuntime) {
    while let Some(ordinal) = runtime.next_audit().map(|fact| fact.ordinal) {
        runtime
            .acknowledge_audit(ordinal)
            .expect("fixture consumes audit fact");
    }
}

#[test]
fn spectate_opt_in_secret_rotation_and_stale_writer_conflict() {
    let mut r = runtime();
    let target = live();
    let observer = identity(10);
    assert_eq!(
        r.admit_code(Some(&target), observer, &[0; 16]),
        Err(Refusal::NotAvailable)
    );
    assert!(r.status(&target).is_none());
    let (first, code) = grant(&mut r, &target);
    assert_eq!(format!("{code:?}"), "GrantCode(REDACTED)");
    assert_eq!(code.expose_to_target().len(), 16);
    r.admit_code(Some(&target), observer, code.expose_to_target())
        .expect("admitted");
    let (second, fresh) = r
        .set_grant(&target, Some(first), Audience::Code)
        .expect("rotate");
    assert_ne!(first, second);
    assert_ne!(
        code.expose_to_target(),
        fresh.as_ref().expect("code").expose_to_target()
    );
    assert_eq!(
        r.dispatch(observer, ObserverInput::Keepalive),
        Err(Refusal::NotAvailable)
    );
    assert_eq!(
        r.set_grant(&target, Some(first), Audience::Contacts).err(),
        Some(Refusal::Conflict)
    );
    assert_eq!(r.revoke(&target, first), Err(Refusal::Conflict));
    assert_eq!(r.status(&target), Some((second, 0)));
    assert_eq!(
        r.admit_code(Some(&target), identity(11), code.expose_to_target()),
        Err(Refusal::NotAvailable)
    );
}

#[test]
fn spectate_uniform_missing_revoked_wrong_offline_and_contacts_fail_closed() {
    let mut r = runtime();
    let target = live();
    let (revision, code) = grant(&mut r, &target);
    assert_eq!(
        r.admit_code(None, identity(10), code.expose_to_target()),
        Err(Refusal::NotAvailable)
    );
    assert_eq!(
        r.admit_code(Some(&target), identity(11), &[0; 16]),
        Err(Refusal::NotAvailable)
    );
    assert_eq!(
        r.admit_code(Some(&target), identity(12), &[0; 15]),
        Err(Refusal::NotAvailable)
    );
    r.revoke(&target, revision).expect("revoke");
    assert_eq!(
        r.admit_code(Some(&target), identity(13), code.expose_to_target()),
        Err(Refusal::NotAvailable)
    );
    let offline = current(3, 1, 1, 1, 4, false, true, false);
    assert_eq!(
        r.admit_code(Some(&offline), identity(14), code.expose_to_target()),
        Err(Refusal::NotAvailable)
    );
    let (_, code) = r
        .set_grant(&target, None, Audience::Contacts)
        .expect("contacts consent intent");
    assert!(code.is_none());
    assert_eq!(r.admit_contacts(), Err(Refusal::NotAvailable));
    assert_eq!(
        r.admit_code(Some(&target), identity(15), &[0; 16]),
        Err(Refusal::NotAvailable)
    );
}

#[test]
fn spectate_all_registered_and_future_gameplay_commands_refused_without_side_effect() {
    let mut r = runtime();
    let target = live();
    let (revision, code) = grant(&mut r, &target);
    let observer = identity(10);
    r.admit_code(Some(&target), observer, code.expose_to_target())
        .expect("admit");
    let registry: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json"
    ))
    .expect("registry");
    let commands = registry["command_types"].as_array().expect("commands");
    assert!(!commands.is_empty());
    let before_audit = r.audit.len();
    for command in commands
        .iter()
        .map(|row| u32::try_from(row["id"].as_u64().expect("id")).expect("u32"))
        .chain([0, u32::MAX])
    {
        assert_eq!(
            r.dispatch(observer, ObserverInput::Gameplay(command)),
            Err(Refusal::NotAllowed)
        );
        assert_eq!(r.status(&target), Some((revision, 1)));
        assert!(
            r.observers
                .get(&observer.session)
                .expect("observer")
                .queue
                .is_empty()
        );
        assert_eq!(r.audit.len(), before_audit);
    }
    r.dispatch(observer, ObserverInput::Keepalive)
        .expect("keepalive");
    r.dispatch(observer, ObserverInput::Stop).expect("stop");
    assert_eq!(r.status(&target), Some((revision, 0)));
}

#[test]
fn spectate_each_native_lifecycle_change_ends_grant_and_queued_stream() {
    // Each case changes one current native invariant, independently of retained grant data.
    for changed in [
        current(3, 2, 1, 1, 4, true, true, false),
        current(3, 1, 2, 1, 4, true, true, false),
        current(3, 1, 1, 2, 4, true, true, false),
        current(3, 1, 1, 1, 5, true, true, false),
        current(4, 1, 1, 1, 4, true, true, false),
        current(3, 1, 1, 1, 4, false, true, false),
        current(3, 1, 1, 1, 4, true, false, false),
        current(3, 1, 1, 1, 4, true, true, true),
    ] {
        let mut r = runtime();
        let target = live();
        let (_, code) = grant(&mut r, &target);
        let observer = identity(10);
        r.admit_code(Some(&target), observer, code.expose_to_target())
            .expect("admit");
        r.project(&target, Projection::Spatial(&spatial()))
            .expect("enqueue");
        let mut sent = false;
        assert_eq!(
            r.try_write(&changed, observer, |_| {
                sent = true;
                WriteAttempt::Written
            }),
            Err(Refusal::NotAvailable)
        );
        assert!(!sent);
        assert!(r.grants.is_empty());
        assert!(r.observers.is_empty());
        assert!(
            r.audit
                .iter()
                .any(|fact| matches!(fact.kind, AuditKind::ObserverEnded(_)))
        );
    }
}

#[test]
fn spectate_explicit_death_logout_scope_hooks_and_restart_never_resume() {
    for reason in [
        EndReason::TargetGone,
        EndReason::TargetMoved,
        EndReason::NotObservable,
    ] {
        let mut r = runtime();
        let target = live();
        let (_, code) = grant(&mut r, &target);
        r.admit_code(Some(&target), identity(10), code.expose_to_target())
            .expect("admit");
        r.end_target(target.binding().session, reason);
        assert!(r.observers.is_empty());
        assert!(r.grants.is_empty());
    }
    let mut r = runtime();
    let target = live();
    let (_, code) = grant(&mut r, &target);
    r.admit_code(Some(&target), identity(10), code.expose_to_target())
        .expect("admit");
    drop(r);
    let mut restarted = runtime();
    assert_eq!(
        restarted.admit_code(Some(&target), identity(10), code.expose_to_target()),
        Err(Refusal::NotAvailable)
    );
    assert!(restarted.status(&target).is_none());
}

#[test]
fn spectate_revocation_drops_stalled_frames_before_any_later_write() {
    let mut r = runtime();
    let target = live();
    let (revision, code) = grant(&mut r, &target);
    let observer = identity(10);
    r.admit_code(Some(&target), observer, code.expose_to_target())
        .expect("admit");
    r.project(&target, Projection::Spatial(&spatial()))
        .expect("enqueue");
    let bytes = r
        .observers
        .get(&observer.session)
        .expect("observer")
        .queued_bytes;
    assert!(
        !r.try_write(&target, observer, |_| WriteAttempt::Blocked)
            .expect("blocked")
    );
    assert_eq!(
        r.observers
            .get(&observer.session)
            .expect("observer")
            .queued_bytes,
        bytes
    );
    r.revoke(&target, revision).expect("revoke");
    let mut called = false;
    assert_eq!(
        r.try_write(&target, observer, |_| {
            called = true;
            WriteAttempt::Written
        }),
        Err(Refusal::NotAvailable)
    );
    assert!(!called);
}

#[test]
fn spectate_private_chat_and_rooms_never_enter_world_queue() {
    let mut r = runtime();
    let target = live();
    let (_, code) = grant(&mut r, &target);
    let observer = identity(10);
    r.admit_code(Some(&target), observer, code.expose_to_target())
        .expect("admit");
    for line in [
        ChatLine::Private {
            speaker_name: "secret".into(),
            text: "private".into(),
        },
        ChatLine::Room {
            room: ChatRoom::World,
            speaker_name: "secret".into(),
            text: "room".into(),
        },
        ChatLine::Dropped,
    ] {
        r.project(&target, Projection::Chat(&line)).expect("filter");
    }
    assert!(
        r.observers
            .get(&observer.session)
            .expect("observer")
            .queue
            .is_empty()
    );
    // Inventory/account/command outcome payloads have no Projection constructor at all.
    let line = ChatLine::Local {
        speaker_name: "public".into(),
        text: "hello".into(),
        position: spatial().actor_position,
    };
    r.project(&target, Projection::Chat(&line)).expect("local");
    let expected = oteryn_protocol_oteryn::chat::encode_chat_line(&line).expect("native payload");
    assert!(
        r.try_write(&target, observer, |frame| {
            assert_eq!(frame.bytes(), expected);
            WriteAttempt::Written
        })
        .expect("write")
    );
    assert_eq!(
        r.observers
            .get(&observer.session)
            .expect("observer")
            .queued_bytes,
        0
    );
}

#[test]
fn spectate_overflow_drops_only_slow_observer_and_bounds_inflight_budget() {
    let mut r = runtime();
    let target = live();
    let (revision, code) = grant(&mut r, &target);
    let slow = identity(10);
    let fast = identity(11);
    r.admit_code(Some(&target), slow, code.expose_to_target())
        .expect("slow");
    r.admit_code(Some(&target), fast, code.expose_to_target())
        .expect("fast");
    for _ in 0..=EGRESS_ENTRIES {
        r.project(&target, Projection::Spatial(&spatial()))
            .expect("fanout");
        assert!(
            r.try_write(&target, fast, |_| WriteAttempt::Written)
                .expect("fast write")
        );
    }
    assert!(!r.observers.contains_key(&slow.session));
    assert_eq!(r.status(&target), Some((revision, 1)));
    assert!(
        r.audit
            .iter()
            .any(|a| a.observer_session == Some(slow.session)
                && a.kind == AuditKind::ObserverEnded(EndReason::Overload))
    );
}

#[test]
fn spectate_uniform_monotonic_rate_limit_and_bounded_attempt_table() {
    let mut r = runtime();
    let target = live();
    let (_, code) = grant(&mut r, &target);
    let who = identity(10);
    let start = Instant::now();
    for _ in 0..CODE_ATTEMPTS {
        assert_eq!(
            r.admit_code_at(None, who, &[0; 16], start),
            Err(Refusal::NotAvailable)
        );
    }
    assert_eq!(
        r.admit_code_at(Some(&target), who, code.expose_to_target(), start),
        Err(Refusal::NotAvailable)
    );
    assert_eq!(r.rejected_code_attempts(who.account), 6);
    r.admit_code_at(
        Some(&target),
        who,
        code.expose_to_target(),
        start + CODE_WINDOW,
    )
    .expect("window expired");
    assert_eq!(r.attempts.len(), 1);
    let mut bounded = SpectateRuntime::new(Limits {
        node_sessions: 1,
        target_grants: 1,
        attempt_accounts: 1,
    })
    .expect("limits");
    assert_eq!(
        bounded.admit_code_at(None, identity(20), &[0; 16], start),
        Err(Refusal::NotAvailable)
    );
    assert_eq!(
        bounded.admit_code_at(None, identity(21), &[0; 16], start),
        Err(Refusal::NotAvailable)
    );
    assert_eq!(bounded.attempts.len(), 1);
}

#[test]
fn spectate_target_account_and_node_admission_caps_are_independent() {
    let mut r = runtime();
    let target = live();
    let (_, code) = grant(&mut r, &target);
    for n in 10..60 {
        r.admit_code(Some(&target), identity(n), code.expose_to_target())
            .expect("within target cap");
    }
    assert_eq!(
        r.admit_code(Some(&target), identity(60), code.expose_to_target()),
        Err(Refusal::Full)
    );
    let mut r = runtime();
    let (_, code) = grant(&mut r, &target);
    for n in 10..12 {
        let mut who = identity(n);
        who.account = account(70);
        r.admit_code(Some(&target), who, code.expose_to_target())
            .expect("account cap");
    }
    let mut third = identity(12);
    third.account = account(70);
    assert_eq!(
        r.admit_code(Some(&target), third, code.expose_to_target()),
        Err(Refusal::Full)
    );
    let mut r = SpectateRuntime::new(Limits {
        node_sessions: 1,
        target_grants: 1,
        attempt_accounts: 8,
    })
    .expect("limits");
    let (_, code) = grant(&mut r, &target);
    flush(&mut r);
    r.admit_code(Some(&target), identity(10), code.expose_to_target())
        .expect("first");
    assert_eq!(
        r.admit_code(Some(&target), identity(11), code.expose_to_target()),
        Err(Refusal::Full)
    );
}

#[test]
fn spectate_audit_reservation_never_blocks_privacy_end_or_leaks_secret() {
    let mut r = SpectateRuntime::new(Limits {
        node_sessions: 1,
        target_grants: 1,
        attempt_accounts: 8,
    })
    .expect("limits");
    let target = live();
    let (revision, code) = grant(&mut r, &target);
    r.admit_code(Some(&target), identity(10), code.expose_to_target())
        .expect("admit");
    assert!(!r.can_open());
    r.revoke(&target, revision)
        .expect("revoke despite full audit reservations");
    assert_eq!(r.audit.len(), r.audit_limit);
    assert!(r.observers.is_empty());
    assert_eq!(
        r.set_grant(&target, None, Audience::Code).err(),
        Some(Refusal::AuditBackpressure)
    );
    assert_eq!(r.acknowledge_audit(99), Err(Refusal::Conflict));
    assert_eq!(r.audit.len(), 4);
    flush(&mut r);
    grant(&mut r, &target);
    assert!(r.audit.iter().all(|fact| fact.observer_session.is_none()));
}

#[test]
fn spectate_observer_connection_substitution_cannot_stop_or_read_other_session() {
    let mut r = runtime();
    let target = live();
    let (revision, code) = grant(&mut r, &target);
    let observer = identity(10);
    r.admit_code(Some(&target), observer, code.expose_to_target())
        .expect("admit");
    r.project(&target, Projection::Spatial(&spatial()))
        .expect("enqueue");
    for changed in [
        ObserverIdentity {
            generation: ConnectionGeneration::new(2).expect("generation"),
            ..observer
        },
        ObserverIdentity {
            account: account(11),
            ..observer
        },
    ] {
        assert_eq!(
            r.dispatch(changed, ObserverInput::Stop),
            Err(Refusal::NotAvailable)
        );
        let mut sent = false;
        assert_eq!(
            r.try_write(&target, changed, |_| {
                sent = true;
                WriteAttempt::Written
            }),
            Err(Refusal::NotAvailable)
        );
        assert!(!sent);
        assert_eq!(r.status(&target), Some((revision, 1)));
    }
}

#[test]
fn spectate_invalid_limits_and_revision_exhaustion_write_nothing() {
    assert!(matches!(
        SpectateRuntime::new(Limits {
            node_sessions: 0,
            target_grants: 1,
            attempt_accounts: 1
        }),
        Err(Refusal::InvalidLimits)
    ));
    assert!(matches!(
        SpectateRuntime::new(Limits {
            node_sessions: usize::MAX,
            target_grants: 1,
            attempt_accounts: 1
        }),
        Err(Refusal::InvalidLimits)
    ));
    let mut r = runtime();
    r.next_revision = u64::MAX;
    assert_eq!(
        r.set_grant(&live(), None, Audience::Code).err(),
        Some(Refusal::Full)
    );
    assert!(r.grants.is_empty());
    assert!(r.audit.is_empty());
}

#[test]
fn spectate_rolling_minute_budget_covers_wrong_and_correct_codes_at_rollover() {
    for correct_at_rollover in [false, true] {
        let mut r = runtime();
        let target = live();
        let (_, code) = grant(&mut r, &target);
        let who = identity(10);
        let start = Instant::now();
        assert_eq!(
            r.admit_code_at(None, who, &[0; 16], start),
            Err(Refusal::NotAvailable)
        );
        for _ in 0..4 {
            assert_eq!(
                r.admit_code_at(None, who, &[0; 16], start + Duration::from_secs(59)),
                Err(Refusal::NotAvailable)
            );
        }
        let supplied = if correct_at_rollover {
            code.expose_to_target().as_slice()
        } else {
            &[0; 16]
        };
        let first = r.admit_code_at(Some(&target), who, supplied, start + CODE_WINDOW);
        assert_eq!(
            first,
            if correct_at_rollover {
                Ok(())
            } else {
                Err(Refusal::NotAvailable)
            }
        );
        // Only t=0 expired. The four t=59 attempts still consume four slots; a second valid
        // identity of the same account would fit its account cap but has no remaining budget.
        let mut second = identity(11);
        second.account = who.account;
        for _ in 0..4 {
            assert_eq!(
                r.admit_code_at(
                    Some(&target),
                    second,
                    code.expose_to_target(),
                    start + CODE_WINDOW
                ),
                Err(Refusal::NotAvailable)
            );
        }
        assert_eq!(
            r.attempts.get(&who.account).expect("window").attempts.len(),
            5
        );
        assert_eq!(
            r.status(&target).expect("grant").1,
            usize::from(correct_at_rollover)
        );
    }
}

#[test]
fn spectate_failed_partial_transport_write_closes_only_that_observer() {
    let mut r = runtime();
    let target = live();
    let (revision, code) = grant(&mut r, &target);
    let observer = identity(10);
    r.admit_code(Some(&target), observer, code.expose_to_target())
        .expect("admit");
    r.project(&target, Projection::Spatial(&spatial()))
        .expect("enqueue");
    assert_eq!(
        r.try_write(&target, observer, |_| WriteAttempt::Failed),
        Err(Refusal::NotAvailable)
    );
    assert_eq!(r.status(&target), Some((revision, 0)));
    assert!(r.observers.is_empty());
}
