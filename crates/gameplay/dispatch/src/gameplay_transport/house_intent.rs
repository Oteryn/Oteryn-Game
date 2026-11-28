//! HOUSE-WIRE-1. Only the composed server owner supplies authority and the live pin.
use super::{
    AdmissionRetryBudget, ComposedFreshAdmission,
    connection::{AdmissionRefusal, FreshAdmissionAuthority, UseCommand},
};
use crate::durability::DurabilityError;
use crate::durability::house_management::{HouseManagementError, HouseManagementRequest};
use crate::durability::house_ownership::HouseOwnershipError;
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
    include_kind: bool,
    include_guild_acl: bool,
) -> Option<Vec<u8>> {
    if query {
        return super::house_query::dispatch(authority, actor, command, body, include_kind, false, include_guild_acl)
            .await;
    }
    let Ok(intent) = wire::decode_house_intent_selected(body, include_guild_acl) else {
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
            let Some(kind) = super::house_query::source_house_kind(&intent.house) else {
                return refuse();
            };
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
            let catalogue_rent = super::house_query::source_house_rent(&intent.house)?;
            let request = crate::durability::house_ownership::HouseBidRequest {
                operation_id,
                command: command_ref,
                house_key: intent.house,
                maximum,
                content_digest,
                kind: kind.database(),
                catalogue_rent,
            };
            return match retry_house_holder(
                self.root,
                async || {
                    self.root
                        .house_bid(self.character, self.holder, gate, fence, request.clone())
                        .await
                },
                |error| matches!(error, HouseOwnershipError::Unavailable(DurabilityError::RootUnavailable)),
            )
            .await
            {
                Ok(_) => Some(wire::encode_house_result(D::Ok, 0)),
                Err(HouseOwnershipError::Unavailable(DurabilityError::RootUnavailable)) => refuse(),
                Err(HouseOwnershipError::Unavailable(error)) => {
                    #[cfg(test)]
                    eprintln!("House bid unavailable: {error:?}");
                    #[cfg(not(test))]
                    let _ = error;
                    None
                }
                Err(HouseOwnershipError::RequestConflict) => {
                    Some(wire::encode_house_result(D::Conflict, 0))
                }
                Err(HouseOwnershipError::Guildhall(reason)) => {
                    use oteryn_gameplay_housing::guildhall::GuildhallRefusal as R;
                    let disposition = match reason {
                        R::NotGuildLeader => D::NotGuildLeader,
                        R::GuildInactive => D::GuildInactive,
                        R::GuildAlreadyHasHall => D::GuildAlreadyHasHall,
                        R::GuildAlreadyBidding => D::GuildAlreadyBidding,
                        R::InsufficientFunds => D::InsufficientFunds,
                        R::BalanceLimit => D::BalanceLimit,
                        R::StaleLeader => D::StaleLeader,
                        R::NoChange => D::NoChange,
                        R::PersonalHouseLimitExceeded => D::Rejected,
                    };
                    Some(wire::encode_house_result(disposition, 0))
                }
                Err(_) => refuse(),
            };
        }
        let action = match intent.action {
            wire::HouseAction::GuildAcl {
                subowners,
                revision,
                rules,
                ordered,
            } => HouseManagementAction::ReplaceGuildAcl {
                list: if subowners {
                    HouseManagementList::Subowner
                } else {
                    HouseManagementList::Guest
                },
                expected_revision: revision,
                rules,
                ordered,
            },
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
                guild_selectors: None,
                ordered: false,
            },
            wire::HouseAction::Door {
                x,
                y,
                floor,
                revision,
                members,
                guild_selectors,
                ordered,
            } => {
                let floor = floor.checked_neg()?;
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
                        .movement_cells()
                        .house_tiles()
                        .house_at(self.movement_cells().scope(), cell)
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
                    guild_selectors,
                    ordered,
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
            retry_management(self.root, async || {
                self.root
                    .commit_house_management(self.character, self.holder, fence, request.clone())
                    .await
            })
            .await,
        )
    }
}
pub(super) async fn retry_management<T>(
    root: &crate::durability::DurabilityRoot,
    step: impl AsyncFnMut() -> Result<T, HouseManagementError>,
) -> Result<T, HouseManagementError> {
    retry_house_holder(root, step, |error| {
        matches!(error, HouseManagementError::Unavailable(DurabilityError::RootUnavailable))
    }).await
}

async fn retry_house_holder<T, E>(
    root: &crate::durability::DurabilityRoot,
    mut step: impl AsyncFnMut() -> Result<T, E>,
    holder_busy: impl Fn(&E) -> bool,
) -> Result<T, E> {
    let retry = AdmissionRetryBudget::for_command(root);
    loop {
        let result = step().await;
        // A holder checkout refusal issued no House mutation. A database error
        // or deadline on a live pass may leave a receipt to reconcile.
        if !result.as_ref().is_err_and(&holder_busy)
            || !retry.retries_unavailable(AdmissionRefusal::Unavailable).await
        {
            return result;
        }
    }
}

pub(super) fn reply(result: Result<u64, HouseManagementError>) -> Option<Vec<u8>> {
    match result {
        Ok(revision) => Some(wire::encode_house_result(D::Ok, revision)),
        Err(HouseManagementError::Rejected | HouseManagementError::Unavailable(DurabilityError::RootUnavailable)) => {
            Some(wire::encode_house_result(D::Rejected, 0))
        }
        Err(HouseManagementError::Conflict) => Some(wire::encode_house_result(D::Conflict, 0)),
        Err(HouseManagementError::Unavailable(_)) => None,
    }
}
pub(super) fn offer_guild_acl(
    base: &[super::capabilities::OfferedCapability],
    available: bool,
) -> Vec<super::capabilities::OfferedCapability> {
    use super::capabilities::OfferedCapability;
    let mut offers = base.to_vec();
    if !available
        || !offers
            .iter()
            .any(|offer| offer.id == wire::CAPABILITY_HOUSE_V1)
    {
        offers.retain(|offer| offer.id != wire::CAPABILITY_HOUSE_GUILD_ACL_V1);
    } else if let Err(at) =
        offers.binary_search_by_key(&wire::CAPABILITY_HOUSE_GUILD_ACL_V1, |offer| offer.id)
    {
        offers.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_HOUSE_GUILD_ACL_V1,
                requires: &[wire::CAPABILITY_HOUSE_V1],
            },
        );
    }
    offers
}

macro_rules! authority_methods {
    () => {
        fn house_guild_acl_available(&self) -> bool {
            false
        }
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
            _include_kind: bool,
            _auction_order: bool,
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
        fn house_guild_acl_available(&self) -> bool {
            self.bundle_map().is_some()
        }
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
            include_kind: bool,
            auction_order: bool,
        ) -> Option<Vec<u8>> {
            self.query_houses(actor, command, query, include_kind, auction_order)
                .await
        }
    };
}
pub(super) use composed_methods;
