//! PROGRESSION-OWNER-1 (ARCH-PROGRESSION-SOURCE-0 §1.4-§1.5, §2.2): the session binding the
//! progression step installs, as both the death and the kill path read it. The durable step
//! itself is exercised against PostgreSQL (`character_progression_admission_postgres_cases`).
#![allow(clippy::expect_used)]

use super::character_progression_binding::ProgressionUnbound;
use super::{AdmittedSession, SessionProgressionBinding, actor_spell, player_death_request};
use crate::combat::CreatureDeathRewardInput;
use crate::content::character_progression_content::{
    CHARACTER_EXPERIENCE_TABLE_LEVELS, CharacterProgressionContent, tests::section_bytes,
};
use crate::foundation::{ChannelId, MovementLocalPosition, WorldId};

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
