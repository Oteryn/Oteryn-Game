//! HOUSE-ITEM-WIRE-1: explicit wrap command, House custody and durable conversion.
use super::house_furnishing::{self as rules, Action, Authority, Command, Item, Rule, State};
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::content::ReferenceItemField;
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::house_wrap::HouseWrapCommand;
use oteryn_protocol_oteryn::house_wrap as wire;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const FURNITURE: &str = "oteryn:item.tibia.i6355";
const KIT: &str = "oteryn:item.tibia.i6371";

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut out: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_HOUSE_FURNITURE_WRAP_V1)
        .collect();
    if available {
        let at = out.partition_point(|c| c.id < wire::CAPABILITY_HOUSE_FURNITURE_WRAP_V1);
        out.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
                requires: &[49],
            },
        );
    }
    out
}
macro_rules! authority_methods {
    () => {
        fn house_wrap_available(&self) -> bool {
            false
        }
        fn house_wrap(
            &self,
            _: $crate::gameplay_transport::connection::AdmittedSession,
            _: u64,
            _: oteryn_protocol_oteryn::house_wrap::Request,
        ) -> impl std::future::Future<Output = Option<oteryn_protocol_oteryn::house_wrap::Reply>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn house_wrap_available(&self) -> bool {
            self.bundle_map().is_some()
                && self
                    .active_generation
                    .and_then(|g| g.native_gameplay())
                    .is_some()
        }
        async fn house_wrap(
            &self,
            admitted: connection::AdmittedSession,
            command: u64,
            request: oteryn_protocol_oteryn::house_wrap::Request,
        ) -> Option<oteryn_protocol_oteryn::house_wrap::Reply> {
            house_wrap::execute(self, admitted, command, request).await
        }
    };
}
pub(super) use composed_methods;

fn draft(request: wire::Request, definition: &str) -> Option<String> {
    request.expected_revision.checked_add(1)?;
    let action = match request.action {
        wire::Action::Wrap => Action::Wrap,
        wire::Action::Unwrap => Action::Unwrap,
    };
    let mut state = State {
        revision: request.expected_revision,
        ..State::default()
    };
    let items = match request.action {
        wire::Action::Wrap => &mut state.house,
        wire::Action::Unwrap => &mut state.inventory,
    };
    items.insert(
        1,
        Item {
            definition: definition.into(),
            amount: 1,
            liquid: None,
            hit_points: 0,
        },
    );
    let command = Command {
        revision: request.expected_revision,
        generation: 1,
        acl_revision: request.acl_revision,
        action,
        source: 1,
        target: None,
    };
    let conversion = [
        Rule {
            action: Action::Wrap,
            source: FURNITURE,
            output: KIT,
            amount: 0,
        },
        Rule {
            action: Action::Unwrap,
            source: KIT,
            output: FURNITURE,
            amount: 0,
        },
    ];
    if rules::dispatch(
        &mut state,
        Some(Authority {
            generation: 1,
            acl_revision: request.acl_revision,
            may_edit: true,
        }),
        &command,
        &conversion,
    ) != UseDisposition::Committed
    {
        return None;
    }
    let after = match request.action {
        wire::Action::Wrap => state.inventory.get(&1),
        wire::Action::Unwrap => state.house.get(&1),
    }?;
    Some(after.definition.clone())
}

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    request: wire::Request,
) -> Option<wire::Reply> {
    let refuse = |r| Some(wire::Reply::Refused(r));
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_FURNITURE_WRAP_V1)
    {
        return None;
    }
    let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
        return refuse(wire::Refusal::NotInHouse);
    };
    if fence.game_session_id != admitted.game_session_id
        || fence.connection_generation.get() != admitted.continuity.connection_generation
    {
        return refuse(wire::Refusal::Stale);
    }
    let _lane = owner.spell_lane_permit().await?;
    let blocked_doors = owner.door.lock().await.blocking_cells().clone();
    let house = {
        let runtime = owner.runtime.lock().await;
        let Some(index) =
            super::item_ref_admission::pinned_index(&runtime, owner.active_generation)
        else {
            return refuse(wire::Refusal::Stale);
        };
        // The pair comes from reference evidence; its definitions must belong to this pin.
        if index
            .definition_ref("Item", FURNITURE, "definition-r1")
            .is_none()
            || index.definition_ref("Item", KIT, "definition-r1").is_none()
        {
            return refuse(wire::Refusal::NotWrapable);
        }

        let Ok(state) = runtime.house_actor_state(actor, admitted.game_session_id) else {
            return refuse(wire::Refusal::NotInHouse);
        };
        if state.scope != fence.runtime_scope
            || state.generation != fence.scope_ownership_generation
            || state.lease.generation() != fence.character_lease_generation
            || state.lease.character_id().as_bytes() != fence.character_id.as_bytes()
        {
            return refuse(wire::Refusal::Stale);
        }
        if request.action == wire::Action::Unwrap {
            let Ok(floor) = request.position[2].try_into() else {
                return refuse(wire::Refusal::Blocked);
            };
            let cell = crate::foundation::MovementLocalPosition {
                x: request.position[0],
                y: request.position[1],
                floor,
            };
            if blocked_doors.contains(&crate::content::LogicalCell {
                x: cell.x,
                y: cell.y,
                z: i32::from(cell.floor),
            }) || runtime
                .house_cell_occupied_except(
                    actor,
                    admitted.game_session_id,
                    crate::foundation::LocalPosition {
                        x: cell.x,
                        y: cell.y,
                        floor: cell.floor,
                    },
                )
                .ok()?
                || owner.movement_cells.index().lookup(
                    owner.movement_cells.scope(),
                    crate::content::LogicalCell {
                        x: cell.x,
                        y: cell.y,
                        z: i32::from(cell.floor),
                    },
                ) != Ok(crate::content::CollisionClass::Walkable)
            {
                return refuse(wire::Refusal::Blocked);
            }
        }
        HouseId::new(admitted.world_id, &state.house)?
    };
    let native = owner.active_generation?.native_gameplay()?.clone();
    owner
        .root
        .wrap_house_furniture(
            owner.holder,
            fence,
            HouseWrapCommand {
                house,
                command,
                request,
            },
            move |key| draft(request, key),
            move |key| {
                let policy = native.item_policy(key, "definition-r1")?;
                let ReferenceItemField::Known(container) = &policy.record().semantics.container
                else {
                    return None;
                };
                let ReferenceItemField::Known(capacity) = container.capacity else {
                    return None;
                };
                u8::try_from(capacity).ok()
            },
        )
        .await
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn house_wrap_drafts_only_the_reference_pair() {
        let mut request = wire::Request {
            action: wire::Action::Wrap,
            item: [7; 16],
            expected_revision: 1,
            acl_revision: 1,
            position: [1, 2, 7],
        };
        assert_eq!(draft(request, FURNITURE).as_deref(), Some(KIT));
        assert!(draft(request, "chair").is_none());
        request.action = wire::Action::Unwrap;
        assert_eq!(draft(request, KIT).as_deref(), Some(FURNITURE));
    }
}
