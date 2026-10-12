//! PROGRESSION-OWNER-1 (ARCH-PROGRESSION-SOURCE-0 §1.4-§1.5, §2.2): the session binding the
//! progression step installs, as both the death and the kill path read it. The durable step
//! itself is exercised against PostgreSQL (`character_progression_admission_postgres_cases`).
#![allow(clippy::expect_used)]

use super::character_progression_binding::ProgressionUnbound;
use super::{
    AdmittedSession, DeathCommit, ProgressionSessions, SessionProgressionBinding, actor_spell,
    death_commit_refusal, player_death_request, session_progression_binding, settle_session_death,
};
use crate::combat::CreatureDeathRewardInput;
use crate::content::character_progression_content::{
    CHARACTER_EXPERIENCE_TABLE_LEVELS, CharacterProgressionContent, tests::section_bytes,
};
use crate::durability::DurabilityError;
use crate::durability::character_progression::CharacterProgressionError;
use crate::foundation::{ChannelId, GameSessionId, MovementLocalPosition, WorldId};
use std::cell::Cell;
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

/// #1964 Codex P1 4243820239 (ARCH-PROGRESSION-SOURCE-0 §1.5, DEATH-2 §4.3-§4.4): a session
/// bound at admission whose Character interpretation is advanced by the control plane mid-session
/// has its durable death refused with `ProgressionContextMismatch`. The death then completes on
/// the non-durable respawn path after exactly one durable attempt: the session is downgraded to
/// unbound, and neither a retry tick nor a later death of that session attempts the write again,
/// so the actor is not left dead retrying for ever. Another session's binding is untouched.
#[test]
fn a_binding_refused_mid_session_settles_its_death_non_durably_once() {
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
        assert!(
            settle_session_death(&sessions, refused, attempt(DeathCommit::ContextMismatch)).await
        );
        assert_eq!(attempts.get(), 1);
        assert!(session_progression_binding(&sessions, refused).is_none());
        assert!(session_progression_binding(&sessions, other).is_some());
        // The session is unbound now: later settles respawn at once with no durable attempt.
        for _ in 0..3 {
            assert!(settle_session_death(&sessions, refused, attempt(DeathCommit::Settled)).await);
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
        assert!(
            !settle_session_death(&sessions, target, |_| async { DeathCommit::Unavailable }).await
        );
        assert!(settle_session_death(&sessions, target, |_| async { DeathCommit::Settled }).await);
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
        assert!(refused);
        let kept = session_progression_binding(&sessions, target).expect("replacement");
        assert!(Arc::ptr_eq(&kept, &replacement));
        // An unknown session respawns non-durably without an attempt.
        assert!(
            settle_session_death(&sessions, session(4), |_| async {
                DeathCommit::Unavailable
            })
            .await
        );
    });
}
