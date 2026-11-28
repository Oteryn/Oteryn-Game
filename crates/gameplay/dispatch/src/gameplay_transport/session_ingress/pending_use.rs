//! The active USE can name one receipt owner for its outstanding reservation.
use super::{CommandId, ExactActorRef, SessionCommands};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{CommandRef, GameSessionId};

#[derive(Clone, Copy)]
pub(in crate::gameplay_transport) enum UseReceipt {
    Drome,
    Craft,
    Harvest,
    Terrain,
    Coin,
}

pub(in crate::gameplay_transport) struct PendingUseReceipt {
    pub command: CommandRef,
    pub wire: Vec<u8>,
    pub receipt: Option<UseReceipt>,
}

#[derive(Clone, Copy)]
pub(in crate::gameplay_transport) struct TerrainQuestController {
    pub actor: ExactActorRef,
    pub fence: CurrentCharacterItemFence,
}

impl SessionCommands {
    pub(in crate::gameplay_transport) fn recovered_terrain_quest(
        &self,
        session: GameSessionId,
    ) -> Option<CommandId> {
        let commands = self.0.lock().ok()?;
        commands
            .get(&session)?
            .lock()
            .ok()?
            .recovered_terrain_quests
            .first()
            .copied()
    }

    pub(in crate::gameplay_transport) fn set_terrain_quest_controller(
        &self,
        controller: TerrainQuestController,
    ) {
        if let Ok(commands) = self.0.lock()
            && let Some(entry) = commands.get(&controller.fence.game_session_id)
            && let Ok(mut state) = entry.lock()
            && !state.recovered_terrain_quests.is_empty()
            && state.terrain_quest_controller.is_none_or(|old| {
                old.fence.connection_generation <= controller.fence.connection_generation
            })
        {
            state.terrain_quest_controller = Some(controller);
        }
    }

    pub(in crate::gameplay_transport) fn terrain_quest_controller(
        &self,
        session: GameSessionId,
    ) -> Option<TerrainQuestController> {
        let commands = self.0.lock().ok()?;
        commands
            .get(&session)?
            .lock()
            .ok()?
            .terrain_quest_controller
    }

    pub(in crate::gameplay_transport) fn clear_recovered_terrain_quest(&self, command: CommandRef) {
        if let Ok(commands) = self.0.lock()
            && let Some(entry) = commands.get(&command.game_session_id())
            && let Ok(mut state) = entry.lock()
        {
            state.recovered_terrain_quests.remove(&command.command_id());
            if state.recovered_terrain_quests.is_empty() {
                state.terrain_quest_controller = None;
            }
        }
    }

    pub(in crate::gameplay_transport) fn remember_use_wire(
        &self,
        command: CommandRef,
        wire: &[u8],
    ) -> Option<()> {
        let commands = self.0.lock().ok()?;
        let mut state = commands.get(&command.game_session_id())?.lock().ok()?;
        let pending = state.pending_delivery?;
        if !state.active || pending.command != command.command_id().get() {
            return None;
        }
        // Autowalk dispatches the same reserved USE again after reaching the target.
        if let Some(original) = &state.pending_use {
            return (original.command == command && original.wire == wire).then_some(());
        }
        state.pending_use = Some(PendingUseReceipt {
            command,
            wire: wire.to_vec(),
            receipt: None,
        });
        Some(())
    }

    pub(in crate::gameplay_transport) fn set_use_receipt(
        &self,
        command: CommandRef,
        receipt: Option<UseReceipt>,
    ) {
        if let Ok(commands) = self.0.lock()
            && let Some(entry) = commands.get(&command.game_session_id())
            && let Ok(mut state) = entry.lock()
            && state.active
            && let Some(pending) = state.pending_use.as_mut()
            && pending.command == command
        {
            pending.receipt = receipt;
        }
    }

    pub(in crate::gameplay_transport) fn pending_use_receipt(
        &self,
        session: GameSessionId,
    ) -> Option<Option<(CommandRef, Vec<u8>, UseReceipt)>> {
        let commands = self.0.lock().ok()?;
        let Some(entry) = commands.get(&session) else {
            return Some(None);
        };
        let state = entry.lock().ok()?;
        let Some(pending) = &state.pending_use else {
            return Some(None);
        };
        let Some(receipt) = pending.receipt else {
            return Some(None);
        };
        let id = CommandId::new(state.pending_delivery?.command).ok()?;
        if state.active
            || state.ingress.outstanding() != 1
            || pending.command != CommandRef::new(session, id)
        {
            return None;
        }
        Some(Some((pending.command, pending.wire.clone(), receipt)))
    }
}
