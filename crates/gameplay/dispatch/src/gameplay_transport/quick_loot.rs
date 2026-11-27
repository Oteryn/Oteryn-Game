//! QUICKLOOT-1: manual commands delegate to the existing durable item loop.
use super::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, UseCommand},
};
use crate::durability::item_loop_move::QuickLootInput;
use crate::durability::item_loop_move::QuickLootCommand;
use crate::{durability::DurabilityError, foundation::CommandStatus};
use oteryn_protocol_oteryn::quick_loot::{self as wire, QuickLootRefusal};
use crate::foundation::{CommandId, CommandRef};

pub struct OpenLootSelection {
    pub corpse_identity: [u8; 16],
    pub entry: [u8; 16],
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
        fn quick_loot_selected(
            &self,
            _admitted: AdmittedSession,
            _command: UseCommand,
            _request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
            _selection: Option<crate::gameplay_transport::quick_loot::OpenLootSelection>,
        ) -> impl std::future::Future<Output = Option<(crate::foundation::CommandStatus, Vec<u8>)>>
        {
            async { None }
        }
        fn quick_loot_wire(
            &self,
            _admitted: AdmittedSession,
            _command: UseCommand,
            _request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
        ) -> impl std::future::Future<Output = Option<(crate::foundation::CommandStatus, Vec<u8>)>> {
            async { None }
        }

        #[cfg(test)]
        fn quick_loot(
            &self,
            admitted: AdmittedSession,
            command: UseCommand,
            request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
        ) -> impl std::future::Future<
            Output = Option<(
                crate::foundation::CommandStatus,
                oteryn_protocol_oteryn::quick_loot::QuickLootResult,
            )>,
        > {
            async move {
                use oteryn_protocol_oteryn::quick_loot as wire;
                let (status, payload) = self.quick_loot_wire(admitted, command, request).await?;
                let items = if status == crate::foundation::CommandStatus::Rejected {
                    wire::decode_quick_loot_refusal(&payload).ok()?;
                    Vec::new()
                } else {
                    wire::decode_quick_loot_result(&payload).ok()?.items
                };
                Some((status, wire::QuickLootResult { items }))
            }
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
                None,
            )
            .await
        }
        async fn quick_loot_selected(
            &self,
            admitted: connection::AdmittedSession,
            command: connection::UseCommand,
            request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
            selection: Option<crate::gameplay_transport::quick_loot::OpenLootSelection>,
        ) -> Option<(crate::foundation::CommandStatus, Vec<u8>)> {
            self.execute_quick_loot(
                admitted,
                command,
                crate::durability::item_loop_move::QuickLootInput::Selected {
                    request,
                    entry: selection.as_ref().map(|selection| selection.entry),
                },
                selection.map(|selection| selection.corpse_identity),
            )
            .await
        }
        async fn quick_loot_wire(
            &self,
            admitted: connection::AdmittedSession,
            command: connection::UseCommand,
            request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
        ) -> Option<(crate::foundation::CommandStatus, Vec<u8>)> {
            self.execute_quick_loot(
                admitted,
                command,
                crate::durability::item_loop_move::QuickLootInput::Legacy(request),
                None,
            )
            .await
        }
    };
}
pub(super) use composed_methods;

pub(super) fn log_refusal(
    admitted: &AdmittedSession,
    command_id: u64,
    stage: &str,
    reason: &impl std::fmt::Debug,
) {
    let server_sequence = admitted.continuity.server_sequence.saturating_add(1);
    let reason = format!("{reason:?}").replace(' ', "_");
    super::operator_event(&format!(
        "event=quick_loot_refused stage={stage} reason={reason} command_id={command_id} server_sequence={server_sequence}"
    ));
}

fn rejected(reason: QuickLootRefusal) -> Option<(CommandStatus, Vec<u8>)> {
    Some((CommandStatus::Rejected, wire::encode_quick_loot_refusal(reason)))
}

fn durability_refusal(error: &DurabilityError) -> QuickLootRefusal {
    match error {
        DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded => {
            QuickLootRefusal::Busy
        }
        _ => QuickLootRefusal::Unavailable,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn execute_quick_loot(
        &self,
        admitted: AdmittedSession,
        command: UseCommand,
        input: QuickLootInput<'_>,
        open_corpse: Option<[u8; 16]>,
    ) -> Option<(CommandStatus, Vec<u8>)> {
        let refuse = |stage, reason| {
            log_refusal(&admitted, command.command_id, stage, &reason);
            rejected(reason)
        };
        let Some(fence) = command.item_fence else {
            return refuse("missing_item_fence", QuickLootRefusal::Stale);
        };
        let Some(actor) = admitted.runtime_actor else {
            return refuse("missing_actor", QuickLootRefusal::Stale);
        };
        let Ok(command_id) = CommandId::new(command.command_id) else {
            return refuse("command_id", QuickLootRefusal::InvalidRequest);
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return refuse("spell_lane", QuickLootRefusal::Unavailable);
        };
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let dead = states.is_dead(actor);
        let pending = states.has_pending_spell_commit(actor, command.game_session_id);
        drop(states);
        if dead {
            return refuse("actor_dead", QuickLootRefusal::Rejected);
        }
        if pending {
            return refuse("pending_spell_commit", QuickLootRefusal::Busy);
        }
        let Some(content) = super::equipment_move::native_for_move(
            &runtime,
            self.active_generation,
            dead,
            pending,
        ) else {
            return refuse("content_pin", QuickLootRefusal::Unavailable);
        };
        if !matches!(input, QuickLootInput::Legacy(_))
            && !super::battle_instinct_status::actor_is_current(&runtime, admitted)
        {
            return refuse("actor_fence", QuickLootRefusal::Stale);
        }
        // D85 corpse identity and the durable instance have different lifetimes.
        // Keep the incoming bytes for replay, and move only from the bound item.
        let legacy_corpse = |identity| {
            runtime.bound_corpse(identity).map_or(identity, |corpse| corpse.item.item_instance_id)
        };
        let corpse = match &input {
            QuickLootInput::Legacy(request) => request.corpse.map(legacy_corpse),
            QuickLootInput::Managed { request, .. } => request
                .filter(|request| matches!(request.action,
                    oteryn_protocol_oteryn::quick_loot::QuickLootAction::Click
                    | oteryn_protocol_oteryn::quick_loot::QuickLootAction::LootAll))
                .and_then(|request| request.corpse)
                .map(legacy_corpse),
            QuickLootInput::Selected { request, .. } => open_corpse.and_then(|identity| {
                let bound = runtime.bound_corpse(identity)?;
                let instance = bound.item.item_instance_id;
                request.corpse.is_none_or(|supplied| supplied == identity || supplied == instance)
                    .then_some(instance)
            }),
        };
        let position = match runtime.read_actor_position(actor) {
            Ok(position) => position.position(),
            Err(error) => {
                log_refusal(&admitted, command.command_id, "actor_position", &error);
                return rejected(QuickLootRefusal::Stale);
            }
        };
        let mut cell = [0; 10];
        cell[..4].copy_from_slice(&position.x.to_be_bytes());
        cell[4..8].copy_from_slice(&position.y.to_be_bytes());
        cell[8..].copy_from_slice(&position.floor.to_be_bytes());
        drop(runtime);
        let premium = if input.needs_premium_decision()
            && let Some(coordinator) = self.premium_coordinator
            && let Some(account) = self.session_account(admitted.game_session_id)
        {
            match coordinator.quick_loot_premium(account).await {
                Ok(gate) => gate.premium_current(),
                Err(error) => {
                    log_refusal(&admitted, command.command_id, "premium", &error);
                    return rejected(durability_refusal(&error));
                }
            }
        } else {
            false
        };
        let result = self
            .root
            .commit_quick_loot(
                &permit,
                self.character,
                self.holder,
                fence,
                &super::item_ref_admission::NativeItemWeights(content).with_wheel(self.premium_coordinator),
                QuickLootCommand {
                    command: CommandRef::new(
                        command.game_session_id,
                        command_id,
                    ),
                    cell,
                    premium,
                    input,
                    selected_corpse: corpse,
                },
            )
            .await;
        match result {
            Ok(Ok(result)) => Some((CommandStatus::Accepted, result)),
            Ok(Err(reason)) => {
                log_refusal(&admitted, command.command_id, "transaction_refused", &reason);
                rejected(reason.wire_reason())
            }
            Err(error) => {
                log_refusal(&admitted, command.command_id, "durability", &error);
                match error {
                    // Keep the in-flight CommandId for receipt reconciliation when the
                    // transaction may have committed before the acknowledgement failed.
                    DurabilityError::CommitOutcomeUnknown
                    | DurabilityError::RootPassDeadlineExceeded
                    | DurabilityError::RootTaskFailed => None,
                    _ => rejected(durability_refusal(&error)),
                }
            }
        }
    }
}
