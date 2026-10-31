//! Depot search composition: retrieval withdraws a supply-stash balance, or moves the
//! depot and Inbox items through the item-loop batch owner.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use super::item_ref_admission::NativeItemWeights;
use crate::durability::character_stash::StashRequest;
use crate::durability::item_loop_move::DepotSearchRetrieval;
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::depot_search::{Outcome, Reply, Request};
use oteryn_protocol_oteryn::stash::{StashAction, StashIntent, StashOutcome};

fn retrieval_amount(available: u32, all: bool) -> Option<u32> {
    (available > 0).then_some(if all { available } else { 1 })
}

macro_rules! authority_methods {
    () => {
        fn depot_search_command(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command: u64,
            _request: oteryn_protocol_oteryn::depot_search::Request,
        ) -> impl std::future::Future<Output = oteryn_protocol_oteryn::depot_search::Reply> {
            async {
                oteryn_protocol_oteryn::depot_search::Reply {
                    outcome: oteryn_protocol_oteryn::depot_search::Outcome::Rejected,
                    retrieved: 0,
                    results: Vec::new(),
                }
            }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn depot_search_command(
            &self,
            actor: $crate::foundation::ExactActorRef,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command: u64,
            request: oteryn_protocol_oteryn::depot_search::Request,
        ) -> oteryn_protocol_oteryn::depot_search::Reply {
            depot_search::execute(self, actor, admitted, command, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    admitted: AdmittedSession,
    command: u64,
    request: Request,
) -> Reply {
    let rejected = || Reply {
        outcome: Outcome::Rejected,
        retrieved: 0,
        results: Vec::new(),
    };
    let Some(fence) = admitted.item_fence else {
        return rejected();
    };
    let current = match super::FreshAdmissionStore::from_root(owner.root.clone())
        .current_session_at(admitted.game_session_id)
        .await
    {
        Ok(value) => value.0,
        Err(_) => return rejected(),
    };
    if current.session_state() != super::GameSessionState::Active
        || super::item_fence_of(current) != Some(fence)
    {
        return rejected();
    }
    let Some(_permit) = owner.spell_lane_permit().await else {
        return rejected();
    };
    {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        if super::equipment_move::native_for_move(
            &runtime,
            owner.active_generation,
            states.is_dead(actor),
            states.has_pending_spell_commit(actor, admitted.game_session_id),
        )
        .is_none()
            || super::bundle_map_position(&mut runtime, owner.movement_cells, actor)
                .as_ref()
                .and_then(super::depot::locker)
                .is_none()
        {
            return rejected();
        }
    }
    if let Request::Retrieve { client_id, all } = request {
        let Ok(command_id) = CommandId::new(command) else {
            return rejected();
        };
        let Some(native) = owner.active_generation.and_then(|g| g.native_gameplay()) else {
            return rejected();
        };
        // Supply-stash value remains owned by `stash_conversion`: depot search only
        // resolves the server-issued row and selects one unit or the complete balance.
        // This is intentionally one balance/revision, so the operation keeps the
        // stash owner's immutable replay receipt and all-or-nothing capacity checks.
        let key = format!("oteryn:item.tibia.i{client_id}");
        let row = match owner
            .root
            .read_stash_definition(owner.character, fence.character_id, &key)
            .await
        {
            Ok(Some(row)) => row,
            // Without a stash balance, retrieval moves the depot and Inbox items, nested
            // entries included, as one batch of conserved item-loop moves.
            Ok(None) => {
                let retrieval = owner
                    .root
                    .commit_depot_search_retrieval(
                        &_permit,
                        owner.character,
                        owner.holder,
                        fence,
                        CommandRef::new(admitted.game_session_id, command_id),
                        &NativeItemWeights(native),
                        client_id,
                        all,
                    )
                    .await;
                return match retrieval {
                    Ok(Ok(DepotSearchRetrieval::Retrieved(0))) => Reply {
                        outcome: Outcome::NoRoom,
                        retrieved: 0,
                        results: Vec::new(),
                    },
                    Ok(Ok(DepotSearchRetrieval::Retrieved(retrieved))) => Reply {
                        outcome: Outcome::Retrieved,
                        retrieved,
                        results: Vec::new(),
                    },
                    Ok(Ok(DepotSearchRetrieval::NotFound)) => Reply {
                        outcome: Outcome::NotFound,
                        retrieved: 0,
                        results: Vec::new(),
                    },
                    _ => rejected(),
                };
            }
            Err(_) => return rejected(),
        };
        let Some(amount) = retrieval_amount(row.quantity, all) else {
            return rejected();
        };
        let receipt = owner
            .root
            .stash_conversion(
                &_permit,
                owner.character,
                owner.holder,
                fence,
                StashRequest {
                    command: CommandRef::new(admitted.game_session_id, command_id),
                    intent: StashIntent {
                        action: StashAction::Withdraw {
                            row: row.handle,
                            amount,
                        },
                        page: 0,
                    },
                    source: None,
                    row_open: true,
                    access_refusal: None,
                },
                native,
                &NativeItemWeights(native),
                false,
            )
            .await;
        return match receipt {
            Ok(Some(receipt)) if receipt.outcome == StashOutcome::Committed => Reply {
                outcome: Outcome::Retrieved,
                retrieved: amount,
                results: Vec::new(),
            },
            Ok(Some(receipt))
                if matches!(
                    receipt.outcome,
                    StashOutcome::NoRoom
                        | StashOutcome::NoBackpack
                        | StashOutcome::Overweight
                ) =>
            {
                Reply {
                    outcome: Outcome::NoRoom,
                    retrieved: 0,
                    results: Vec::new(),
                }
            }
            _ => rejected(),
        };
    }
    let client_id = match request {
        Request::Search { client_id }
        | Request::Detail { client_id }
        | Request::Retrieve { client_id, .. } => client_id,
    };
    let result = owner
        .root
        .read_depot_search(owner.character, fence.character_id, client_id)
        .await;
    let current = match super::FreshAdmissionStore::from_root(owner.root.clone())
        .current_session_at(admitted.game_session_id)
        .await
    {
        Ok(value) => value.0,
        Err(_) => return rejected(),
    };
    if current.session_state() != super::GameSessionState::Active
        || super::item_fence_of(current) != Some(fence)
        || owner
            .runtime
            .lock()
            .await
            .player_control_facts(actor, admitted.game_session_id)
            .is_err()
    {
        return rejected();
    }
    match result {
        Ok(Some(result)) => Reply {
            outcome: Outcome::Found,
            retrieved: 0,
            results: vec![result],
        },
        Ok(None) => Reply {
            outcome: Outcome::NotFound,
            retrieved: 0,
            results: Vec::new(),
        },
        Err(_) => rejected(),
    }
}

#[cfg(test)]
mod tests {
    use super::retrieval_amount;

    #[test]
    fn retrieval_selects_one_or_the_complete_balance() {
        assert_eq!(retrieval_amount(62, false), Some(1));
        assert_eq!(retrieval_amount(62, true), Some(62));
        assert_eq!(retrieval_amount(0, true), None);
    }
}
