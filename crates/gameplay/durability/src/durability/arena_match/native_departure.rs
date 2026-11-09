//! M51 cancellation after an actual native relocation out of the owned pit.
//! Crystal201189f movements_arena_pit.lua:14-20 and
//! data-global/lib/quests/svargrond_arena.lua:259-287.
use super::*;
use crate::foundation::{
    ExactActorRef, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};

#[derive(Clone, Debug)]
pub struct NativeArenaDeparture {
    actor: ExactActorRef,
    command: CommandRef,
    run: [u8; 16],
    pit: u8,
    binding: Vec<u8>,
}

impl NativeArenaDeparture {
    /// The caller retains the spell lane and compares the actual actor snapshot
    /// under the runtime guard before writing. This is never client Leave ingress.
    pub fn new(
        actor: ExactActorRef,
        session: GameSessionId,
        at: MovementPositionSnapshot,
        epoch: u64,
        run: [u8; 16],
        pit: u8,
    ) -> Option<Self> {
        if actor.channel_id().is_none()
            || at.world_id() != actor.world_id()
            || !outside_pit(pit, at.position())?
        {
            return None;
        }
        let mut binding = b"oteryn:svargrond-native-departure:v1".to_vec();
        binding.extend_from_slice(&actor.placement_identity());
        binding.extend_from_slice(session.as_bytes());
        binding.extend_from_slice(&run);
        binding.push(pit);
        binding.extend_from_slice(&epoch.to_be_bytes());
        binding.extend_from_slice(&actor.scope_generation().get().to_be_bytes());
        binding.extend_from_slice(&at.revision().to_be_bytes());
        binding.extend_from_slice(&at.last_translation_revision().to_be_bytes());
        binding.extend_from_slice(&at.position().x.to_be_bytes());
        binding.extend_from_slice(&at.position().y.to_be_bytes());
        binding.extend_from_slice(&at.position().floor.to_be_bytes());
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
    pub(super) fn command(&self) -> CommandRef {
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
        matches!(before.phase, 1 | 2)
            && (1..=3).contains(&before.rank)
            && before.run == self.run
            && before.pit == self.pit
            && before.session == *fence.game_session_id.as_bytes()
            && before.scope_generation == fence.scope_ownership_generation.get()
            && self.command.game_session_id() == fence.game_session_id
            && self.actor.runtime_scope() == fence.runtime_scope
            && self.actor.scope_generation() == fence.scope_ownership_generation
    }
}

fn outside_pit(pit: u8, at: MovementLocalPosition) -> Option<bool> {
    let center = oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(pit)?
        .layout()
        .center;
    // Keep the same live-owner envelope as the dispatcher, including its portal.
    Some(
        i32::from(at.floor) != -center[2]
            || at.x.abs_diff(center[0]) > 6
            || at.y.abs_diff(center[1]) > 6,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bestiary_postgres_harness as h;
    #[test]
    fn m51_native_departure_cannot_cancel_a_completed_or_different_run() -> h::TestResult {
        let fence = h::fence(1)?;
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err("channel fixture".into());
        };
        let cause = NativeArenaDeparture {
            actor: ExactActorRef::transport_fixture(world_id, channel_id),
            command: CommandRef::new(
                fence.game_session_id,
                crate::foundation::CommandId::new(93).map_err(h::debug)?,
            ),
            run: h::id(230),
            pit: 1,
            binding: b"native-departure".to_vec(),
        };
        let state = MatchState {
            rank: 1,
            pit: 1,
            phase: 1,
            run: cause.run,
            session: *fence.game_session_id.as_bytes(),
            scope_generation: fence.scope_ownership_generation.get(),
            ..MatchState::default()
        };
        for phase in [1, 2] {
            let mut state = state.clone();
            state.phase = phase;
            assert!(cause.validates(&state, fence));
        }
        for phase in [0, 3] {
            let mut state = state.clone();
            state.phase = phase;
            assert!(!cause.validates(&state, fence));
        }
        let mut changed = state.clone();
        changed.run = h::id(231);
        assert!(!cause.validates(&changed, fence));
        changed = state.clone();
        changed.pit = 2;
        assert!(!cause.validates(&changed, fence));
        changed = state.clone();
        changed.scope_generation += 1;
        assert!(!cause.validates(&changed, fence));
        let origin = NativeArenaOrigin::Departure(cause);
        let payload = origin.payload().ok_or("departure payload")?;
        assert!(origin.validates(&state, fence, origin.command(), &payload, None));
        let advance = oteryn_protocol_oteryn::arena::encode_match_request(
            oteryn_protocol_oteryn::arena::ArenaAction::Advance,
        )
        .map_err(h::debug)?;
        assert!(!origin.validates(&state, fence, origin.command(), &advance, None));
        assert!(
            !origin.requires_live_pit(),
            "cleanup must still work after expiry"
        );
        for pit in 1..=10 {
            let c = oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(pit)
                .ok_or("pit")?
                .layout()
                .center;
            let mut at = MovementLocalPosition {
                x: c[0],
                y: c[1],
                floor: 7,
            };
            assert_eq!(outside_pit(pit, at), Some(false));
            at.x += 6;
            assert_eq!(outside_pit(pit, at), Some(false));
            at.x += 1;
            assert_eq!(outside_pit(pit, at), Some(true));
            at.x = c[0];
            at.floor = 6;
            assert_eq!(outside_pit(pit, at), Some(true));
        }
        assert_eq!(
            outside_pit(
                0,
                MovementLocalPosition {
                    x: 0,
                    y: 0,
                    floor: 0
                }
            ),
            None
        );
        Ok(())
    }
}
