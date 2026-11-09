//! M51 source death occurrence. Crystal201189f
//! data-global/scripts/quests/svargrond_arena/creaturescripts_arena_kill.lua:2-38.
use super::*;
use crate::foundation::{CreatureDeathOccurrenceKey, ExactActorRef, GameSessionId};

/// A real native creature death, captured only after the runtime owner proves
/// the current opponent and most-damage Character. Current custody is separate.
#[derive(Clone, Debug)]
pub struct NativeArenaDeath {
    actor: ExactActorRef,
    command: CommandRef,
    run: [u8; 16],
    pit: u8,
    death: CreatureDeathOccurrenceKey,
    binding: Vec<u8>,
}

impl NativeArenaDeath {
    pub fn new(
        actor: ExactActorRef,
        session: GameSessionId,
        death: CreatureDeathOccurrenceKey,
        epoch: u64,
        run: [u8; 16],
        pit: u8,
    ) -> Option<Self> {
        if actor.channel_id().is_none()
            || actor.runtime_scope() != death.runtime_scope()
            || actor.scope_generation() != death.scope_ownership_generation()
            || oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(pit).is_none()
        {
            return None;
        }
        let mut binding = b"oteryn:svargrond-native-death:v1".to_vec();
        binding.extend_from_slice(&actor.placement_identity());
        binding.extend_from_slice(session.as_bytes());
        binding.extend_from_slice(&run);
        binding.push(pit);
        binding.extend_from_slice(&epoch.to_be_bytes());
        binding.extend_from_slice(&death.native_bytes());
        let hash = Sha256::digest(&binding);
        let mut compact = [0; 8];
        compact.copy_from_slice(&hash[..8]);
        let id = crate::foundation::CommandId::new(u64::from_be_bytes(compact).max(1)).ok()?;
        Some(Self {
            actor,
            command: CommandRef::new(session, id),
            run,
            pit,
            death,
            binding,
        })
    }

    pub(super) fn command(&self) -> CommandRef {
        self.command
    }
    pub(super) fn binding(&self) -> &[u8] {
        &self.binding
    }
    pub(super) fn death(&self) -> CreatureDeathOccurrenceKey {
        self.death
    }
    pub(super) fn validates(
        &self,
        before: &MatchState,
        fence: CurrentCharacterGameplayFence,
    ) -> bool {
        before.phase == 1
            && before.run == self.run
            && before.pit == self.pit
            && (1..=3).contains(&before.rank)
            && before.session == *fence.game_session_id.as_bytes()
            && before.scope_generation == fence.scope_ownership_generation.get()
            && self.command.game_session_id() == fence.game_session_id
            && self.actor.runtime_scope() == fence.runtime_scope
            && self.actor.scope_generation() == fence.scope_ownership_generation
            && self.death.runtime_scope() == fence.runtime_scope
            && self.death.scope_ownership_generation() == fence.scope_ownership_generation
    }
}
