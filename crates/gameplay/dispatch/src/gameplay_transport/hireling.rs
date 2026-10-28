//! Owner-only, durable house hireling summon and bank-service actor projection.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::foundation::{MovementFacing, NpcPlacement};
use oteryn_protocol_oteryn::hireling as wire;

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut offers: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_HIRELING_V1)
        .collect();
    if available {
        let at = offers.partition_point(|c| c.id < wire::CAPABILITY_HIRELING_V1);
        offers.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_HIRELING_V1,
                requires: &[],
            },
        );
    }
    offers
}

macro_rules! authority_methods {
    () => {
        fn hireling_available(&self) -> bool {
            false
        }
        fn hireling(
            &self,
            _: $crate::gameplay_transport::connection::AdmittedSession,
            _: u64,
            _: oteryn_protocol_oteryn::hireling::Request,
        ) -> impl std::future::Future<Output = Option<bool>> {
            async { Some(false) }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn hireling_available(&self) -> bool {
            true
        }
        async fn hireling(
            &self,
            admitted: connection::AdmittedSession,
            command: u64,
            request: oteryn_protocol_oteryn::hireling::Request,
        ) -> Option<bool> {
            hireling::execute(self, admitted, command, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    request: wire::Request,
) -> Option<bool> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HIRELING_V1)
    {
        return Some(false);
    }
    let fence = admitted.item_fence?;
    let actor = admitted.runtime_actor?;
    let house = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(actor, admitted.game_session_id)
            .ok()?;
        match request {
            wire::Request::Summon { x, y, floor }
                if state.position.x.abs_diff(i32::from(x)) <= 1
                    && state.position.y.abs_diff(i32::from(y)) <= 1
                    && state.position.floor == i16::from(floor) => {}
            wire::Request::Summon { .. } => return Some(false),
            _ => {}
        }
        state.house
    };
    let position = match request {
        wire::Request::Summon { x, y, floor } => Some((x, y, floor)),
        _ => None,
    };
    let active = owner
        .root
        .commit_house_hireling(
            fence,
            &house,
            command,
            position,
            matches!(request, wire::Request::Dismiss),
        )
        .await
        .ok()?;
    let key = format!("oteryn:npc.hireling-bank:{house}");
    let mut runtime = owner.runtime.lock().await;
    runtime.remove_dynamic_npc(&key).ok()?;
    if let Some((x, y, floor)) = active {
        runtime
            .place_dynamic_npc(
                &key,
                NpcPlacement {
                    x,
                    y,
                    floor,
                    direction: MovementFacing::South,
                },
            )
            .ok()?;
        Some(true)
    } else {
        Some(false)
    }
}
