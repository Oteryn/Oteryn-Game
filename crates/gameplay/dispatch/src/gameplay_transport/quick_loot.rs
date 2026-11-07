//! QUICKLOOT-1: manual commands delegate to the existing durable item loop.
use super::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, UseCommand},
};
use crate::foundation::CommandStatus;
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::quick_loot::{QuickLootRequest, QuickLootResult};

macro_rules! authority_methods {
    () => {
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
        async fn quick_loot(
            &self,
            admitted: connection::AdmittedSession,
            command: connection::UseCommand,
            request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
        ) -> Option<(
            crate::foundation::CommandStatus,
            oteryn_protocol_oteryn::quick_loot::QuickLootResult,
        )> {
            self.execute_quick_loot(admitted, command, request).await
        }
    };
}
pub(super) use composed_methods;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn execute_quick_loot(
        &self,
        admitted: AdmittedSession,
        command: UseCommand,
        request: QuickLootRequest,
    ) -> Option<(CommandStatus, QuickLootResult)> {
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
        let premium = self
            .session_premium_gate(admitted.game_session_id)
            .await
            .require()
            .is_ok();
        let result = self
            .root
            .commit_quick_loot(
                &permit,
                self.character,
                self.holder,
                fence,
                CommandRef::new(
                    command.game_session_id,
                    CommandId::new(command.command_id).ok()?,
                ),
                &super::item_ref_admission::NativeItemWeights(content).with_wheel(self.premium_coordinator),
                cell,
                premium,
                request,
            )
            .await
            .ok()?;
        match result {
            Ok(result) => Some((CommandStatus::Accepted, result)),
            Err(_) => Some((
                CommandStatus::Rejected,
                QuickLootResult { items: Vec::new() },
            )),
        }
    }
}
