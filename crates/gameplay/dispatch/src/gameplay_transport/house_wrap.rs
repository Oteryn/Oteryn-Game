//! HOUSE-ITEM-WIRE-1: explicit wrap command, House custody and durable conversion.
use super::furniture_catalog;
use super::house_furnishing::{self as rules, Action, Authority, Command, Item, Rule, State};
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::content::ReferenceItemField;
use crate::domain::hireling::catalogue::{DECORATION_KIT_ITEM_ID, wrapped_trade_item};
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::house_wrap::HouseWrapCommand;
use crate::durability::item_mint::TypedDefinitionRef;
use oteryn_protocol_oteryn::house_wrap as wire;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[path = "house_wrap/catalogue.rs"]
pub(super) mod catalogue;

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
        fn house_furniture_catalogue(
            &self,
            _: $crate::gameplay_transport::connection::AdmittedSession,
            _: u16,
        ) -> impl std::future::Future<
            Output = Option<
                Result<
                    oteryn_protocol_oteryn::house_wrap::Catalogue,
                    oteryn_protocol_oteryn::house_wrap::Refusal,
                >,
            >,
        > {
            async { None }
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
        async fn house_furniture_catalogue(
            &self,
            admitted: connection::AdmittedSession,
            page: u16,
        ) -> Option<
            Result<
                oteryn_protocol_oteryn::house_wrap::Catalogue,
                oteryn_protocol_oteryn::house_wrap::Refusal,
            >,
        > {
            house_wrap::catalogue::read(self, admitted, page).await
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

fn native_wrap_eligible(id: u32) -> bool {
    #[derive(serde::Deserialize)]
    struct Eligibility {
        wrap_eligible: Vec<u32>,
    }
    static SOURCE: std::sync::OnceLock<Option<Eligibility>> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            serde_json::from_str(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/content-schema/native-gameplay/house-decoration-actions.json"
            )))
            .ok()
        })
        .as_ref()
        .is_some_and(|source| source.wrap_eligible.binary_search(&id).is_ok())
}

fn draft(
    request: wire::Request,
    definition: &str,
    target: Option<&TypedDefinitionRef>,
) -> Option<String> {
    // Crystal src/game/game.cpp:4876-4880 selects unwrap for the decoration
    // kit; its appearance eligibility never authorizes a wrap-to-self action.
    if request.action == wire::Action::Wrap && definition == "oteryn:item.tibia.i23398" {
        return None;
    }
    if let Some(target) = target {
        request.expected_revision.checked_add(1)?;
        let id = target
            .production_key
            .strip_prefix("oteryn:item.tibia.i")?
            .parse::<u32>()
            .ok()?;
        if target.family != "Item"
            || target.revision_ref != "definition-r1"
            || (!wrapped_trade_item(id) && !native_wrap_eligible(id))
        {
            return None;
        }
        let kit = format!("oteryn:item.tibia.i{DECORATION_KIT_ITEM_ID}");
        let legacy_kit =
            furniture_catalog::wrapped_to(id).map(|id| format!("oteryn:item.tibia.i{id}"));
        let proposed = match request.action {
            wire::Action::Unwrap
                if definition == kit || legacy_kit.as_deref() == Some(definition) =>
            {
                Some(target.production_key.clone())
            }
            wire::Action::Wrap if definition == target.production_key => Some(kit),
            _ => None,
        };
        if proposed.is_some() || request.action == wire::Action::Unwrap {
            return proposed;
        }
        // A prior purchased/wrapped orientation cannot suppress a currently
        // source-qualified forward pair after the same furniture was rotated.
    }
    request.expected_revision.checked_add(1)?;
    let source_id = definition
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    let (action, output_id) = match request.action {
        // Crystal src/game/game.cpp:4898 uses the common decoration kit;
        // items.cpp:238-241/item_parse.cpp:203-206 establish eligibility.
        // Preserve the existing explicit HOUSE-ITEM-WIRE-1 oven reference pair.
        wire::Action::Wrap if source_id != 6355 && native_wrap_eligible(source_id) => {
            (Action::Wrap, DECORATION_KIT_ITEM_ID)
        }
        wire::Action::Wrap => (Action::Wrap, furniture_catalog::wrapped_to(source_id)?),
        // Crystal construction_kits.lua:1-58 places a kit only on a House tile;
        // House admission and ACL fencing are enforced by execute before this draft.
        wire::Action::Unwrap => (
            Action::PlaceKit,
            furniture_catalog::construction_kit_to(source_id)?,
        ),
    };
    let output = format!("oteryn:item.tibia.i{output_id}");
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
        destruction_roll: 1,
    };
    let conversion = [Rule {
        action,
        source: definition,
        output: &output,
        amount: 0,
    }];
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
    let target = owner
        .root
        .house_furniture_kit_target(owner.character, request.item)
        .await
        .ok()?;
    let _lane = owner.spell_lane_permit().await?;
    let blocked_doors = owner.door.lock().await.blocking_cells().clone();
    let (house, placement_blocked) = {
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

        if let Some(target) = &target
            && (index
                .definition_ref(&target.family, &target.production_key, &target.revision_ref)
                .is_none()
                || index
                    .definition_ref(
                        "Item",
                        &format!("oteryn:item.tibia.i{DECORATION_KIT_ITEM_ID}"),
                        &target.revision_ref,
                    )
                    .is_none())
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
        let placement_blocked = if request.action == wire::Action::Unwrap {
            let Ok(floor) = request.position[2].try_into() else {
                return refuse(wire::Refusal::Blocked);
            };
            let cell = crate::foundation::MovementLocalPosition {
                x: request.position[0],
                y: request.position[1],
                floor,
            };
            blocked_doors.contains(&crate::content::LogicalCell {
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
                || owner.movement_cells().index().lookup(
                    owner.movement_cells().scope(),
                    crate::content::LogicalCell {
                        x: cell.x,
                        y: cell.y,
                        z: i32::from(cell.floor),
                    },
                ) != Ok(crate::content::CollisionClass::Walkable)
        } else {
            false
        };
        (
            HouseId::new(admitted.world_id, &state.house)?,
            placement_blocked,
        )
    };
    let native = owner.active_generation?.native_gameplay()?.clone();
    let reply = owner
        .root
        .wrap_house_furniture(
            owner.holder,
            fence,
            HouseWrapCommand {
                house,
                command,
                request,
            },
            // The native owner checks the exact immutable receipt before it
            // validates a fresh conversion. A replay may legitimately find its
            // original target cell blocked now. None closes the proposed fresh
            // pair, while preserving that authoritative receipt lookup.
            move |key| {
                if placement_blocked {
                    None
                } else {
                    draft(request, key, target.as_ref())
                }
            },
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
        .ok()?;
    // Keep durable replay and current authority outcomes intact. Only the
    // fresh pair rejected for our live placement check receives Blocked.
    Some(
        if placement_blocked && reply == wire::Reply::Refused(wire::Refusal::NotWrapable) {
            wire::Reply::Refused(wire::Refusal::Blocked)
        } else {
            reply
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    #[test]
    fn initialized_appearance_eligibility_wraps_transient_forms_but_not_the_kit_itself() {
        let request = wire::Request {
            action: wire::Action::Wrap,
            item: [7; 16],
            expected_revision: 1,
            acl_revision: 1,
            position: [1, 2, 7],
        };
        assert!(native_wrap_eligible(23398));
        assert!(draft(request, "oteryn:item.tibia.i23398", None).is_none());
        assert_eq!(
            draft(request, "oteryn:item.tibia.i35950", None).as_deref(),
            Some("oteryn:item.tibia.i23398")
        );
    }
    #[test]
    fn house_wrap_drafts_only_the_reference_pair() {
        let mut request = wire::Request {
            action: wire::Action::Wrap,
            item: [7; 16],
            expected_revision: 1,
            acl_revision: 1,
            position: [1, 2, 7],
        };
        assert_eq!(draft(request, FURNITURE, None).as_deref(), Some(KIT));
        assert!(draft(request, "chair", None).is_none());
        request.action = wire::Action::Unwrap;
        assert_eq!(draft(request, KIT, None).as_deref(), Some(FURNITURE));
    }

    #[test]
    fn purchased_kit_keeps_its_exact_source_target_both_directions() {
        let mut request = wire::Request {
            action: wire::Action::Unwrap,
            item: [7; 16],
            expected_revision: 1,
            acl_revision: 1,
            position: [1, 2, 7],
        };
        for id in crate::domain::hireling::catalogue::WRAPPED_TRADE_ITEM_IDS {
            let target = TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{id}"),
                revision_ref: "definition-r1".into(),
            };
            request.action = wire::Action::Unwrap;
            assert_eq!(
                draft(request, "oteryn:item.tibia.i23398", Some(&target)),
                Some(target.production_key.clone())
            );
            request.action = wire::Action::Wrap;
            assert_eq!(
                draft(request, &target.production_key, Some(&target)).as_deref(),
                Some("oteryn:item.tibia.i23398")
            );
            assert!(draft(request, "oteryn:item.tibia.i3264", Some(&target)).is_none());
        }
        request.action = wire::Action::Unwrap;
        assert!(draft(request, "oteryn:item.tibia.i23398", None).is_none());
        let forged = TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i3657".into(),
            revision_ref: "definition-r1".into(),
        };
        assert!(draft(request, "oteryn:item.tibia.i23398", Some(&forged)).is_none());
    }

    #[test]
    fn every_construction_result_is_in_the_pinned_item_index() {
        let world = crate::foundation::WorldId::decode(&[
            1, 1, 1, 1, 1, 1, 0x70, 1, 0x80, 1, 1, 1, 1, 1, 1, 1,
        ]);
        assert!(world.is_ok(), "world");
        let Ok(world) = world else { return };
        let controller = activated_with_loot_tables(world, None).0;
        let index = controller
            .active()
            .and_then(|generation| generation.native_gameplay())
            .and_then(|native| native.item_index());
        assert!(index.is_some(), "pinned item index");
        let Some(index) = index else { return };
        for (_, result) in furniture_catalog::CONSTRUCTION_KITS {
            let key = format!("oteryn:item.tibia.i{result}");
            assert_eq!(index.revision(&key), Some("definition-r1"), "{key}");
        }
    }
}
