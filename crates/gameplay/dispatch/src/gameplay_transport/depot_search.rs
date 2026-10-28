//! Depot search composition; mutations fail closed until their atomic owner lands.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::depot_search::{Outcome, Reply, Request};

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
            _command: u64,
            request: oteryn_protocol_oteryn::depot_search::Request,
        ) -> oteryn_protocol_oteryn::depot_search::Reply {
            depot_search::execute(self, actor, admitted, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    admitted: AdmittedSession,
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
    // Retrieval stays fail-closed until the transaction owner can admit all selected
    // depot, Inbox and stash rows together; reads are already useful to native clients.
    if matches!(request, Request::Retrieve { .. }) {
        return rejected();
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
