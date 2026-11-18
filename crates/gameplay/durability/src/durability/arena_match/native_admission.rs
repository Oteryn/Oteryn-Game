//! Crystal 201189f8 data-global/npc/halvar.lua:99-107 and
//! movements_arena_enter.lua:3-35 separate payment from placement.
use super::*;
use crate::foundation::{ExactActorRef, GameSessionId, MovementPositionSnapshot};

#[derive(Clone, Debug)]
pub struct NativeArenaAdmission {
    actor: ExactActorRef,
    command: CommandRef,
    binding: Vec<u8>,
    payment: bool,
}

impl NativeArenaAdmission {
    pub fn payment(actor: ExactActorRef, command: CommandRef) -> Option<Self> {
        actor.channel_id()?;
        let mut binding = b"oteryn:svargrond-halvar-payment:v1".to_vec();
        binding.extend_from_slice(&actor.placement_identity());
        binding.extend_from_slice(&actor.scope_generation().get().to_be_bytes());
        Some(Self {
            actor,
            command,
            binding,
            payment: true,
        })
    }

    pub fn entry(
        actor: ExactActorRef,
        session: GameSessionId,
        step: MovementPositionSnapshot,
        placement: u64,
        reset_epoch: u64,
    ) -> Option<Self> {
        actor.channel_id()?;
        if step.world_id() != actor.world_id()
            || step.revision() == 0
            || step.revision() != step.last_translation_revision()
        {
            return None;
        }
        let mut binding = b"oteryn:svargrond-halvar-entry:v1".to_vec();
        binding.extend_from_slice(session.as_bytes());
        binding.extend_from_slice(&actor.placement_identity());
        binding.extend_from_slice(&actor.scope_generation().get().to_be_bytes());
        binding.extend_from_slice(&reset_epoch.to_be_bytes());
        binding.extend_from_slice(&step.revision().to_be_bytes());
        binding.extend_from_slice(&placement.to_be_bytes());
        let digest = Sha256::digest(&binding);
        let id = u64::from_be_bytes(digest[..8].try_into().ok()?).max(1);
        let command = CommandRef::new(session, crate::foundation::CommandId::new(id).ok()?);
        Some(Self {
            actor,
            command,
            binding,
            payment: false,
        })
    }

    pub fn command(&self) -> CommandRef {
        self.command
    }

    pub(super) fn binding(&self) -> &[u8] {
        &self.binding
    }

    pub fn is_payment(&self) -> bool {
        self.payment
    }

    pub(super) fn validates(
        &self,
        before: &MatchState,
        fence: CurrentCharacterGameplayFence,
    ) -> bool {
        let ready = if self.payment {
            before.paid_rank == 0
        } else {
            before.paid_rank == before.completed + 1
        };
        before.phase == 0
            && before.rank == 0
            && before.completed < 3
            && ready
            && self.command.game_session_id() == fence.game_session_id
            && self.actor.runtime_scope() == fence.runtime_scope
            && self.actor.scope_generation() == fence.scope_ownership_generation
    }
}
