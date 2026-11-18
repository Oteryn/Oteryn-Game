//! Barbarian Test completion at the arena ingress. The match owner still owns the
//! rank, fee, opponent and pit lease. Crystal action 25100 identifies the StepIn.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn barbarian_test_complete(
        &self,
        fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    ) -> bool {
        self.root
            .read_character_quest_state(self.character, fence.character_id)
            .await
            .is_ok_and(|state| {
                state
                    .tracks()
                    .get(oteryn_gameplay_quests::quest::barbarian_mead::TRACKS[0])
                    .is_some_and(|stage| *stage >= 8)
                    || state
                        .states()
                        .get("oteryn:quest.barbarian_test")
                        .is_some_and(|quest| quest.completed)
            })
    }

    pub(in crate::gameplay_transport) async fn gate_arena_entrance_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: crate::foundation::MovementPositionSnapshot,
        entered: crate::foundation::MovementPositionSnapshot,
    ) -> Option<crate::foundation::MovementPositionSnapshot> {
        use crate::map::{
            overlay::TilePos,
            view::{MapFacts, placement_key},
        };
        let at = entered.position();
        if origin.position() == at || !at_entrance(at) {
            return None;
        }
        let map = self.bundle_map()?;
        let pos = TilePos {
            x: at.x.try_into().ok()?,
            y: at.y.try_into().ok()?,
            floor: i8::try_from(at.floor).ok()?.checked_neg()?,
        };
        let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
        let ordinals = tile.depths().iter().filter(|depth| **depth == 0).count();
        if !(0..ordinals)
            .filter_map(|ordinal| placement_key(pos, u8::try_from(ordinal).ok()?))
            .any(|key| !map.facts.base_hidden(key) && map.facts.action_at(key) == Some(25100))
        {
            return None;
        }
        if let Ok(Some(fence)) = self.current_quest_fence(session).await
            && self.barbarian_test_complete(fence).await
        {
            return None;
        }
        let mut runtime = self.runtime.lock().await;
        runtime
            .relocate_arena_player(actor, session, entered, origin.position())
            .ok()?;
        self.states
            .chat
            .push_quest_feedback(session, "Sorry, not possible.");
        runtime.read_actor_position(actor).ok()
    }
}
