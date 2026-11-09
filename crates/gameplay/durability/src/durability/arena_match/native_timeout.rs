//! M51 original kick timer. Crystal201189f
//! data-global/lib/quests/svargrond_arena.lua:259-313.
use super::*;
use crate::foundation::{ExactActorRef, GameSessionId};

#[derive(Clone, Debug)]
pub struct NativeArenaTimeout {
    actor: ExactActorRef,
    command: CommandRef,
    run: [u8; 16],
    pit: u8,
    deadline_ms: i64,
    binding: Vec<u8>,
}
impl NativeArenaTimeout {
    /// The current fenced writer, not the callback's clock, decides expiry.
    pub fn new(
        actor: ExactActorRef,
        session: GameSessionId,
        epoch: u64,
        state: &MatchState,
    ) -> Option<Self> {
        if actor.channel_id().is_none()
            || !matches!(state.phase, 1 | 2)
            || !(1..=3).contains(&state.rank)
            || state.deadline_ms <= 0
            || oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(state.pit).is_none()
            || state.session != *session.as_bytes()
            || state.scope_generation != actor.scope_generation().get()
        {
            return None;
        }
        let mut binding = b"oteryn:svargrond-native-timeout:v1".to_vec();
        binding.extend_from_slice(&actor.placement_identity());
        binding.extend_from_slice(session.as_bytes());
        binding.extend_from_slice(&state.run);
        binding.push(state.pit);
        binding.extend_from_slice(&epoch.to_be_bytes());
        binding.extend_from_slice(&state.scope_generation.to_be_bytes());
        binding.extend_from_slice(&state.deadline_ms.to_be_bytes());
        let hash = Sha256::digest(&binding);
        let mut compact = [0; 8];
        compact.copy_from_slice(&hash[..8]);
        let id = crate::foundation::CommandId::new(u64::from_be_bytes(compact).max(1)).ok()?;
        Some(Self {
            actor,
            command: CommandRef::new(session, id),
            run: state.run,
            pit: state.pit,
            deadline_ms: state.deadline_ms,
            binding,
        })
    }
    pub(super) fn command(&self) -> CommandRef {
        self.command
    }
    pub(super) fn binding(&self) -> &[u8] {
        &self.binding
    }
    pub(super) fn due_at(&self, now_ms: i64) -> bool {
        now_ms >= self.deadline_ms
    }
    pub(super) fn validates(
        &self,
        before: &MatchState,
        fence: CurrentCharacterGameplayFence,
    ) -> bool {
        matches!(before.phase, 1 | 2)
            && (1..=3).contains(&before.rank)
            && before.run == self.run
            && before.pit == self.pit
            && before.deadline_ms == self.deadline_ms
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
    fn m51_native_timeout_binds_the_original_deadline_and_active_run() -> h::TestResult {
        let fence = h::fence(1)?;
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err("channel fixture".into());
        };
        let actor = ExactActorRef::transport_fixture(world_id, channel_id);
        let state = MatchState {
            rank: 1,
            pit: 1,
            phase: 1,
            run: h::id(230),
            deadline_ms: 2000,
            session: *fence.game_session_id.as_bytes(),
            scope_generation: fence.scope_ownership_generation.get(),
            ..MatchState::default()
        };
        let cause =
            NativeArenaTimeout::new(actor, fence.game_session_id, 1, &state).ok_or("timeout")?;
        assert!(cause.validates(&state, fence));
        assert!(!cause.due_at(1999));
        assert!(cause.due_at(2000));
        assert!(cause.due_at(2001));
        for phase in [0, 3] {
            let mut s = state.clone();
            s.phase = phase;
            assert!(NativeArenaTimeout::new(actor, fence.game_session_id, 1, &s).is_none());
        }
        let mut changed = state.clone();
        changed.deadline_ms = 0;
        assert!(NativeArenaTimeout::new(actor, fence.game_session_id, 1, &changed).is_none());
        changed = state.clone();
        changed.scope_generation += 1;
        assert!(NativeArenaTimeout::new(actor, fence.game_session_id, 1, &changed).is_none());
        changed = state.clone();
        changed.session = h::id(51);
        assert!(NativeArenaTimeout::new(actor, fence.game_session_id, 1, &changed).is_none());
        changed = state.clone();
        changed.deadline_ms += 1;
        assert!(!cause.validates(&changed, fence));
        changed = state.clone();
        changed.run = h::id(231);
        assert!(!cause.validates(&changed, fence));
        Ok(())
    }
}
