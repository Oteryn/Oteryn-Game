//! PROGRESSION-OWNER-1 (ARCH-PROGRESSION-SOURCE-0 §1.4-§1.5, §2.2): the session binding the
//! progression step installs, as both the death and the kill path read it. The durable step
//! itself is exercised against PostgreSQL (`character_progression_admission_postgres_cases`).
#![allow(clippy::expect_used)]

use super::character_progression_binding::ProgressionUnbound;
use super::{
    AdmittedSession, AuthorizedRespawn, DeathCommit, ProgressionSessions, RespawnConsumption,
    SessionDeath, SessionProgressionBinding, SettledRespawn, TransitionFence, actor_spell,
    death_commit_refusal, place_authorized_respawn, player_death_request, respawn_in_order,
    session_progression_binding, settle_session_death, settled_respawn_consumed,
    settled_respawn_disposition,
};
use crate::combat::CreatureDeathRewardInput;
use crate::content::character_progression_content::{
    CHARACTER_EXPERIENCE_TABLE_LEVELS, CharacterProgressionContent, tests::section_bytes,
};
use crate::durability::DurabilityError;
use crate::durability::character_death::{ConsumedRespawn, PlayerDeathOccurrence};
use crate::durability::character_progression::CharacterProgressionError;
use crate::foundation::{
    ChannelContentPin, ChannelId, ChannelRuntimeV1, CharacterId, CharacterLease, ExactActorRef,
    GameSessionId, MovementLocalPosition, NodeId, ScopeOwnershipGeneration, WorldId,
};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

const ID: [u8; 16] = [
    0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x72, 0x22, 0x92, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
];

fn content() -> CharacterProgressionContent {
    CharacterProgressionContent::decode(&section_bytes()).expect("progression section")
}

fn binding() -> SessionProgressionBinding {
    let policy = content()
        .policy_for("profile-1", "ruleset-1", "content-1")
        .expect("policy");
    SessionProgressionBinding::from_content(policy)
}

#[test]
fn every_unbound_reason_is_logged_by_its_spec_name() {
    let reasons = [
        ProgressionUnbound::Uninitialized,
        ProgressionUnbound::ContextMismatch,
        ProgressionUnbound::StaleRevision,
        ProgressionUnbound::InitializationUnavailable,
    ]
    .map(ProgressionUnbound::reason);
    assert_eq!(
        reasons,
        [
            "progression_uninitialized",
            "progression_context_mismatch",
            "progression_stale_revision",
            "progression_initialization_unavailable",
        ]
    );
}

#[test]
fn the_binding_carries_the_policy_and_the_character_root_revisions() {
    let content = content();
    let binding = binding();
    assert_eq!(binding.context, binding.policy.context);
    assert_eq!(binding.context.profile, "profile-1");
    assert_eq!(binding.context.ruleset, "ruleset-1");
    assert_eq!(binding.context.content, "content-1");
    assert_eq!(binding.context.simulation, content.simulation());
    assert_eq!(binding.context.evidence, content.evidence());
    assert_eq!(binding.context.declaration, content.declaration());
    assert_eq!(binding.policy_revision, content.policy_revision());
    assert_eq!(binding.reward_revision, content.reward_revision());
    assert_eq!(binding.policy.policy_revision, binding.policy_revision);
    assert_eq!(binding.policy.reward_revision, binding.reward_revision);
    assert_eq!(
        binding.policy.thresholds.len(),
        CHARACTER_EXPERIENCE_TABLE_LEVELS
    );
}

#[test]
fn the_death_request_is_formed_from_the_session_binding() {
    let binding = binding();
    let occurrence =
        crate::durability::character_death::PlayerDeathOccurrence::from_bytes(ID).expect("occ");
    let world = WorldId::decode(&ID).expect("world");
    let channel = ChannelId::decode(&ID).expect("channel");
    let cell = MovementLocalPosition {
        x: 1,
        y: 2,
        floor: 7,
    };
    let request = player_death_request(
        &binding,
        (world, channel),
        [0x01; 32],
        actor_spell::PlayerDeath { occurrence, cell },
        cell,
    );
    assert_eq!(request.context, binding.context);
    assert_eq!(request.policy_revision, binding.policy_revision);
    assert_eq!(request.reward_revision, binding.reward_revision);
    assert_eq!(request.policy, binding.policy);
}

/// The kill path's input and the death path's binding share the table width: both are
/// `CHARACTER_EXPERIENCE_TABLE_LEVELS`, so the kill settle passes the session binding as is.
#[test]
fn the_kill_and_death_paths_bind_the_same_table_width() {
    fn kill_progression(
        input: CreatureDeathRewardInput<CHARACTER_EXPERIENCE_TABLE_LEVELS>,
    ) -> Option<SessionProgressionBinding> {
        input.progression
    }
    let _ = kill_progression;
    assert_eq!(
        binding().policy.thresholds.len(),
        CHARACTER_EXPERIENCE_TABLE_LEVELS
    );
}

/// The binding lives in the seam's session map, not in the admitted session, which stays a
/// plain copyable value (a resume copies it out of `lost`).
#[test]
fn the_admitted_session_stays_copy() {
    fn copy<T: Copy>() {}
    copy::<AdmittedSession>();
}

fn session(seed: u8) -> GameSessionId {
    let mut id = ID;
    id[15] = seed;
    GameSessionId::decode(&id).expect("session")
}

fn bound(sessions: &[(GameSessionId, Arc<SessionProgressionBinding>)]) -> ProgressionSessions {
    let map = ProgressionSessions::default();
    {
        let mut bound = map.lock().expect("sessions");
        for (session, binding) in sessions {
            bound.insert(*session, Arc::clone(binding));
        }
    }
    map
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime")
        .block_on(future)
}

/// #1964 Codex P1 4243820239 (ARCH-PROGRESSION-SOURCE-0 §1.5, DEATH-2 §4.3-§4.4) and
/// DEATH-RESTART-SAFETY-1 (first player death decision §4.5): a session bound at admission whose
/// Character interpretation is advanced by the control plane mid-session has its durable death
/// refused with `ProgressionContextMismatch`. After exactly one durable attempt the session is
/// downgraded to unbound and the death is decided `NoDurableDeath`: neither a retry tick nor a
/// later death of that session attempts the write again, and none of them respawns the actor
/// without a durable death. Another session's binding is untouched.
#[test]
fn a_binding_refused_mid_session_never_respawns_without_a_durable_death() {
    let (refused, other) = (session(1), session(2));
    let binding = Arc::new(binding());
    let sessions = bound(&[
        (refused, Arc::clone(&binding)),
        (other, Arc::new(self::binding())),
    ]);
    let attempts = Cell::new(0_u32);
    let attempt = |decided: DeathCommit| {
        let (attempts, binding) = (&attempts, &binding);
        move |used: Arc<SessionProgressionBinding>| {
            assert!(Arc::ptr_eq(&used, binding));
            attempts.set(attempts.get() + 1);
            async move { decided }
        }
    };
    block_on(async {
        // The interpretation advanced after the bind: the durable writer refuses the context.
        assert_eq!(
            death_commit_refusal(&CharacterProgressionError::ProgressionContextMismatch),
            DeathCommit::ContextMismatch
        );
        assert_eq!(
            settle_session_death(&sessions, refused, attempt(DeathCommit::ContextMismatch)).await,
            SessionDeath::NoDurableDeath
        );
        assert_eq!(attempts.get(), 1);
        assert!(session_progression_binding(&sessions, refused).is_none());
        assert!(session_progression_binding(&sessions, other).is_some());
        // The session is unbound now: later settles attempt nothing and never respawn.
        for _ in 0..3 {
            assert_eq!(
                settle_session_death(&sessions, refused, attempt(DeathCommit::Settled)).await,
                SessionDeath::NoDurableDeath
            );
        }
        assert_eq!(attempts.get(), 1);
    });
}

/// Every other durable outcome keeps the binding: a settled death keeps the session bound, and
/// an attempt without an outcome (or any refusal other than the context) keeps the actor dead for
/// the next cadence, as before. A binding replaced between the attempt and its refusal is not the
/// one refused, so it is kept.
#[test]
fn only_a_context_refusal_of_the_used_binding_unbinds_the_session() {
    let target = session(3);
    let binding = Arc::new(binding());
    let sessions = bound(&[(target, Arc::clone(&binding))]);
    for error in [
        CharacterProgressionError::RespawnPending,
        CharacterProgressionError::CharacterRevisionMismatch,
        CharacterProgressionError::AuthorityRejected,
        CharacterProgressionError::Unavailable(DurabilityError::Unavailable),
    ] {
        assert_eq!(death_commit_refusal(&error), DeathCommit::Unavailable);
    }
    block_on(async {
        assert_eq!(
            settle_session_death(&sessions, target, |_| async { DeathCommit::Unavailable }).await,
            SessionDeath::Retry
        );
        assert_eq!(
            settle_session_death(&sessions, target, |_| async { DeathCommit::Settled }).await,
            SessionDeath::Durable
        );
        let kept = session_progression_binding(&sessions, target).expect("bound");
        assert!(Arc::ptr_eq(&kept, &binding));
        // The refused attempt used `binding`; a replacement installed meanwhile stays.
        let replacement = Arc::new(self::binding());
        let refused = settle_session_death(&sessions, target, |_| {
            sessions
                .lock()
                .expect("sessions")
                .insert(target, Arc::clone(&replacement));
            async { DeathCommit::ContextMismatch }
        })
        .await;
        assert_eq!(refused, SessionDeath::NoDurableDeath);
        let kept = session_progression_binding(&sessions, target).expect("replacement");
        assert!(Arc::ptr_eq(&kept, &replacement));
    });
}

/// DEATH-RESTART-SAFETY-1 (first player death decision §4.5): a session without a progression
/// binding (a World pin without a death policy, or any unbound admission of §1.5) commits no
/// durable death, so its player is never respawned: no durable attempt is made, no placement,
/// consumption or refill runs, and every later cadence decides the same.
#[test]
fn a_session_without_a_death_policy_never_respawns_its_dead_player() {
    let sessions = bound(&[]);
    let unbound = session(4);
    let steps = RefCell::new(Vec::new());
    block_on(async {
        for _ in 0..3 {
            let refilled = respawn_in_order(
                settle_session_death(&sessions, unbound, |_| {
                    steps.borrow_mut().push("commit");
                    async { DeathCommit::Settled }
                }),
                async {
                    steps.borrow_mut().push("authorize");
                    Some(())
                },
                |()| step(&steps, "place", true),
                |()| step(&steps, "consume", true),
                async {
                    steps.borrow_mut().push("refill");
                    Some(())
                },
            )
            .await;
            assert_eq!(refilled, None);
        }
    });
    assert!(steps.borrow().is_empty(), "{:?}", steps.borrow());
}

async fn step(steps: &RefCell<Vec<&'static str>>, name: &'static str, outcome: bool) -> bool {
    steps.borrow_mut().push(name);
    outcome
}

/// DEATH-RESTART-SAFETY-1 (first player death decision §4.5, DEATH-0 §3.4): the respawn is
/// authorized against the session's fresh durable authority before anything moves, the pending
/// respawn is consumed only after the actor is placed, and the vitals are refilled (ending the
/// death) only after the consumption. A step that fails stops the chain with the actor still
/// dead, so a restart at that point finds the pending respawn recorded for the next admission.
#[test]
fn an_in_session_respawn_authorizes_then_places_then_consumes_then_refills() {
    let run = |settled: SessionDeath, authorized: bool, placed: bool, consumed: bool| {
        let steps = RefCell::new(Vec::new());
        let refilled = block_on(respawn_in_order(
            async {
                steps.borrow_mut().push("settle");
                settled
            },
            async {
                steps.borrow_mut().push("authorize");
                authorized.then_some(())
            },
            |()| step(&steps, "place", placed),
            |()| step(&steps, "consume", consumed),
            async {
                steps.borrow_mut().push("refill");
                Some(7_u64)
            },
        ));
        (refilled, steps.into_inner())
    };
    assert_eq!(
        run(SessionDeath::Durable, true, true, true),
        (
            Some(7),
            vec!["settle", "authorize", "place", "consume", "refill"]
        )
    );
    // Codex review of #2004: a settle that replayed a retained receipt checked no current
    // authority, so a terminal or superseded session refused by the fresh authorization moves
    // nothing, clears nothing and consumes nothing.
    assert_eq!(
        run(SessionDeath::Durable, false, true, true),
        (None, vec!["settle", "authorize"])
    );
    // A consumption without an outcome keeps the actor dead and its row recorded.
    assert_eq!(
        run(SessionDeath::Durable, true, true, false),
        (None, vec!["settle", "authorize", "place", "consume"])
    );
    // A placement that cannot complete consumes nothing.
    assert_eq!(
        run(SessionDeath::Durable, true, false, true),
        (None, vec!["settle", "authorize", "place"])
    );
    // No durable outcome: nothing is authorized, placed, consumed or refilled.
    for undecided in [SessionDeath::Retry, SessionDeath::NoDurableDeath] {
        assert_eq!(run(undecided, true, true, true), (None, vec!["settle"]));
    }
}

/// DEATH-RESTART-SAFETY-1: after a durable settle and the placement, the consumption settles the
/// respawn when it deletes this occurrence's row, or finds none (an earlier consumption whose
/// acknowledgement was lost). An error or another occurrence's row settles nothing.
#[test]
fn a_settled_respawn_is_consumed_by_its_own_occurrence_or_already_gone() {
    let occurrence = |tag: u8| {
        let mut id = ID;
        id[15] = tag;
        PlayerDeathOccurrence::from_bytes(id).expect("occurrence")
    };
    let row = |tag: u8| {
        Ok::<_, &str>(Some(ConsumedRespawn {
            occurrence: occurrence(tag),
            respawn_position: vec![0; 10],
        }))
    };
    assert!(settled_respawn_consumed(occurrence(1), row(1)));
    assert!(settled_respawn_consumed(occurrence(1), Ok::<_, &str>(None)));
    assert!(!settled_respawn_consumed(occurrence(1), row(2)));
    assert!(!settled_respawn_consumed(occurrence(1), Err("consumption")));
}

/// DEATH-RESTART-SAFETY-1 (Codex review of #2004): the in-session placement is volatile, so a
/// restart right after the consumption must still land the actor on the recorded respawn. The
/// row is consumed only when it records the cell a fresh admission writes; a row recording any
/// other cell, or none that decodes, is kept for the next admission, and a row that is gone or
/// belongs to another occurrence leaves nothing of this death to consume.
#[test]
fn a_settled_respawn_is_consumed_only_when_an_admission_reproduces_it() {
    let occurrence = |tag: u8| {
        let mut id = ID;
        id[15] = tag;
        PlayerDeathOccurrence::from_bytes(id).expect("occurrence")
    };
    let admission = MovementLocalPosition {
        x: 3,
        y: 4,
        floor: 7,
    };
    let row = |tag: u8, respawn_position: Vec<u8>| ConsumedRespawn {
        occurrence: occurrence(tag),
        respawn_position,
    };
    let at_admission = vec![0, 0, 0, 3, 0, 0, 0, 4, 0, 7];
    let elsewhere = vec![0, 0, 0, 3, 0, 0, 0, 5, 0, 7];
    assert_eq!(
        settled_respawn_disposition(
            occurrence(1),
            admission,
            Some(&row(1, at_admission.clone()))
        ),
        SettledRespawn::Reproduced
    );
    assert_eq!(
        settled_respawn_disposition(occurrence(1), admission, Some(&row(1, elsewhere))),
        SettledRespawn::Kept
    );
    assert_eq!(
        settled_respawn_disposition(occurrence(1), admission, Some(&row(1, vec![0; 9]))),
        SettledRespawn::Kept
    );
    assert_eq!(
        settled_respawn_disposition(occurrence(1), admission, None),
        SettledRespawn::Gone
    );
    assert_eq!(
        settled_respawn_disposition(occurrence(1), admission, Some(&row(2, at_admission))),
        SettledRespawn::Gone
    );
}

/// A Channel owner under scope ownership generation 1 whose pinned entry start, the respawn
/// position, is (3, 4, 7), with the player of `session(5)` positioned at (10, 10, 7) and bound to
/// lease generation 1 of the Character `ID`, as its admission bound it.
fn respawning_owner() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
    let world = WorldId::decode(&uuid_v7(0x60)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid_v7(0x61)).expect("channel"),
        NodeId::decode(&uuid_v7(0x62)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::from_activation(world, 1, [1; 32], [2; 32], [3; 32], [4; 32], (3, 4, 7)),
    )
    .expect("runtime");
    let session = session(5);
    let reservation = runtime.reserve_fresh_session(session).expect("reserve");
    let actor = runtime.commit_fresh_session(reservation).expect("commit");
    runtime
        .initialize_movement_test_position(
            actor,
            MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7,
            },
        )
        .expect("player position");
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(CharacterId::decode(&ID).expect("character"), 1).expect("lease"),
        )
        .expect("bind lease");
    (runtime, actor, session)
}

fn uuid_v7(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
    ]
}

/// The authority a fresh durable read of `session(5)` proved current before the placement: lease
/// generation 1 of the Character `ID` under scope ownership generation 1. Written out from those
/// facts, never read back from the owner whose binding the placement checks.
fn authorized() -> AuthorizedRespawn {
    AuthorizedRespawn {
        consumption: RespawnConsumption::Consume,
        character_id: crate::domain::CharacterId::from_bytes(ID).expect("character"),
        lease_generation: 1,
        scope_generation: ScopeOwnershipGeneration::new(1).expect("scope generation"),
    }
}

/// DEATH-RESTART-SAFETY-1 (third Codex review of #2004): the authorization releases the
/// Character's revision slot before the placement waits for the runtime lock, so the session can
/// end or be superseded in that window. The placement revalidates the authorized authority against
/// the owner's binding under the lock and fails closed: a terminal release that fenced the slot,
/// a successor rebound to it, or an authorization of another lease generation, Character or scope
/// ownership generation moves nothing, so no respawn condition is cleared and the actor stays
/// dead. Each case changes exactly one fact; only the binding the authorization proved is placed.
#[test]
fn a_session_ended_between_authorization_and_placement_moves_nothing() {
    let entry = MovementLocalPosition {
        x: 3,
        y: 4,
        floor: 7,
    };
    // A terminal release fenced the slot (CHARM-DESC-FENCE-LEASE step (a)) after the authorizing
    // read, while the placement waited for the lock.
    let (mut runtime, actor, session) = respawning_owner();
    let before = runtime.read_actor_position(actor).expect("position");
    TransitionFence::Transition(1)
        .fence(&mut runtime, actor, session)
        .expect("release fence");
    assert!(!place_authorized_respawn(
        &mut runtime,
        actor,
        session,
        &authorized()
    ));
    assert_eq!(runtime.read_actor_position(actor).expect("kept"), before);
    // A release that left the session holding its lease lifts the fence; the same authority
    // then places the actor at the respawn position.
    assert_eq!(
        TransitionFence::Transition(1).lift(&mut runtime, actor, session),
        Ok(true)
    );
    assert!(place_authorized_respawn(
        &mut runtime,
        actor,
        session,
        &authorized()
    ));
    let placed = runtime.read_actor_position(actor).expect("placed");
    assert_eq!(placed.position(), entry);
    assert_eq!(placed.revision(), before.revision() + 1);

    // A successor session took the slot over with a newer lease of the same Character.
    let (mut runtime, actor, session) = respawning_owner();
    let before = runtime.read_actor_position(actor).expect("position");
    runtime
        .rebind_player_session(
            actor,
            session,
            self::session(6),
            CharacterLease::new(CharacterId::decode(&ID).expect("character"), 2).expect("lease"),
        )
        .expect("successor rebind");
    assert!(!place_authorized_respawn(
        &mut runtime,
        actor,
        session,
        &authorized()
    ));
    assert_eq!(runtime.read_actor_position(actor).expect("kept"), before);

    // An authorization of authority other than the owner's binding, one fact at a time.
    let (mut runtime, actor, session) = respawning_owner();
    let before = runtime.read_actor_position(actor).expect("position");
    let mut other_character = ID;
    other_character[15] = 0x31;
    for stale in [
        AuthorizedRespawn {
            lease_generation: 2,
            ..authorized()
        },
        AuthorizedRespawn {
            character_id: crate::domain::CharacterId::from_bytes(other_character)
                .expect("other character"),
            ..authorized()
        },
        AuthorizedRespawn {
            scope_generation: ScopeOwnershipGeneration::new(2).expect("scope generation"),
            ..authorized()
        },
    ] {
        assert!(!place_authorized_respawn(
            &mut runtime,
            actor,
            session,
            &stale
        ));
        assert_eq!(runtime.read_actor_position(actor).expect("kept"), before);
    }
    assert!(place_authorized_respawn(
        &mut runtime,
        actor,
        session,
        &authorized()
    ));
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .expect("placed")
            .position(),
        entry
    );
}
