//! HOUSE-1b owner commands composed from the admitted actor and current Channel owner.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::{house_disposition::HouseDispositionError, house_scope_handoff::HouseId};
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
use oteryn_gameplay_housing::house_move_out::HouseMoveOutCommand;
use oteryn_protocol_oteryn::house_move_out as wire;

#[derive(Debug)]
pub enum HouseMoveOutReply {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn house_move_out_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> HouseMoveOutReply {
        let Ok(request) = wire::decode_house_move_out_request(payload) else {
            return HouseMoveOutReply::Rejected;
        };
        if wire::encode_house_move_out_request(&request).is_err() {
            return HouseMoveOutReply::Rejected;
        }
        let Some(fence) = admitted.item_fence else {
            return HouseMoveOutReply::Rejected;
        };
        let Ok(command) = CommandId::new(id) else {
            return HouseMoveOutReply::Rejected;
        };
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_HOUSE_MOVE_OUT_V1)
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || admitted.world_id != binding.world_id()
            || admitted.channel_id != binding.channel_id()
            || fence.runtime_scope
                != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
            || fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
            || !admitted.runtime_actor.is_some_and(|actor| {
                runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            })
        {
            return HouseMoveOutReply::Rejected;
        }
        let Some(house) = HouseId::new(admitted.world_id, &request.house) else {
            return HouseMoveOutReply::Rejected;
        };
        let command = CommandRef::new(admitted.game_session_id, command);
        let result = match (kind, request.action) {
            (wire::COMMAND_TYPE_HOUSE_MOVE_OUT_QUERY, 0) => {
                self.root
                    .read_house_move_out_owner(self.character, self.holder, fence, house, command)
                    .await
            }
            (wire::COMMAND_TYPE_HOUSE_MOVE_OUT, 1 | 2) => {
                let action = if request.action == 1 {
                    HouseMoveOutCommand::Notice {
                        due_at_unix_ms: request.due_at_unix_ms,
                    }
                } else {
                    HouseMoveOutCommand::Cancel
                };
                self.root
                    .apply_house_move_out_command(
                        self.character,
                        self.holder,
                        fence,
                        command,
                        house,
                        action,
                    )
                    .await
            }
            _ => return HouseMoveOutReply::Rejected,
        };
        match result {
            Ok(result) => wire::encode_house_move_out_reply(&result)
                .map_or(HouseMoveOutReply::Unavailable, HouseMoveOutReply::Payload),
            Err(HouseDispositionError::Rejected | HouseDispositionError::Conflict) => {
                HouseMoveOutReply::Rejected
            }
            Err(HouseDispositionError::Unavailable(_)) => HouseMoveOutReply::Unavailable,
        }
    }

    pub(super) async fn drain_source_house_dispositions(
        &self,
    ) -> Result<(), HouseDispositionError> {
        let permit = self
            .spell_lane_permit()
            .await
            .ok_or(HouseDispositionError::Rejected)?;
        let runtime = self.runtime.lock().await;
        runtime
            .owner_fence()
            .map_err(|_| HouseDispositionError::Rejected)?;
        let binding = runtime.binding();
        self.root
            .drain_house_dispositions(
                &permit,
                self.character,
                self.holder,
                RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
                binding.scope_generation().get(),
            )
            .await
    }
}
macro_rules! authority_methods {
    () => {
        fn house_move_out_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<
            Output = $crate::gameplay_transport::house_move_out::HouseMoveOutReply,
        > {
            async { $crate::gameplay_transport::house_move_out::HouseMoveOutReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn house_move_out_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> house_move_out::HouseMoveOutReply {
            self.house_move_out_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
