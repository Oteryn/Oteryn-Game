//! Owner-only, durable house hireling summon and bank-service actor projection.
use super::{
    ComposedFreshAdmission,
    capabilities::OfferedCapability,
    connection::{AdmittedSession, EarnedNotice, UseCommand, UseOutcome},
    item_view::ItemKey,
};
use crate::foundation::{MovementFacing, NpcPlacement};
use oteryn_protocol_oteryn::hireling as wire;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const HIRELING_LAMP: &str = "oteryn:item.tibia.i29432";

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
    let purchase = matches!(request, wire::Request::Purchase);
    let active = owner
        .root
        .commit_house_hireling(
            fence,
            &house,
            command,
            position,
            matches!(request, wire::Request::Dismiss),
            purchase,
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

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Crystal's lamp action enters through ordinary carried-item USE. Oteryn's accepted
    /// hireling model keeps the durable identity rather than consuming or recreating it; the
    /// lamp therefore acts only as the placement control for that identity.
    pub(super) async fn use_hireling_lamp(
        &self,
        actor: crate::foundation::ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Option<UseOutcome> {
        let ItemKey::Instance(item_id) = item else {
            return None;
        };
        let fence = command.item_fence?;
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .ok()?;
        let is_lamp = inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .any(|entry| {
                entry.item.item_instance_id == item_id
                    && entry.item.definition.production_key == HIRELING_LAMP
            });
        if !is_lamp {
            return None;
        }
        let state = {
            let runtime = self.runtime.lock().await;
            runtime
                .house_actor_state(actor, command.game_session_id)
                .ok()?
        };
        let position = (
            u16::try_from(state.position.x).ok()?,
            u16::try_from(state.position.y).ok()?,
            i8::try_from(state.position.floor).ok()?,
        );
        let active = self
            .root
            .commit_house_hireling(
                fence,
                &state.house,
                command.command_id,
                Some(position),
                false,
                false,
            )
            .await
            .ok()?;
        let Some((x, y, floor)) = active else {
            return Some(UseOutcome::rejected());
        };
        let key = format!("oteryn:npc.hireling-bank:{}", state.house);
        let mut runtime = self.runtime.lock().await;
        runtime.remove_dynamic_npc(&key).ok()?;
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
        Some(UseOutcome {
            disposition: UseDisposition::Committed,
            committed: None,
            earned: EarnedNotice::NoneEarned,
            gate_message_id: None,
            refusal_detail: None,
        })
    }
}
