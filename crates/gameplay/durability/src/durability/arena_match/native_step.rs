//! M51 native movement origin, not a decoded client command. Crystal201189f
//! data-global/scripts/quests/svargrond_arena/movements_arena_pit.lua:52-64.
use super::*;
use crate::foundation::{ExactActorRef, GameSessionId, MovementPositionSnapshot};

/// Constructed from an actual committed movement snapshot. The caller must retain
/// the spell lane and compare the exact actor/snapshot under the runtime guard.
#[derive(Clone, Debug)]
pub struct NativeArenaStep {
    actor: ExactActorRef,
    command: CommandRef,
    run: [u8; 16],
    pit: u8,
    binding: Vec<u8>,
}

impl NativeArenaStep {
    pub fn new(
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
        epoch: u64,
        run: [u8; 16],
        pit: u8,
    ) -> Option<Self> {
        let layout = oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(pit)?.layout();
        let at = entered.position();
        if actor.channel_id().is_none()
            || entered.last_translation_revision() == 0
            || entered.revision() != entered.last_translation_revision()
            || entered.world_id() != actor.world_id()
            || [at.x, at.y, -i32::from(at.floor)] != layout.pillar
        {
            return None;
        }
        let mut binding = b"oteryn:svargrond-native-step:v1".to_vec();
        binding.extend_from_slice(&actor.placement_identity());
        binding.extend_from_slice(session.as_bytes());
        binding.extend_from_slice(&run);
        binding.push(pit);
        binding.extend_from_slice(&epoch.to_be_bytes());
        binding.extend_from_slice(&entered.revision().to_be_bytes());
        binding.extend_from_slice(&entered.last_translation_revision().to_be_bytes());
        binding.extend_from_slice(&actor.scope_generation().get().to_be_bytes());
        binding.extend_from_slice(&at.x.to_be_bytes());
        binding.extend_from_slice(&at.y.to_be_bytes());
        binding.extend_from_slice(&at.floor.to_be_bytes());
        let hash = Sha256::digest(&binding);
        let mut compact = [0; 8];
        compact.copy_from_slice(&hash[..8]);
        let id = crate::foundation::CommandId::new(u64::from_be_bytes(compact).max(1)).ok()?;
        Some(Self {
            actor,
            command: CommandRef::new(session, id),
            run,
            pit,
            binding,
        })
    }

    pub fn command(&self) -> CommandRef {
        self.command
    }

    pub(super) fn binding(&self) -> &[u8] {
        &self.binding
    }

    pub(super) fn validates(
        &self,
        before: &MatchState,
        fence: CurrentCharacterGameplayFence,
    ) -> bool {
        before.phase == 2
            && before.run == self.run
            && before.pit == self.pit
            && before.session == *fence.game_session_id.as_bytes()
            && before.scope_generation == fence.scope_ownership_generation.get()
            && self.command.game_session_id() == fence.game_session_id
            && self.actor.runtime_scope() == fence.runtime_scope
            && self.actor.scope_generation() == fence.scope_ownership_generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bestiary_postgres_harness as h;

    #[test]
    fn m51_native_step_requires_credited_matching_run_and_current_scope() -> h::TestResult {
        let fence = h::fence(1)?;
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err("channel fixture".into());
        };
        let actor = ExactActorRef::transport_fixture(world_id, channel_id);
        let command = CommandRef::new(
            fence.game_session_id,
            crate::foundation::CommandId::new(91).map_err(h::debug)?,
        );
        let step = NativeArenaStep {
            actor,
            command,
            run: h::id(232),
            pit: 1,
            binding: b"native-step".to_vec(),
        };
        let before = MatchState {
            phase: 2,
            pit: 1,
            run: h::id(232),
            session: *fence.game_session_id.as_bytes(),
            scope_generation: fence.scope_ownership_generation.get(),
            ..MatchState::default()
        };
        assert!(step.validates(&before, fence));
        let mut changed = before.clone();
        changed.phase = 1;
        assert!(
            !step.validates(&changed, fence),
            "no uncredited advancement"
        );
        changed = before.clone();
        changed.run = h::id(233);
        assert!(
            !step.validates(&changed, fence),
            "another run cannot reuse a STEP"
        );
        changed = before.clone();
        changed.pit = 2;
        assert!(!step.validates(&changed, fence));
        changed = before.clone();
        changed.scope_generation += 1;
        assert!(!step.validates(&changed, fence));
        Ok(())
    }
}
