//! Survivor progress uses the sole Character Quest writer and its movement receipt key.
use super::*;
use crate::foundation::{CharacterLease, ExactActorRef, ScopeOwnershipGeneration};
use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffectKind, QuestTransition};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, waves::WaveRun};

#[derive(Clone, Debug)]
pub struct WaveEgress {
    callback: String,
    transition: String,
    actor: ExactActorRef,
    command: CommandRef,
    victory: bool,
    lease: CharacterLease,
    owner_generation: ScopeOwnershipGeneration,
    movement_revision: u64,
    reset_epoch: u64,
}

impl WaveEgress {
    /// The Channel calls this for a committed forced movement only.
    #[allow(
        clippy::too_many_arguments,
        reason = "complete exact movement and lease witness"
    )]
    pub fn from_committed_movement(
        callback: String,
        transition: String,
        actor: ExactActorRef,
        command: CommandRef,
        victory: bool,
        lease: CharacterLease,
        owner_generation: ScopeOwnershipGeneration,
        movement_revision: u64,
        reset_epoch: u64,
    ) -> Self {
        Self {
            callback,
            transition,
            actor,
            command,
            victory,
            lease,
            owner_generation,
            movement_revision,
            reset_epoch,
        }
    }

    pub fn command(&self) -> CommandRef {
        self.command
    }

    pub(super) fn request(
        &self,
        fence: &CurrentCharacterGameplayFence,
    ) -> Result<QuestTransitionRequest> {
        if self.actor.runtime_scope() != fence.runtime_scope
            || self.command.game_session_id() != fence.game_session_id
            || self.owner_generation != fence.scope_ownership_generation
            || self.lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || !self
                .lease
                .accepts_generation(fence.character_lease_generation)
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        Ok(QuestTransitionRequest {
            transition_key: self.transition.clone(),
            cause: QuestCause::Step(self.command),
        })
    }

    pub(super) fn matching_run<'a>(
        &self,
        catalogue: &QuestStateCatalogue,
        callbacks: &'a [Callback],
        request: &QuestTransitionRequest,
    ) -> Option<&'a WaveRun> {
        callbacks.iter().find_map(|callback| {
            let run = callback.effects.encounter.as_ref()?.wave_run.as_ref()?;
            let credit = run.credit(self.victory)?;
            (callback.key == self.callback
                && credit.key() == request.transition_key
                && catalogue
                    .transition(credit.key())
                    .is_some_and(|transition| credit.matches_transition(&callback.quest, transition)))
                .then_some(run)
        })
    }

    pub(super) fn achievement<'a>(&self, run: &'a WaveRun) -> Option<&'a str> {
        self.victory.then_some(())?;
        run.victory.as_ref()?.achievement.as_deref()
    }

    pub(super) fn binding(
        &self,
        character: CharacterId,
        request: &QuestTransitionRequest,
    ) -> [u8; 33] {
        let mut binding = request_binding(character, request);
        let mut hash = Sha256::new();
        hash.update(binding);
        hash.update(b"oteryn:quest-wave-egress:v1");
        hash.update(self.actor.placement_identity());
        hash.update([u8::from(self.victory)]);
        hash.update(self.movement_revision.to_be_bytes());
        hash.update(self.reset_epoch.to_be_bytes());
        hash.update(self.lease.generation().to_be_bytes());
        hash.update(self.callback.as_bytes());
        binding[1..].copy_from_slice(&hash.finalize());
        binding
    }
}

/// Use the same receipt for a winner whose guarded progress does not advance.
pub(super) fn without_progress(transition: &QuestTransition) -> QuestTransition {
    let mut unchanged = transition.clone();
    for effect in &mut unchanged.effects {
        effect.from = QuestComparison::Any;
        effect.effect = QuestEffectKind::Add(0);
    }
    unchanged
}

impl DurabilityRoot {
    pub async fn commit_quest_wave_egress(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: WaveEgress,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = input.request(&fence)?;
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(super::QuestSourceAction::WaveEgress(input)),
        )
        .await
    }
}
