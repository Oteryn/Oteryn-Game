//! Full weekly/bounty Task Board. All actions share one fenced durable owner.
use std::sync::Arc;

use super::{
    ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession,
    operator_event,
};
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceRewardOccurrence,
};
use crate::durability::quest_state::QuestCause;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::task_board::{TaskBoardCatalogue, TaskBoardCommand, TaskBoardCommit};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::task_board::full::TaskBoardAction;
use oteryn_protocol_oteryn::task_board as wire;

#[path = "bounty_talisman.rs"]
pub(super) mod bounty_talisman;
#[path = "task_board_projection.rs"]
mod projection;

#[derive(Debug)]
pub enum TaskBoardReply {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
}

/// Preserve the bundle-world rollout gate from the capability fragment.
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|capability| available || capability.id != wire::CAPABILITY_TASK_BOARD_V1)
        .collect()
}

fn decode_action(payload: &[u8]) -> Option<(TaskBoardAction, bool)> {
    if let Ok(request) = wire::full::decode_request(payload) {
        let slot = u8::try_from(request.selection).ok()?;
        let action = match request.action {
            4 => TaskBoardAction::Query,
            5 => TaskBoardAction::StartWeekly { difficulty: slot },
            6 => TaskBoardAction::Deliver { slot },
            7 => TaskBoardAction::OpenBounty,
            8 => TaskBoardAction::SelectBounty { slot },
            9 => TaskBoardAction::ClaimBounty,
            10 => TaskBoardAction::ChangeBountyDifficulty { difficulty: slot },
            11 => TaskBoardAction::RerollBounty,
            12 => TaskBoardAction::ClaimDailyReroll,
            13 => TaskBoardAction::SetPreferred {
                slot,
                race: (!request.race.is_empty()).then_some(request.race),
            },
            14 => TaskBoardAction::SetUnwanted {
                slot,
                race: (!request.race.is_empty()).then_some(request.race),
            },
            15 => TaskBoardAction::UnlockList,
            16 => TaskBoardAction::BuyShop {
                offer: request.selection,
            },
            17 => TaskBoardAction::BuyTalisman,
            18 => TaskBoardAction::UpgradeTalisman { path: slot },
            _ => return None,
        };
        return Some((action, false));
    }
    let request = wire::decode_task_board_request(payload).ok()?;
    let action = match request.action {
        0 => TaskBoardAction::Query,
        1 | 2 => TaskBoardAction::RefuseLegacy {
            action: request.action,
            task: request.task,
        },
        3 => TaskBoardAction::UpgradeTalisman {
            path: request.task.checked_sub(1)?,
        },
        _ => return None,
    };
    Some((action, true))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn task_board_catalogue(&self) -> Option<TaskBoardCatalogue> {
        let mut catalogue =
            TaskBoardCatalogue::try_from(self.active_generation?.native_gameplay()?).ok()?;
        catalogue.gameplay_content_revision =
            self.quest_catalogue.as_ref()?.content_revision().to_owned();
        Some(catalogue)
    }

    pub(super) async fn task_board_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> TaskBoardReply {
        let Some((action, legacy)) = decode_action(payload) else {
            return TaskBoardReply::Rejected;
        };
        if kind != wire::COMMAND_TYPE_TASK_BOARD {
            return TaskBoardReply::Rejected;
        }
        let Some(item_fence) = admitted.item_fence else {
            return TaskBoardReply::Rejected;
        };
        let Ok(command) = CommandId::new(id) else {
            return TaskBoardReply::Rejected;
        };
        if self.quest_catalogue.is_none() || self.task_board_catalogue().is_none() {
            return TaskBoardReply::Rejected;
        }
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_TASK_BOARD_V1)
                || item_fence.game_session_id != admitted.game_session_id
                || item_fence.connection_generation.get()
                    != admitted.continuity.connection_generation
                || admitted.world_id != binding.world_id()
                || Some(admitted.channel_id) != binding.channel_id()
                || item_fence.runtime_scope != binding.runtime_scope()
                || item_fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !admitted.runtime_actor.is_some_and(|actor| {
                    runtime
                        .player_control_facts(actor, admitted.game_session_id)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                })
            {
                return TaskBoardReply::Rejected;
            }
        }
        // Every contributor and shared-XP member can own task credit from another
        // principal's queue. Drain it before changing the board, then retain the
        // lane only after checking that no intervening death remains unresolved.
        let Some(permit) = self.task_board_lane_after_kills().await else {
            return TaskBoardReply::Unavailable;
        };
        let Ok(Some(fence)) = self.current_quest_fence(admitted.game_session_id).await else {
            return TaskBoardReply::Rejected;
        };
        if fence.character_id.as_bytes() != item_fence.character_id.as_bytes()
            || fence.connection_generation != item_fence.connection_generation
            || fence.character_lease_generation != item_fence.character_lease_generation
            || fence.runtime_scope != item_fence.runtime_scope
            || fence.scope_ownership_generation != item_fence.scope_ownership_generation
        {
            return TaskBoardReply::Rejected;
        }
        let fee_change = if action == TaskBoardAction::BuyTalisman {
            let backpack = match self
                .root
                .read_character_backpack(self.character, fence.character_id)
                .await
            {
                Ok(backpack) => backpack,
                Err(_) => return TaskBoardReply::Unavailable,
            };
            let runtime = self.runtime.lock().await;
            super::charm_progression::fee_change(
                self,
                &runtime,
                backpack.as_ref(),
                [
                    super::npc_trade_command::derived(
                        b"taskboard:change:platinum",
                        admitted.game_session_id,
                        id,
                    ),
                    super::npc_trade_command::derived(
                        b"taskboard:change:gold",
                        admitted.game_session_id,
                        id,
                    ),
                ],
            )
        } else {
            None
        };
        let request = TaskBoardCommand {
            fee_change,
            cause: QuestCause::Command(CommandRef::new(admitted.game_session_id, command)),
            action,
            payload_binding: payload.to_vec(),
        };
        let Some(result) = self.commit_task_board(fence, request, Some(&permit)).await else {
            return TaskBoardReply::Unavailable;
        };
        drop(permit);
        self.refresh_quest_session(admitted.game_session_id, false)
            .await;
        // Encode the immutable receipt view, not a new read that could change the replay response.
        let encoded = if legacy {
            projection::legacy(&result).and_then(|reply| wire::encode_task_board_reply(&reply).ok())
        } else {
            projection::full(&result).and_then(|reply| wire::full::encode_reply(&reply).ok())
        };
        encoded.map_or(TaskBoardReply::Unavailable, TaskBoardReply::Payload)
    }

    async fn commit_task_board(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: TaskBoardCommand,
        permit: Option<&SpellLanePermit>,
    ) -> Option<TaskBoardCommit> {
        let catalogue = Arc::new(match self.task_board_catalogue() {
            Some(catalogue) => catalogue,
            None if matches!(request.action, TaskBoardAction::CreditKill { .. }) => {
                TaskBoardCatalogue {
                    creatures: Vec::new(),
                    content_revision: String::new(),
                    items: std::collections::BTreeMap::new(),
                    cosmetics: None,
                    gameplay_content_revision: String::new(),
                    progression: None,
                }
            }
            None => return None,
        });
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_task_board(
                self.root,
                self.character,
                self.holder,
                fence,
                request,
                catalogue,
                permit,
            )
            .await;
        drop(slot);
        match result {
            Ok(result) => Some(result),
            Err(error) => {
                operator_event(&format!("task_board_write_failed reason={error:?}"));
                None
            }
        }
    }

    /// Credit every eligible recipient with the already memoized per-Character death occurrence.
    /// A failed write retains the kill-settle work for retry under the same occurrence.
    pub(super) async fn credit_task_board_kill(
        &self,
        fence: CurrentCharacterGameplayFence,
        race: &str,
        occurrence: [u8; 16],
    ) -> bool {
        let Ok(occurrence) = ExperienceRewardOccurrence::from_bytes(occurrence) else {
            return false;
        };
        let session = fence.game_session_id;
        let request = TaskBoardCommand {
            cause: QuestCause::CreatureDeath(occurrence),
            action: TaskBoardAction::CreditKill {
                race: race.to_owned(),
            },
            payload_binding: Vec::new(),
            fee_change: None,
        };
        if self.commit_task_board(fence, request, None).await.is_some() {
            self.refresh_quest_session(session, false).await;
            true
        } else {
            self.schedule_quest_refresh(session);
            false
        }
    }
}

macro_rules! authority_methods {
    () => {
        fn task_board_available(&self) -> bool { false }
        fn task_board_command(&self, _admitted: $crate::gameplay_transport::connection::AdmittedSession, _id:u64, _kind:u32, _payload:&[u8]) -> impl std::future::Future<Output=$crate::gameplay_transport::task_board::TaskBoardReply> {
            async { $crate::gameplay_transport::task_board::TaskBoardReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn task_board_available(&self) -> bool {
            self.task_board_catalogue().is_some()
        }
        async fn task_board_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> task_board::TaskBoardReply {
            self.task_board_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
#[path = "task_board_tests.rs"]
mod tests;
