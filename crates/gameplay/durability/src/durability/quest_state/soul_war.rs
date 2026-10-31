//! Quest-owned Soul War state, command receipts and native creature-death consumers.
//! No client command can supply a killed boss or a final encounter outcome.
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterRevision;
use crate::foundation::CommandRef;
use oteryn_gameplay_quests::quest::soul_war::{SoulWarRefusal, SoulWarTransition};
pub use oteryn_gameplay_quests::quest::soul_war::SoulWarState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod storage;
mod native;
mod reward;
mod terminal;
pub(crate) use terminal::apply_soul_war_failed_final_in_transaction;
pub use native::{apply_soul_war_kill_in_transaction, soul_war_experience_in_transaction};
pub use reward::{PendingSoulWarReward, SOUL_WAR_REWARD_CLAIM, SOUL_WAR_REWARD_PLACEMENT, SOUL_WAR_REWARD_REVISION};
pub(crate) use reward::confirm_soul_war_reward_in_transaction;

type Result<T> = std::result::Result<T, CharacterProgressionError>;
type StoredResult<T> = std::result::Result<T, DurabilityError>;

/// Only the confirmed Flickering Soul dialogue supplies these actions. Entry
/// level is checked by the entry consumer; the donor NPC accepts a task at any level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoulWarCommand {
    Start,
    Cleanse,
    ReportCompletion,
}

impl SoulWarCommand {
    fn transition(self) -> SoulWarTransition {
        match self {
            Self::Start => SoulWarTransition::AcceptTask,
            Self::Cleanse => SoulWarTransition::Cleanse,
            Self::ReportCompletion => SoulWarTransition::ReportCompletion,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulWarCommandOutcome {
    pub state: SoulWarState,
    pub result: std::result::Result<(), SoulWarRefusal>,
    pub character_revision: CharacterRevision,
    pub replayed: bool,
}

fn command_key(fence: &CurrentCharacterGameplayFence, command: CommandRef) -> [u8; 32] {
    Sha256::new()
        .chain_update(b"oteryn:soul-war:command:v1")
        .chain_update(fence.character_id.as_bytes())
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(fence.connection_generation.get().to_be_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .finalize().into()
}

impl DurabilityRoot {
    /// Read the current source state under recovery/session/lease/scope and
    /// CharacterRevision fences. Expiry is projected from the database clock;
    /// reading never creates a new CharacterRevision or extends the taint window.
    pub async fn read_soul_war_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<SoulWarState> {
        let recovery = authority.record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                return Ok(Err(error));
            }
            let now = storage::database_time(&mut tx).await?;
            let state = storage::load(&mut tx, fence.character_id).await?.expired(now);
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(state))
        })).await?
    }

    /// Read-only reconciliation after an unknown NPC-command outcome. Absence
    /// authorizes no fresh execution; the caller revalidates the NPC separately.
    pub async fn reconcile_soul_war_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterGameplayFence,
        command_ref: CommandRef,
        command: SoulWarCommand,
    ) -> Result<Option<SoulWarCommandOutcome>> {
        if command_ref.game_session_id() != fence.game_session_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let recovery = authority.record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let key = command_key(&fence, command_ref);
        let binding = storage::binding(command.transition())?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            storage::lock_occurrence(&mut tx, fence.character_id, &key).await?;
            let found = storage::replay(&mut tx, fence.character_id, &key, &binding).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(found)
        })).await?
    }

    /// The native dialogue handler owns the NPC reachability/identity check and
    /// Character revision slot. New writes recheck every gameplay fence in this
    /// transaction; replay returns the original receipt without repeating grants.
    pub async fn commit_soul_war_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command_ref: CommandRef,
        command: SoulWarCommand,
    ) -> Result<SoulWarCommandOutcome> {
        if command_ref.game_session_id() != fence.game_session_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let recovery = authority.record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let key = command_key(&fence, command_ref);
        let transition = command.transition();
        let binding = storage::binding(transition)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            storage::lock_occurrence(&mut tx, fence.character_id, &key).await?;
            match storage::replay(&mut tx, fence.character_id, &key, &binding).await? {
                Ok(Some(outcome)) => {
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(Ok(outcome));
                }
                Err(error) => return Ok(Err(error)),
                Ok(None) => {}
            }
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                Ok(root) => root,
                Err(error) => return Ok(Err(error)),
            };
            let outcome = storage::commit_locked(
                &mut tx, fence.character_id, &root, &key, transition, None, None,
            ).await?;
            if outcome.result.is_ok() && command == SoulWarCommand::ReportCompletion {
                super::super::character_appearance::grant_quest_outfit(
                    &mut tx, fence.character_id, &key,
                    super::super::character_appearance::QuestOutfitGrant::Revenant,
                ).await?;
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(outcome))
        })).await?
    }
}
