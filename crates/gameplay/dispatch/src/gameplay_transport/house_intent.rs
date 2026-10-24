//! HOUSE-WIRE-1. Only the composed server owner supplies authority and the live pin.
use super::{
    ComposedFreshAdmission,
    connection::{FreshAdmissionAuthority, UseCommand},
};
use crate::durability::house_management::{HouseManagementError, HouseManagementRequest};
use crate::durability::house_scope_handoff::HouseId;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_housing::house_management::{HouseManagementAction, HouseManagementList};
use oteryn_protocol_oteryn::house::{self as wire, HouseDisposition as D};

pub(super) async fn dispatch(
    authority: &impl FreshAdmissionAuthority,
    actor: ExactActorRef,
    command: UseCommand,
    body: &[u8],
    query: bool,
) -> Option<Vec<u8>> {
    if query {
        return super::house_query::dispatch(authority, actor, command, body).await;
    }
    let Ok(intent) = wire::decode_house_intent(body) else {
        return Some(wire::encode_house_result(D::Rejected, 0));
    };
    authority.house_intent(actor, command, intent).await
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn manage_house(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: wire::HouseIntent,
    ) -> Option<Vec<u8>> {
        let refuse = || Some(wire::encode_house_result(D::Rejected, 0));
        let Some(fence) = command.item_fence else {
            return refuse();
        };
        if *self.world_id.as_bytes() != intent.world
            || command.game_session_id != fence.game_session_id
            || fence.runtime_scope.world_id() != self.world_id
        {
            return refuse();
        }
        let Some(house) = HouseId::new(self.world_id, &intent.house) else {
            return refuse();
        };
        let content_digest = {
            let runtime = self.runtime.lock().await;
            let Ok(lease) = runtime.bound_attacker_lease(actor, command.game_session_id) else {
                return refuse();
            };
            if runtime.owner_fence().is_err()
                || lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || lease.generation() != fence.character_lease_generation
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return refuse();
            }
            runtime.content_pin().server_artifact_digest()
        };
        if let wire::HouseAction::Bid(maximum) = intent.action {
            let Some(gate) = self.premium_coordinator else {
                return refuse();
            };
            let command_ref = CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).ok()?,
            );
            use sha2::{Digest, Sha256};
            let digest = Sha256::new()
                .chain_update(b"oteryn:house-bid-command:v1")
                .chain_update(command.game_session_id.as_bytes())
                .chain_update(command.command_id.to_be_bytes())
                .finalize();
            let mut operation_id = [0_u8; 16];
            operation_id.copy_from_slice(&digest[..16]);
            operation_id[6] = (operation_id[6] & 15) | 0x70;
            operation_id[8] = (operation_id[8] & 63) | 0x80;
            let request = crate::durability::house_ownership::HouseBidRequest {
                operation_id,
                command: command_ref,
                house_key: intent.house,
                maximum,
            };
            return match self
                .root
                .house_bid(self.character, self.holder, gate, fence, request)
                .await
            {
                Ok(_) => Some(wire::encode_house_result(D::Ok, 0)),
                Err(crate::durability::house_ownership::HouseOwnershipError::Unavailable(_)) => {
                    None
                }
                Err(crate::durability::house_ownership::HouseOwnershipError::RequestConflict) => {
                    Some(wire::encode_house_result(D::Conflict, 0))
                }
                Err(_) => refuse(),
            };
        }
        let action = match intent.action {
            wire::HouseAction::Acl {
                subowners,
                revision,
                members,
            } => HouseManagementAction::ReplaceAcl {
                list: if subowners {
                    HouseManagementList::Subowner
                } else {
                    HouseManagementList::Guest
                },
                expected_revision: revision,
                members,
            },
            wire::HouseAction::Door {
                x,
                y,
                floor,
                revision,
                members,
            } => {
                let cell = crate::content::LogicalCell {
                    x: i32::from(x),
                    y: i32::from(y),
                    z: i32::from(floor),
                };
                if let Some(map) = self.bundle_map() {
                    let Some(source) = super::house_query::source_house_id(&intent.house) else {
                        return refuse();
                    };
                    let Some(_id) = map
                        .facts
                        .house_door(crate::map::overlay::TilePos { x, y, floor }, source)
                    else {
                        return refuse();
                    };
                } else {
                    let Some(placement) = self
                        .movement_cells
                        .house_tiles()
                        .house_at(self.movement_cells.scope(), cell)
                        .ok()
                        .flatten()
                    else {
                        return refuse();
                    };
                    if placement.house().key != intent.house {
                        return refuse();
                    }
                    let Some(_id) = placement.door_at(cell) else {
                        return refuse();
                    };
                };
                HouseManagementAction::ReplaceAcl {
                    list: HouseManagementList::Door { x, y, floor },
                    expected_revision: revision,
                    members,
                }
            }
            wire::HouseAction::MoveOut(due_at_unix_ms) => {
                HouseManagementAction::MoveOut { due_at_unix_ms }
            }
            wire::HouseAction::CancelMoveOut => HouseManagementAction::CancelMoveOut,
            // Exit requires its actual owning runtime. A bid was handled above.
            wire::HouseAction::Bid(_) | wire::HouseAction::Kick(_) | wire::HouseAction::Leave => {
                return refuse();
            }
        };
        let command_id = CommandId::new(command.command_id).ok()?;
        let request = HouseManagementRequest {
            house,
            command: CommandRef::new(command.game_session_id, command_id),
            action,
            content_digest,
        };
        reply(
            self.root
                .commit_house_management(self.character, self.holder, fence, request)
                .await,
        )
    }
}
pub(super) fn reply(result: Result<u64, HouseManagementError>) -> Option<Vec<u8>> {
    match result {
        Ok(revision) => Some(wire::encode_house_result(D::Ok, revision)),
        Err(HouseManagementError::Rejected) => Some(wire::encode_house_result(D::Rejected, 0)),
        Err(HouseManagementError::Conflict) => Some(wire::encode_house_result(D::Conflict, 0)),
        Err(HouseManagementError::Unavailable(_)) => None,
    }
}
macro_rules! authority_methods {
    () => {
        fn house_intent(
            &self,
            _actor: crate::foundation::ExactActorRef,
            _command: UseCommand,
            _intent: oteryn_protocol_oteryn::house::HouseIntent,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async {
                Some(oteryn_protocol_oteryn::house::encode_house_result(
                    oteryn_protocol_oteryn::house::HouseDisposition::Rejected,
                    0,
                ))
            }
        }
        fn house_query(
            &self,
            _actor: crate::foundation::ExactActorRef,
            _command: UseCommand,
            _query: oteryn_protocol_oteryn::house::HouseQuery,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async {
                Some(oteryn_protocol_oteryn::house::encode_house_result(
                    oteryn_protocol_oteryn::house::HouseDisposition::Rejected,
                    0,
                ))
            }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn house_intent(
            &self,
            actor: ExactActorRef,
            command: connection::UseCommand,
            intent: oteryn_protocol_oteryn::house::HouseIntent,
        ) -> Option<Vec<u8>> {
            self.manage_house(actor, command, intent).await
        }
        async fn house_query(
            &self,
            actor: ExactActorRef,
            command: connection::UseCommand,
            query: oteryn_protocol_oteryn::house::HouseQuery,
        ) -> Option<Vec<u8>> {
            self.query_houses(actor, command, query).await
        }
    };
}
pub(super) use composed_methods;
