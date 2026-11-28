//! QUICKLOOT-1: manual commands delegate to the existing durable item loop.
use super::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, UseCommand},
};
use crate::durability::item_loop_move::QuickLootInput;
use crate::durability::item_loop_move::QuickLootCommand;
use crate::foundation::CommandStatus;
use crate::foundation::{CommandId, CommandRef};

#[cfg(test)]
tokio::task_local! {
    pub(super) static STALL_QUICK_LOOT_COMMIT: std::cell::Cell<bool>;
}

macro_rules! authority_methods {
    () => {
        fn quick_loot_destinations(
            &self,
            _admitted: AdmittedSession,
            _command: UseCommand,
            _wire: (u32, &[u8]),
            _request: Option<oteryn_protocol_oteryn::quick_loot::QuickLootRequest>,
        ) -> impl std::future::Future<Output = Option<(crate::foundation::CommandStatus, Vec<u8>)>>
        {
            async { None }
        }
        fn quick_loot(
            &self,
            _admitted: AdmittedSession,
            _command: UseCommand,
            _request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
        ) -> impl std::future::Future<
            Output = Option<(
                crate::foundation::CommandStatus,
                oteryn_protocol_oteryn::quick_loot::QuickLootResult,
            )>,
        > {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn quick_loot_destinations(
            &self,
            admitted: connection::AdmittedSession,
            command: connection::UseCommand,
            wire: (u32, &[u8]),
            request: Option<oteryn_protocol_oteryn::quick_loot::QuickLootRequest>,
        ) -> Option<(crate::foundation::CommandStatus, Vec<u8>)> {
            self.execute_quick_loot(
                admitted,
                command,
                crate::durability::item_loop_move::QuickLootInput::Managed {
                    command_type: wire.0,
                    payload: wire.1,
                    request,
                },
            )
            .await
        }
        async fn quick_loot(
            &self,
            admitted: connection::AdmittedSession,
            command: connection::UseCommand,
            request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
        ) -> Option<(
            crate::foundation::CommandStatus,
            oteryn_protocol_oteryn::quick_loot::QuickLootResult,
        )> {
            let (status, result) = self
                .execute_quick_loot(
                    admitted,
                    command,
                    crate::durability::item_loop_move::QuickLootInput::Legacy(request),
                )
                .await?;
            Some((
                status,
                oteryn_protocol_oteryn::quick_loot::decode_quick_loot_result(&result).ok()?,
            ))
        }
    };
}
pub(super) use composed_methods;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn execute_quick_loot(
        &self,
        admitted: AdmittedSession,
        command: UseCommand,
        input: QuickLootInput<'_>,
    ) -> Option<(CommandStatus, Vec<u8>)> {
        let fence = command.item_fence?;
        let actor = admitted.runtime_actor?;
        let permit = self.spell_lane_permit().await?;
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let dead = states.is_dead(actor);
        let pending = states.has_pending_spell_commit(actor, command.game_session_id);
        drop(states);
        let content = super::equipment_move::native_for_move(
            &runtime,
            self.active_generation,
            dead,
            pending,
        )?;
        // D85 corpse identity and the durable instance have different lifetimes.
        // Keep the incoming bytes for replay, and move only from the bound item.
        let corpse = match &input {
            QuickLootInput::Legacy(request) => request.corpse,
            QuickLootInput::Managed { request, .. } => request
                .as_ref()
                .filter(|request| matches!(request.action,
                    oteryn_protocol_oteryn::quick_loot::QuickLootAction::Click
                    | oteryn_protocol_oteryn::quick_loot::QuickLootAction::LootAll))
                .and_then(|request| request.corpse),
        }.map(|identity| {
            runtime.bound_corpse(identity).map_or(identity, |corpse| corpse.item.item_instance_id)
        });
        let position = runtime.read_actor_position(actor).ok()?.position();
        let floor = position.floor;
        let cell = [
            position.x.to_be_bytes().as_slice(),
            position.y.to_be_bytes().as_slice(),
            floor.to_be_bytes().as_slice(),
        ]
        .concat()
        .try_into()
        .ok()?;
        if matches!(input, QuickLootInput::Managed { .. }) {
            if !super::battle_instinct_status::actor_is_current(&runtime, admitted) {
                return Some((CommandStatus::Rejected, Vec::new()));
            }
            drop(runtime);
        }
        let premium = self
            .session_premium_gate(admitted.game_session_id)
            .await
            .require()
            .is_ok();
        let weights = super::item_ref_admission::NativeItemWeights(content).with_wheel(self.premium_coordinator);
        let commit = self.root.commit_quick_loot(
                &permit,
                self.character,
                self.holder,
                fence,
                &weights,
                QuickLootCommand {
                    command: CommandRef::new(
                        command.game_session_id,
                        CommandId::new(command.command_id).ok()?,
                    ),
                    cell,
                    premium,
                    input,
                    selected_corpse: corpse,
                },
            );
        #[cfg(test)]
        let commit = async {
            if STALL_QUICK_LOOT_COMMIT.try_with(|_| ()).is_ok() {
                let (result, stalled) = crate::durability::with_stalled_commit_response(commit).await;
                STALL_QUICK_LOOT_COMMIT.with(|reached| reached.set(stalled));
                result
            } else {
                commit.await
            }
        };
        match commit.await.ok()? {
            Ok(result) => Some((CommandStatus::Accepted, result)),
            Err(_) => Some((CommandStatus::Rejected, Vec::new())),
        }
    }
}
