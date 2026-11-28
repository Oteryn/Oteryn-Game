//! Reconcile the original reservation without dispatching the command again.
use super::{AdmittedSession, CommandId, PendingDelivery, SessionCommandStream, SessionCommands};
use crate::durability::command_receipt::{LostCommandReply, ReplyReceipt};
use crate::gameplay_transport::connection::AdmissionRefusal;
use crate::gameplay_transport::{AdmissionRetryBudget, ComposedFreshAdmission};
use oteryn_protocol_oteryn::{
    ClientCommandView, boss_difficulty, container_batch, container_sort, guild_bank, player_report,
    quick_loot, team_finder, team_finder_members, vip, world_event,
};

/// Add a family here only when its original receipt proves a terminal wire result.
/// Add its read-only keyed lookup in durability. Commands with follow-up effects
/// must reconcile those effects in their family owner before returning a reply.
fn receipt_for(command: ClientCommandView<'_>) -> Option<ReplyReceipt> {
    match command.command_type {
        boss_difficulty::COMMAND_TYPE_BOSS_DIFFICULTY
            if boss_difficulty::decode_request(command.payload).is_ok_and(|request| {
                !matches!(request, boss_difficulty::Request::Start { .. })
            }) =>
        {
            Some(ReplyReceipt::BossDifficulty)
        }
        container_sort::COMMAND_TYPE_CONTAINER_SORT => Some(ReplyReceipt::ContainerSort),
        guild_bank::COMMAND_TYPE_GUILD_BANK_INTENT
            if guild_bank::decode_guild_bank_intent(command.payload).is_ok_and(|intent| {
                !matches!(intent.action, guild_bank::GuildBankAction::Disband(_))
            }) =>
        {
            Some(ReplyReceipt::GuildBank)
        }
        container_batch::COMMAND_TYPE_MANAGED_CONTAINER_MOVE
        | container_batch::COMMAND_TYPE_STASH_CONTAINER
            if matches!(
                container_batch::Request::decode(command.payload),
                Some(container_batch::Request::Confirm { .. })
            ) =>
        {
            Some(
                if command.command_type == container_batch::COMMAND_TYPE_STASH_CONTAINER {
                    ReplyReceipt::StashContainer
                } else {
                    ReplyReceipt::ManagedContainerMove
                },
            )
        }
        player_report::COMMAND_TYPE_PLAYER_REPORT
        | player_report::COMMAND_TYPE_NAMED_RULE_VIOLATION => Some(ReplyReceipt::PlayerReport),
        quick_loot::COMMAND_TYPE_QUICK_LOOT
        | quick_loot::COMMAND_TYPE_QUICK_LOOT_DESTINATIONS
        | quick_loot::COMMAND_TYPE_QUICK_LOOT_DISTRIBUTE => Some(ReplyReceipt::QuickLoot),
        team_finder::COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT
        | team_finder::COMMAND_TYPE_TEAM_FINDER_LEADER_REMOVE => {
            Some(ReplyReceipt::TeamFinderLeader)
        }
        team_finder_members::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION
        | team_finder_members::COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION => {
            Some(ReplyReceipt::TeamFinderMember)
        }
        vip::COMMAND_TYPE_VIP_LIST
        | vip::COMMAND_TYPE_VIP_CONSENT
        | vip::COMMAND_TYPE_VIP_QUERY => Some(ReplyReceipt::Vip),
        world_event::COMMAND_TYPE_WORLD_EVENT_QUERY => Some(ReplyReceipt::WorldEvent),
        _ => None,
    }
}

impl SessionCommandStream {
    pub(in crate::gameplay_transport) fn track_reply_receipt(
        &self,
        command: ClientCommandView<'_>,
    ) {
        let Some(receipt) = receipt_for(command) else {
            return;
        };
        if let Some(entry) = &self.state
            && let Ok(mut state) = entry.lock()
            && let Some(pending) = &mut state.pending_delivery
            && pending.command == command.command_id
        {
            pending.reply_receipt = Some(receipt);
        }
    }
}

impl PendingDelivery {
    fn matches_lost(self, lost: &AdmittedSession) -> bool {
        self.generation == lost.continuity.connection_generation
            && self.selected == lost.continuity.selected_capabilities
            && self.sequence.checked_sub(1) == Some(lost.continuity.server_sequence)
            && self.command.checked_add(1) == Some(lost.continuity.next_command_id)
    }
}

impl SessionCommands {
    fn lost_reply_receipt(&self, lost: &AdmittedSession) -> Option<(CommandId, ReplyReceipt)> {
        let commands = self.0.lock().ok()?;
        let state = commands.get(&lost.game_session_id)?.lock().ok()?;
        let pending = state.pending_delivery?;
        if state.active || state.ingress.outstanding() != 1 || !pending.matches_lost(lost) {
            return None;
        }
        Some((
            CommandId::new(pending.command).ok()?,
            pending.reply_receipt?,
        ))
    }

    fn finish_lost_reply(
        &self,
        lost: &AdmittedSession,
        command: CommandId,
        receipt: ReplyReceipt,
        rejected: bool,
        payload: &[u8],
    ) -> Option<bool> {
        let commands = self.0.lock().ok()?;
        let mut state = commands.get(&lost.game_session_id)?.lock().ok()?;
        if state.active {
            return None;
        }
        let Some(pending) = state.pending_delivery else {
            // A parallel resume can have reconciled the same original already.
            return (state.ingress.outstanding() == 0).then_some(false);
        };
        if !pending.matches_lost(lost)
            || pending.command != command.get()
            || pending.reply_receipt != Some(receipt)
        {
            return None;
        }
        state.complete_reconciled(command, rejected, payload)?;
        Some(true)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn recover_reply_receipt(
        &self,
        retry: &AdmissionRetryBudget<'_>,
        lost: &AdmittedSession,
        last_applied_sequence: u64,
    ) -> Result<(), AdmissionRefusal> {
        use AdmissionRefusal::{Rejected, Unavailable};
        let ingress = &self.states.command_ingress;
        let Some((command, receipt)) = ingress.lost_reply_receipt(lost) else {
            return Ok(());
        };
        let fence = lost.item_fence.ok_or(Rejected)?;
        if fence.game_session_id != lost.game_session_id
            || fence.connection_generation.get() != lost.continuity.connection_generation
            || last_applied_sequence > lost.continuity.server_sequence
        {
            return Err(Rejected);
        }
        let reply = retry
            .retry_unavailable(async || {
                self.root
                    .read_lost_command_reply(fence, command, receipt)
                    .await
                    .map_err(|_| Unavailable)
            })
            .await?;
        let (rejected, payload) = match &reply {
            LostCommandReply::Stale => return Err(Rejected),
            LostCommandReply::Absent if receipt == ReplyReceipt::ContainerSort => (true, &[2][..]),
            LostCommandReply::Absent => (true, &[][..]),
            LostCommandReply::Stored { .. } if receipt == ReplyReceipt::ContainerSort => {
                (false, &[1][..])
            }
            LostCommandReply::Stored { rejected, payload } => (*rejected, payload.as_slice()),
        };
        let applied = ingress
            .finish_lost_reply(lost, command, receipt, rejected, payload)
            .ok_or(Unavailable)?;
        if applied
            && receipt == ReplyReceipt::Vip
            && matches!(reply, LostCommandReply::Stored { .. })
        {
            self.states.chat.vip.invalidate();
        }
        Ok(())
    }
}
