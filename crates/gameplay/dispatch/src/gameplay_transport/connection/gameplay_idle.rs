//! IDLEKICK: gameplay inactivity, independent of transport liveness.

use serde::Deserialize;
use tokio::time::Instant;

/// Node-configured inactivity thresholds, measured on the server's monotonic clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GameplayIdlePolicy {
    pub warning_after_ms: u64,
    pub removal_after_ms: u64,
}

impl Default for GameplayIdlePolicy {
    fn default() -> Self {
        Self {
            warning_after_ms: 15 * 60 * 1_000,
            removal_after_ms: 16 * 60 * 1_000,
        }
    }
}

/// Carried with the existing session continuity so transport replacement is not activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayIdleState {
    since: Option<Instant>,
    warned: bool,
    training: bool,
}

impl GameplayIdleState {
    pub(super) const FRESH: Self = Self {
        since: None,
        warned: false,
        training: false,
    };

    pub(super) fn activity(&mut self, now: Instant) {
        self.since = Some(now);
        self.warned = false;
    }

    pub(super) fn training(&mut self, now: Instant, training: bool) {
        if self.since.is_none() || training || self.training {
            self.activity(now);
        }
        self.training = training;
    }

    pub(in crate::gameplay_transport) fn elapsed(&self, now: Instant) -> std::time::Duration {
        self.since.map_or(std::time::Duration::ZERO, |since| {
            now.saturating_duration_since(since)
        })
    }

    /// Read the existing monotonic source; projections never refresh it.
    pub(in crate::gameplay_transport) const fn last_activity(&self) -> Option<Instant> {
        self.since
    }

    pub(super) fn warning_due(&mut self, now: Instant, policy: GameplayIdlePolicy) -> bool {
        if !self.training
            && !self.warned
            && self.elapsed(now) >= std::time::Duration::from_millis(policy.warning_after_ms)
        {
            self.warned = true;
            true
        } else {
            false
        }
    }
}

/// One classification for received gameplay actions. Queries, unknown commands, liveness
/// acknowledgements and server-driven movement/attacks never refresh the timer. Dispatch
/// additionally requires a selected capability and a registered, decoded command.
pub(super) fn meaningful(
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    selected: &[u32],
) -> bool {
    use oteryn_protocol_oteryn::{
        charm_progression, chat, depot, house_move_out, imbuement, loop_training, market, stash,
        target_move, task_board, timed_item, vip,
    };
    match command.command_type {
        1 => oteryn_protocol_oteryn::world_spatial::decode_step_intent(command.payload).is_ok(),
        2 => oteryn_protocol_oteryn::world_object::decode_use_intent_target(
            selected,
            command.payload,
        )
        .is_ok(),
        3 => super::decode_spell_cast_intent(command.payload).is_ok(),
        7 => oteryn_protocol_oteryn::npc_service::decode_npc_talk_intent(command.payload).is_ok(),
        8 => oteryn_protocol_oteryn::npc_service::decode_npc_trade_intent(command.payload).is_ok(),
        13 => chat::decode_chat_intent(command.payload).is_ok_and(|intent| {
            !matches!(
                intent,
                chat::ChatIntent::OpenRoom(_)
                    | chat::ChatIntent::CloseRoom(_)
                    | chat::ChatIntent::PrivateChannel(
                        chat::private_channel::PrivateChannelIntent::List
                    )
            )
        }),
        36 => depot::decode_depot_intent(command.payload)
            .is_ok_and(|intent| intent.action != depot::DepotAction::Browse),
        38 => timed_item::decode_timed_item_intent(command.payload)
            .is_ok_and(|intent| intent.action != timed_item::TimedAction::Inspect),
        48 | 49 => {
            vip::decode_request(command.command_type, command.payload).is_ok_and(|request| {
                !matches!(
                    request.operation,
                    vip::Operation::Status | vip::Operation::Query
                )
            })
        }
        67 => loop_training::decode_exercise_training(command.payload)
            .is_ok_and(|intent| intent != loop_training::ExerciseIntent::Status),
        76 => house_move_out::decode_house_move_out_request(command.payload)
            .is_ok_and(|request| request.action != 0),
        79 => stash::decode_stash_intent(command.payload)
            .is_ok_and(|intent| intent.action != stash::StashAction::Browse),
        80 => market::decode_request(command.payload).is_ok_and(|request| request.action >= 5),
        81 => target_move::decode_target_move_intent(command.payload).is_ok(),
        84 => charm_progression::decode_charm_progression_request(command.payload)
            .is_ok_and(|action| action != charm_progression::CharmProgressionAction::Query),
        85 => imbuement::decode_imbuement_request(command.payload)
            .is_ok_and(|request| request.action != imbuement::ImbuementAction::Query),
        89 => task_board::decode_task_board_request(command.payload)
            .is_ok_and(|request| request.action != 0),
        160 => oteryn_protocol_oteryn::prey::decode_request(command.payload)
            .is_ok_and(|request| !matches!(request, oteryn_protocol_oteryn::prey::Request::Query)),
        9 => {
            use oteryn_protocol_oteryn::item_view::*;
            let selection = ItemMoveSelection {
                equip_drop: selected.contains(&CAPABILITY_ITEM_EQUIP_DROP_V1),
                container_tree: selected.contains(
                    &oteryn_protocol_oteryn::container_tree::CAPABILITY_CONTAINER_TREE_V1,
                ),
                world_interaction: selected.contains(
                    &oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
                ),
            };
            if selected.contains(&CAPABILITY_ITEM_STACK_COUNT_V1) {
                decode_item_move_intent_with_count_for(command.payload, selection).is_ok()
            } else {
                decode_item_move_intent_for(command.payload, selection).is_ok()
            }
        }
        11 => oteryn_protocol_oteryn::attack::decode_attack_target_intent(command.payload).is_ok(),
        12 => oteryn_protocol_oteryn::attack::decode_fight_modes_intent(command.payload).is_ok(),
        25 => oteryn_protocol_oteryn::appearance::Change::decode(command.payload).is_ok(),
        31 => oteryn_protocol_oteryn::bed::decode_bed_sleep_intent(command.payload).is_ok(),
        oteryn_protocol_oteryn::daily_reward::COMMAND_TYPE_DAILY_REWARD_CLAIM => {
            oteryn_protocol_oteryn::daily_reward::DailyRewardClaim::decode(command.payload).is_ok()
        }
        40 => oteryn_protocol_oteryn::guild::decode_guild_intent(command.payload).is_ok(),
        45 => oteryn_protocol_oteryn::party::decode_party_intent(command.payload).is_ok(),
        56 => oteryn_protocol_oteryn::house::decode_house_intent(command.payload).is_ok(),
        60 => oteryn_protocol_oteryn::staff::decode_envelope(command.payload).is_ok(),
        66 => loop_training::decode_offline_training(command.payload).is_ok(),
        68 => oteryn_protocol_oteryn::guild_bank::decode_guild_bank_intent(command.payload).is_ok(),
        73 => oteryn_protocol_oteryn::cyclopedia::decode_request(
            command.command_type,
            command.payload,
        )
        .is_ok(),
        78 => oteryn_protocol_oteryn::player_trade::decode_intent(command.payload).is_ok(),
        93 => oteryn_protocol_oteryn::player_report::decode_player_report_request(command.payload)
            .is_ok(),
        97 => oteryn_protocol_oteryn::npc_shop::decode_npc_shop_intent(command.payload).is_ok(),
        98 => oteryn_protocol_oteryn::creature_push::decode_creature_push_intent(command.payload)
            .is_ok(),
        _ => false,
    }
}
