//! Test-only adapters over the existing native admission fixture and current-facts constructor.
#![allow(clippy::expect_used)]
use super::*;
use oteryn_gameplay_foundation::foundation::{
    AuthenticatedTransportRefV1, CharacterLease, CharacterWorldEligibilityClaimV1,
    ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts, GameSessionAuthoritySnapshot,
    GameSessionId, GameSessionState, ScopeOwnershipGeneration,
};
use std::num::{NonZeroU64, NonZeroUsize};

pub fn limits(node: usize, audit: usize) -> Limits {
    Limits {
        node_sessions: NonZeroUsize::new(node).expect("node"),
        rate_accounts: NonZeroUsize::new(node).expect("accounts"),
        audit_facts: NonZeroUsize::new(audit).expect("audit"),
    }
}

pub fn observer(marker: u8) -> ObserverIdentity {
    let mut id = [1, 1, 1, 1, 1, 1, 0x71, 1, 0x81, 1, 1, 1, 1, 1, 1, marker];
    let session = GameSessionId::decode(&id).expect("observer");
    id[0] = marker;
    ObserverIdentity {
        session,
        generation: NonZeroU64::new(1).expect("generation"),
        account: id,
    }
}

pub fn facts(generation: u64, lease: u64, scope: u64) -> TargetFacts {
    let native = FreshAdmissionFacts::shared_test_facts(7, lease, scope).expect("existing fixture");
    let transport = AuthenticatedTransportRefV1::decode(&[9; 16]).expect("transport");
    let commit =
        FreshAdmissionCommit::from_facts(observer(200).session, native, transport).expect("commit");
    let authority = GameSessionAuthoritySnapshot::from_current_facts(
        commit,
        GameSessionState::Active,
        ConnectionGeneration::new(generation).expect("generation"),
        Some(transport),
        CharacterLease::new(commit.character_id(), lease).expect("lease"),
        Some(CharacterWorldEligibilityClaimV1::new(
            commit.character_id(),
            commit.world_id(),
        )),
        oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
            commit.world_id(),
            commit.channel_id(),
        ),
        ScopeOwnershipGeneration::new(scope).expect("scope"),
    )
    .expect("current native facts");
    TargetFacts {
        authority,
        alive: true,
        observable: true,
        house: None,
    }
}

pub fn grant(owner: &mut SpectateRuntime, facts: TargetFacts) -> (u64, [u8; 16]) {
    match owner.apply(Request::GrantSet {
        facts,
        expected_revision: 0,
        audience: oteryn_protocol_oteryn::spectate::Audience::Code,
    }) {
        Reply::Grant { revision, code } => (revision, code),
        other => unreachable!("grant refused: {other:?}"),
    }
}

use oteryn_protocol_oteryn::{
    chat::{ChatLine, ChatSpeaker, ChatSpeechMode},
    world_spatial::ActorPosition,
};
pub fn local() -> ChatLine {
    ChatLine::Local {
        speaker: ChatSpeaker {
            identity: [1; 16],
            generation: std::num::NonZeroU64::new(1).expect("generation"),
        },
        speaker_name: "Speaker".into(),
        mode: ChatSpeechMode::Say,
        text: "hello".into(),
        position: ActorPosition {
            x: 1,
            y: 1,
            floor: 0,
        },
    }
}
