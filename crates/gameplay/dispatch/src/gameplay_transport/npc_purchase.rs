//! Capability-gated purchase and Obtain commands over the existing NPC and inventory owners.
use super::ComposedFreshAdmission;
use super::connection::{AdmittedSession, UseCommand};
use crate::foundation::{CommandId, CommandRef, CommandStatus};
use oteryn_protocol_oteryn::npc_purchase::ObtainContainerIntent;
use oteryn_protocol_oteryn::quick_loot::QuickLootResult;

macro_rules! authority_methods {
    () => {
        fn npc_purchase(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _intent: oteryn_protocol_oteryn::npc_purchase::NpcPurchaseIntent,
        ) -> impl std::future::Future<
            Output = (
                $crate::gameplay_transport::npc_trade_command::NpcTradeStep,
                Option<String>,
            ),
        > {
            async {
                (
                    $crate::gameplay_transport::npc_trade_command::NpcTradeStep::Refused(
                        $crate::gameplay_transport::npc_command::NpcTradeRefusal::only(
                            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected,
                        ),
                    ),
                    None,
                )
            }
        }
        fn obtain_container(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _intent: oteryn_protocol_oteryn::npc_purchase::ObtainContainerIntent,
        ) -> impl std::future::Future<
            Output = Option<(
                $crate::foundation::CommandStatus,
                oteryn_protocol_oteryn::quick_loot::QuickLootResult,
            )>,
        > {
            async {
                Some((
                    $crate::foundation::CommandStatus::Rejected,
                    oteryn_protocol_oteryn::quick_loot::QuickLootResult { items: Vec::new() },
                ))
            }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn npc_purchase(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            intent: oteryn_protocol_oteryn::npc_purchase::NpcPurchaseIntent,
        ) -> (
            $crate::gameplay_transport::npc_trade_command::NpcTradeStep,
            Option<String>,
        ) {
            $crate::gameplay_transport::npc_trade_command::purchase(self, actor, command, intent)
                .await
        }
        async fn obtain_container(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command: $crate::gameplay_transport::connection::UseCommand,
            intent: oteryn_protocol_oteryn::npc_purchase::ObtainContainerIntent,
        ) -> Option<(
            $crate::foundation::CommandStatus,
            oteryn_protocol_oteryn::quick_loot::QuickLootResult,
        )> {
            $crate::gameplay_transport::npc_purchase::obtain(self, admitted, command, intent).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn obtain(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: UseCommand,
    intent: ObtainContainerIntent,
) -> Option<(CommandStatus, QuickLootResult)> {
    let fence = command.item_fence?;
    let actor = admitted.runtime_actor?;
    let permit = admission.spell_lane_permit().await?;
    let runtime = admission.runtime.lock().await;
    let states = admission.states.spell_states.lock().await;
    let native = super::equipment_move::native_for_move(
        &runtime,
        admission.active_generation,
        states.is_dead(actor),
        states.has_pending_spell_commit(actor, command.game_session_id),
    )?;
    drop(states);
    let content = super::item_ref_admission::NativeItemWeights(native)
        .with_wheel(admission.premium_coordinator);
    match admission
        .root
        .commit_obtain_container(
            &permit,
            admission.character,
            admission.holder,
            fence,
            CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).ok()?,
            ),
            &content,
            intent,
        )
        .await
        .ok()?
    {
        Ok(result) => Some((CommandStatus::Accepted, result)),
        Err(_) => Some((
            CommandStatus::Rejected,
            QuickLootResult { items: Vec::new() },
        )),
    }
}
