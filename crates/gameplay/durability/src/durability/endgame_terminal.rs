//! W11 consumer of the existing shared W02 owner's terminal unsuccessful event.
//!
//! This module owns only the retained group decision and Quest descendants. It
//! never allocates an encounter, detects timeout/death/disconnect, or accepts a
//! client terminal declaration. Until W02 is composed, the sealed production
//! owner returns `ContentUnavailable` (PR2784 ANSWER6547426887).
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{CharacterProgressionError, CurrentCharacterGameplayFence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};

mod owner;
mod storage;
pub use owner::{ProvenTerminalOutcome, SharedTerminalOwner, UnavailableSharedTerminalOwner};
#[cfg(any(test, feature = "test-support"))]
pub use owner::{TestOwnerState, TestSharedTerminalOwner};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FinalEncounter {
    Megalomania,
    Bakragore,
}
impl FinalEncounter {
    const fn code(self) -> i16 {
        match self {
            Self::Megalomania => 1,
            Self::Bakragore => 2,
        }
    }
    const fn mask(self) -> u8 {
        match self {
            Self::Megalomania => 31,
            Self::Bakragore => 15,
        }
    }
}

/// Server-resolved retained owner identity and current participant fences.
/// The caller holds all native Character sequencer slots in CharacterId order
/// until this method returns, then invalidates those slots' revision cursors.
#[derive(Clone, Debug)]
pub struct EndgameFailureRequest {
    pub attempt: [u8; 32],
    pub encounter: FinalEncounter,
    pub participants: Vec<CurrentCharacterGameplayFence>,
}
impl EndgameFailureRequest {
    fn canonicalize(&mut self) -> Result<()> {
        self.participants.sort_by_key(|member| member.character_id);
        let Some(first) = self.participants.first() else {
            return Err(EndgameTerminalError::InvalidInput);
        };
        if self.attempt == [0; 32]
            || self.participants.len() > 5
            || self
                .participants
                .windows(2)
                .any(|pair| pair[0].character_id == pair[1].character_id)
            || self.participants.iter().any(|member| {
                member.runtime_scope != first.runtime_scope
                    || member.scope_ownership_generation != first.scope_ownership_generation
            })
        {
            return Err(EndgameTerminalError::InvalidInput);
        }
        Ok(())
    }
    fn participant_bytes(&self) -> Vec<u8> {
        self.participants
            .iter()
            .flat_map(|member| member.character_id.as_bytes().iter().copied())
            .collect()
    }
    fn scope(&self) -> Result<(RuntimeScopeRefV1, u64)> {
        self.participants
            .first()
            .map(|member| {
                (
                    member.runtime_scope,
                    member.scope_ownership_generation.get(),
                )
            })
            .ok_or(EndgameTerminalError::InvalidInput)
    }
    fn binding(&self) -> Result<[u8; 32]> {
        let (scope, generation) = self.scope()?;
        let (kind, id) = scope_parts(scope);
        Ok(Sha256::new()
            .chain_update(b"oteryn:endgame-terminal:v1")
            .chain_update(self.attempt)
            .chain_update(self.encounter.code().to_be_bytes())
            .chain_update(scope.world_id().as_bytes())
            .chain_update(kind.to_be_bytes())
            .chain_update(id)
            .chain_update(generation.to_be_bytes())
            .chain_update(self.participant_bytes())
            .finalize()
            .into())
    }
    fn authority_binding(&self) -> Result<[u8; 32]> {
        let mut hash = Sha256::new().chain_update(self.binding()?);
        for member in &self.participants {
            hash = hash
                .chain_update(member.game_session_id.as_bytes())
                .chain_update(member.connection_generation.get().to_be_bytes())
                .chain_update(member.character_lease_generation.to_be_bytes())
                .chain_update(member.expected_character_revision.get().to_be_bytes());
        }
        Ok(hash.finalize().into())
    }
}

fn scope_parts(scope: RuntimeScopeRefV1) -> (i16, [u8; 16]) {
    match scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } => (1, *channel_id.as_bytes()),
        RuntimeScopeRefV1::Instance { instance_id, .. } => (2, instance_id),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EndgameFailureReceipt {
    pub attempt: [u8; 32],
    pub encounter: FinalEncounter,
    /// None is a retained no-op: no current eligible final-boss chain. Soul War
    /// expiry dispositions remain explicit in the immutable full-roster outcome.
    /// The old attempt can never consume newly earned flags on a later retry.
    pub removed_flags: Option<u8>,
    pub replayed: bool,
}

#[derive(Debug)]
pub enum EndgameTerminalError {
    InvalidInput,
    ContentUnavailable,
    NotTerminalUnsuccessful,
    AuthorityRejected,
    ConflictingOccurrence,
    Progression(CharacterProgressionError),
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for EndgameTerminalError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<CharacterProgressionError> for EndgameTerminalError {
    fn from(error: CharacterProgressionError) -> Self {
        Self::Progression(error)
    }
}
type Result<T> = std::result::Result<T, EndgameTerminalError>;

impl DurabilityRoot {
    /// Historical replay is read from its record before acquiring current member
    /// authority. A new outcome requires the sealed W02 proof in this exact SQL
    /// transaction, then every current session/lease/scope/node/Character fence.
    pub async fn apply_endgame_failure<O: SharedTerminalOwner>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        shared_owner: &O,
        mut request: EndgameFailureRequest,
    ) -> Result<EndgameFailureReceipt> {
        request.canonicalize()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| EndgameTerminalError::AuthorityRejected)?;
        let mut context = (shared_owner, request, node.clone(), recovery);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let (owner, request, node, recovery) = context;
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let outcome = match storage::consume(&mut tx, node, *owner, request).await {
                        Ok(outcome) => outcome,
                        Err(error) => return Ok(Err(error)),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(outcome))
                })
            })
            .await?
    }
}

/// Accepted owner rules ANSWER6547488765/6547661530. The identity is retained by
/// W02; it never comes from a player, local clock, disconnect or per-member RNG.
fn choose_pair(attempt: &[u8; 32], encounter: FinalEncounter, common: u8) -> Result<Option<u8>> {
    if common & !encounter.mask() != 0 {
        return Err(EndgameTerminalError::InvalidInput);
    }
    if common.count_ones() < 2 || (encounter == FinalEncounter::Megalomania && common != 31) {
        return Ok(None);
    }
    let digest = Sha256::new()
        .chain_update(b"oteryn:endgame-failed-group-pair:v1")
        .chain_update(attempt)
        .chain_update(encounter.code().to_be_bytes())
        .finalize();
    let mut draw = [0; 8];
    draw.copy_from_slice(&digest[..8]);
    let draw = u64::from_be_bytes(draw);
    if encounter == FinalEncounter::Megalomania {
        let index = oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(draw, 0, 9);
        let index = u64::try_from(index).map_err(|_| EndgameTerminalError::InvalidInput)?;
        return Ok(Some(oteryn_gameplay_quests::quest::soul_war::failure_pair(
            index,
        )));
    }
    let mut pairs = Vec::new();
    for left in 0..5 {
        for right in left + 1..5 {
            let pair = (1_u8 << left) | (1_u8 << right);
            if pair & common == pair {
                pairs.push(pair);
            }
        }
    }
    let index = oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
        draw,
        0,
        i64::try_from(pairs.len() - 1).map_err(|_| EndgameTerminalError::InvalidInput)?,
    );
    Ok(Some(
        pairs[usize::try_from(index).map_err(|_| EndgameTerminalError::InvalidInput)?],
    ))
}

#[cfg(test)]
#[path = "endgame_terminal/tests.rs"]
mod tests;
