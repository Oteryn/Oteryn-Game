//! Immediate and admitted arrival recovery share the current-world citizenship owner.
use super::*;
use crate::durability::character_home_town::{HomeTownRelocation, HomeTownRelocationCause};
use crate::durability::character_last_position::FinalLastPosition;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::npc_travel::{ArrivalConsume, TravelOccurrence};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn consume_travel_arrival(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        occurrence: TravelOccurrence,
        placed: FinalLastPosition,
    ) -> Option<(ArrivalConsume, CurrentCharacterGameplayFence)> {
        // Never replace the capture's original session/lease/scope with a newer fence.
        let town = self
            .root
            .npc_pending_arrival_home_town(self.character, self.holder, fence, occurrence)
            .await
            .ok()?;
        let consumed = if let Some(town) = town {
            let relocation = HomeTownRelocation {
                cause: HomeTownRelocationCause::Arrival,
                placed,
                respawn: None,
                arrival: Some(occurrence),
            };
            if self
                .commit_home_town_at_fence(
                    session,
                    Some(actor),
                    (Some(town), Some(relocation)),
                    self,
                    fence,
                )
                .await
                != Some(town)
            {
                return None;
            }
            ArrivalConsume::Consumed
        } else {
            self.root
                .consume_npc_pending_arrival_placed(
                    self.character,
                    self.holder,
                    fence,
                    occurrence,
                    placed,
                )
                .await
                .ok()?
        };
        // Only the CharacterRevision may have advanced. A NoOp keeps the existing caller's
        // cache behavior; it must not retag a stale immediate capture as a fresh write.
        let current = self.current_quest_fence(session).await.ok()??;
        (current.item_fence() == fence.item_fence()).then_some((consumed, current))
    }
}
