use super::*;
use sha2::{Digest, Sha256};
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Source tile.lua Cake isle and five extraction pads, reached only after an
    /// ordinary admitted player step. The durable owner deduplicates this exact step.
    pub(in crate::gameplay_transport) async fn cake_character_step(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        entered: crate::foundation::MovementPositionSnapshot,
    ) {
        use crate::durability::seasonal_cake::character::CakeCharacterEvent;
        if self
            .states
            .cake_stage
            .load(std::sync::atomic::Ordering::Relaxed)
            == 0
        {
            return;
        }
        let Some(map) = self.bundle_map() else {
            return;
        };
        let at = entered.position();
        let (Ok(x), Ok(y), Ok(floor)) = (
            u16::try_from(at.x),
            u16::try_from(at.y),
            i8::try_from(-at.floor),
        ) else {
            return;
        };
        let event = {
            let runtime = self.runtime.lock().await;
            if runtime.read_actor_position(actor) != Ok(entered)
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return;
            }
            let Ok(Some(tile)) = crate::map::view::compose_tile(
                &map.overlay,
                &map.facts.pinned(),
                TilePos { x, y, floor },
            ) else {
                return;
            };
            if tile
                .entries
                .iter()
                .any(|entry| (12090..=12095).contains(&entry.facts.appearance_id))
            {
                CakeCharacterEvent::Isle
            } else if let Some(city) = machine_city(at)
                && tile
                    .entries
                    .iter()
                    .any(|entry| entry.facts.appearance_id == 8886)
            {
                CakeCharacterEvent::Extract(city)
            } else {
                return;
            }
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return;
        };
        let _slot = self.revision_sequencer.acquire(fence.character_id).await;
        if self.runtime.lock().await.read_actor_position(actor) != Ok(entered) {
            return;
        }
        let occurrence = Sha256::new()
            .chain_update(b"oteryn:cake-character-step:v1")
            .chain_update(actor.placement_identity())
            .chain_update(session.as_bytes())
            .chain_update(self.reset_epoch.to_be_bytes())
            .chain_update(entered.revision().to_be_bytes())
            .finalize()
            .into();
        match self
            .root
            .commit_cake_character_event(
                self.character,
                self.holder,
                fence.item_fence(),
                occurrence,
                event,
                now_seconds(),
            )
            .await
        {
            Ok(Some(progress)) => {
                use crate::durability::seasonal_cake::character::CakeCharacterEffect as Effect;
                let message = match progress.effect {
                    Effect::Armed => {
                        "You feel ready to head to the next extraction machine.".into()
                    }
                    Effect::NoMark => "Go step on the Cake Base at the isle first.".into(),
                    Effect::AlreadyVisited => {
                        "You already visited this machine for Brutus' errand.".into()
                    }
                    Effect::Extracted(city) => {
                        let name = match city {
                            CakeCity::Carlin => "Carlin",
                            CakeCity::Thais => "Thais",
                            CakeCity::Edron => "Edron",
                            CakeCity::Darashia => "Darashia",
                            CakeCity::LibertyBay => "Liberty Bay",
                        };
                        format!(
                            "{name} machine visited. Head back to the Cake Base isle. ({}/5)",
                            progress.visited_count()
                        )
                    }
                    Effect::Complete => {
                        "You have visited all five machines. Return to Brutus for your reward."
                            .into()
                    }
                    Effect::Status | Effect::Started => return,
                };
                self.states.chat.push_quest_feedback(session, &message);
                if matches!(progress.effect, Effect::Armed | Effect::Extracted(_)) {
                    // Crystal utils/utils_definitions.hpp:107 CONST_ME_CAKE=52.
                    let _ = map.facts.emit_movement_effect(TilePos { x, y, floor }, 52);
                }
            }
            Ok(None) => {}
            Err(_) => super::super::operator_event("cake_character_step_transaction_rolled_back"),
        }
    }
}
