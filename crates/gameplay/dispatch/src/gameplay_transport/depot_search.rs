//! Depot search composition: retrieval withdraws a supply-stash balance, or moves the
//! depot and Inbox items through the item-loop batch owner.
use super::item_ref_admission::NativeItemWeights;
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::{DurabilityError, depot_search::DepotSearchCommand};
use crate::durability::item_loop_move::{DepotSearchRetrieval, LoopMoveRefusal};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::depot_search::{Outcome, Reply, Request, SearchResult, TierCount};

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

        fn depot_search_completion(
            &self,
            actor: $crate::foundation::ExactActorRef,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command: u64,
            request: oteryn_protocol_oteryn::depot_search::Request,
        ) -> impl std::future::Future<Output = Option<oteryn_protocol_oteryn::depot_search::Reply>> {
            async move { Some(self.depot_search_command(actor, admitted, command, request).await) }
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
            depot_search::execute(self, actor, admitted, command, request)
                .await
                .unwrap_or_else(depot_search::rejected_reply)
        }

        async fn depot_search_completion(
            &self,
            actor: $crate::foundation::ExactActorRef,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command: u64,
            request: oteryn_protocol_oteryn::depot_search::Request,
        ) -> Option<oteryn_protocol_oteryn::depot_search::Reply> {
            depot_search::execute(self, actor, admitted, command, request).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
tokio::task_local! {
    pub(super) static LOSE_DEPOT_SEARCH_ACK: std::cell::Cell<bool>;
}

pub(super) fn rejected_reply() -> Reply {
    Reply {
        outcome: Outcome::Rejected,
        retrieved: 0,
        results: Vec::new(),
    }
}

fn retrieval_reply(result: Result<DepotSearchRetrieval, LoopMoveRefusal>) -> Reply {
    let (outcome, retrieved) = match result {
        Ok(DepotSearchRetrieval::Retrieved(0)) => (Outcome::NoRoom, 0),
        Ok(DepotSearchRetrieval::Retrieved(amount)) => (Outcome::Retrieved, amount),
        Ok(DepotSearchRetrieval::NotFound) => (Outcome::NotFound, 0),
        Err(_) => (Outcome::Rejected, 0),
    };
    Reply { outcome, retrieved, results: Vec::new() }
}

pub(super) async fn reconcile(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> bool {
    let commands = &owner.states.command_ingress;
    let Some((command, request @ Request::Retrieve { client_id, all })) =
        commands.pending_depot_search(admitted.game_session_id)
    else {
        return true;
    };
    let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
        return false;
    };
    // Resolve the immutable receipt before checking whether the actor can begin another retrieval.
    let receipt = owner.root.read_depot_search_retrieval(
        owner.character,
        fence,
        DepotSearchCommand { command, client_id, all },
    ).await;
    let reply = match receipt {
        Ok(Some(result)) => Some(retrieval_reply(result)),
        Ok(None) => execute(owner, actor, admitted, command.command_id().get(), request).await,
        Err(_) => None,
    };
    reply.is_some_and(|reply| commands.complete_depot_search(command, &reply).is_some())
}

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    admitted: AdmittedSession,
    command: u64,
    request: Request,
) -> Option<Reply> {
    let rejected = || Some(rejected_reply());
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
            || super::bundle_map_position(&mut runtime, owner.movement_cells(), actor)
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
        let reference = CommandRef::new(admitted.game_session_id, command_id);
        let commands = &owner.states.command_ingress;
        if commands.pending(reference) && commands.bind_depot_search(reference, request).is_none() {
            return None;
        }
        let retrieval = owner
            .root
            .commit_depot_search_with_stash(
                &_permit,
                owner.character,
                owner.holder,
                fence,
                DepotSearchCommand {
                    command: reference,
                    client_id,
                    all,
                },
                (
                    native,
                    &NativeItemWeights(native).with_wheel(owner.premium_coordinator),
                ),
            )
            .await;
        #[cfg(test)]
        let retrieval = if retrieval.is_ok()
            && LOSE_DEPOT_SEARCH_ACK.try_with(|lose| lose.replace(false)) == Ok(true)
        {
            Err(DurabilityError::CommitOutcomeUnknown)
        } else {
            retrieval
        };
        return match retrieval {
            Ok(result) => Some(retrieval_reply(result)),
            Err(DurabilityError::CommitOutcomeUnknown) => None,
            Err(_) => rejected(),
        };
    }
    let (client_id, tier, after) = match request {
        Request::Open { after } => (None, None, after),
        Request::Search { client_id } => (Some(client_id), None, 0),
        Request::Detail { client_id, tier } => (Some(client_id), Some(tier), 0),
        Request::Retrieve { .. } => return rejected(),
    };
    let result = if let Request::Search { client_id } = request {
        owner
            .root
            .read_depot_search(owner.character, fence.character_id, client_id)
            .await
            .map(|row| row.into_iter().collect())
    } else {
        owner
            .root
            .read_depot_holdings(owner.character, fence.character_id, client_id, tier, after)
            .await
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
        Ok(mut results) => {
            let outcome = if matches!(request, Request::Open { .. }) {
                if results.len() > oteryn_protocol_oteryn::depot_search::MAX_RESULTS {
                    results.truncate(oteryn_protocol_oteryn::depot_search::MAX_RESULTS);
                    Outcome::More
                } else {
                    Outcome::Listed
                }
            } else if let Request::Detail { client_id, tier } = request {
                if results.is_empty() {
                    results.push(SearchResult {
                        client_id,
                        depot: 0,
                        stash: 0,
                        inbox: 0,
                        tiers: vec![TierCount { tier, count: 0 }],
                    });
                }
                Outcome::Detailed
            } else if results.is_empty() {
                Outcome::NotFound
            } else {
                Outcome::Found
            };
            Some(Reply {
                outcome,
                retrieved: 0,
                results,
            })
        }
        Err(_) => rejected(),
    }
}
