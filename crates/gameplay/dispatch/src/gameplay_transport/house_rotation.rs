//! W18: explicit, capability-gated rotation and current native House selection.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::durability::consume_transform::{ConsumeTransformRequest, HouseActionContext};
use crate::foundation::{CommandId, CommandRef};
use crate::map::view::{self, MapFacts};
use oteryn_protocol_oteryn::house_rotation as wire;
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Rule {
    item_id: u32,
    kind: String,
}
#[derive(Deserialize)]
struct Catalogue {
    schema: String,
    crystal_commit: String,
    native_transforms: Vec<Rule>,
}
fn rotatable(key: &str) -> bool {
    static RULES: OnceLock<Option<Catalogue>> = OnceLock::new();
    let Some(id) = key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse::<u32>().ok())
    else {
        return false;
    };
    RULES
        .get_or_init(|| {
            let rules: Catalogue = serde_json::from_str(include_str!(
                "../../../../../tools/content-schema/native-gameplay/house-decoration-actions.json"
            ))
            .ok()?;
            (rules.schema == "OTERYN_HOUSE_DECORATION_ACTIONS/v1"
                && rules.crystal_commit == "201189f871d23597b82caf88fd611454a2ddefb8")
                .then_some(rules)
        })
        .as_ref()
        .is_some_and(|rules| {
            rules
                .native_transforms
                .iter()
                .any(|rule| rule.item_id == id && rule.kind == "ROTATE")
        })
}
fn label(key: &str) -> String {
    let source = key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse().ok())
        .and_then(super::furniture_catalog::name)
        .unwrap_or("House furniture");
    let mut end = source.len().min(120);
    while !source.is_char_boundary(end) {
        end -= 1;
    }
    source[..end].to_owned()
}
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut offers = base
        .iter()
        .copied()
        .filter(|offer| offer.id != wire::CAPABILITY_HOUSE_FURNITURE_ROTATION_V1)
        .collect::<Vec<_>>();
    if available {
        let at =
            offers.partition_point(|offer| offer.id < wire::CAPABILITY_HOUSE_FURNITURE_ROTATION_V1);
        offers.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_HOUSE_FURNITURE_ROTATION_V1,
                requires: &[49],
            },
        );
    }
    offers
}
macro_rules! authority_methods {
    () => {
        fn house_rotation_available(&self)->bool{false}
        fn house_rotation(&self,_:$crate::gameplay_transport::connection::AdmittedSession,_:u64,_:oteryn_protocol_oteryn::house_rotation::Request)
        ->impl std::future::Future<Output=Option<oteryn_protocol_oteryn::house_rotation::Reply>> {async{None}}
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn house_rotation_available(&self) -> bool {
            self.bundle_map().is_some()
                && self
                    .active_generation
                    .and_then(|g| g.native_gameplay())
                    .is_some()
        }
        async fn house_rotation(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            request: oteryn_protocol_oteryn::house_rotation::Request,
        ) -> Option<oteryn_protocol_oteryn::house_rotation::Reply> {
            house_rotation::execute(self, admitted, id, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    id: u64,
    request: wire::Request,
) -> Option<wire::Reply> {
    let refuse = |disposition| Some(wire::Reply::Applied(disposition));
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_FURNITURE_ROTATION_V1)
    {
        return refuse(UseDisposition::Rejected);
    }
    let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
        return refuse(UseDisposition::Rejected);
    };
    if fence.game_session_id != admitted.game_session_id
        || fence.connection_generation.get() != admitted.continuity.connection_generation
    {
        return refuse(UseDisposition::StaleState);
    }
    if let wire::Request::Rotate(wire::Target::Base(target)) = &request {
        return owner
            .rotate_house_base_decoration(
                actor,
                super::connection::UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: id,
                    item_fence: Some(fence),
                },
                target,
            )
            .await
            .map(|reply| wire::Reply::Applied(reply.disposition));
    }
    let _lane = owner.spell_lane_permit().await?;
    let context = {
        let runtime = owner.runtime.lock().await;
        if super::item_ref_admission::pinned_index(&runtime, owner.active_generation).is_none() {
            return refuse(UseDisposition::StaleState);
        }
        let Ok(context) =
            HouseActionContext::from_native_owner(&runtime, actor, admitted.game_session_id, fence)
        else {
            return refuse(UseDisposition::NotPossible);
        };
        context
    };
    match request {
        wire::Request::Query(page) => query(owner, &context, page)
            .await
            .map(wire::Reply::Catalogue),
        wire::Request::Rotate(wire::Target::Item {
            item,
            expected_revision,
        }) => {
            let native = owner.active_generation?.native_gameplay()?;
            let mut intent = wire::COMMAND.to_be_bytes().to_vec();
            intent.extend(
                wire::Request::Rotate(wire::Target::Item {
                    item,
                    expected_revision,
                })
                .encode()?,
            );
            let request = ConsumeTransformRequest {
                command: CommandRef::new(admitted.game_session_id, CommandId::new(id).ok()?),
                source: item,
                target: None,
                intent,
            };
            let reply = owner
                .root
                .consume_transform_in_house(
                    owner.holder,
                    context.with_rotation(expected_revision),
                    request,
                    native,
                )
                .await
                .ok()?;
            Some(wire::Reply::Applied(reply.disposition))
        }
        wire::Request::Rotate(wire::Target::Base(_)) => None,
    }
}

// The bounded catalogue keeps the current House/range proof and all source entries
// together; the typed target carries its current revision, not an editable position.
#[allow(clippy::too_many_lines)]
async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    context: &HouseActionContext,
    page: u16,
) -> Option<wire::Catalogue> {
    if page > wire::MAX_PAGE {
        return None;
    }
    let map = owner.bundle_map()?;
    let native = owner.active_generation?.native_gameplay()?;
    let index = native.item_index()?;
    let at = context.position();
    let snapshot = owner
        .root
        .read_house_action_items(owner.holder, context.clone())
        .await
        .ok()??;
    let mut entries = Vec::new();
    for item in snapshot.items {
        let pos = HouseActionContext::decode_spatial_position(&item.spatial_position)?;
        if pos[2] != at[2]
            || pos[0].abs_diff(at[0]) > 1
            || pos[1].abs_diff(at[1]) > 1
            || !rotatable(&item.definition.production_key)
        {
            continue;
        }
        if native
            .item_policy(
                &item.definition.production_key,
                &item.definition.revision_ref,
            )
            .is_none()
        {
            continue;
        }
        entries.push(wire::Entry {
            label: label(&item.definition.production_key),
            target: wire::Target::Item {
                item: item.item_instance_id,
                expected_revision: item.state_revision,
            },
        });
    }
    let changes = owner
        .house_base_decoration_facts(context, map.overlay.base().digest())
        .await?;
    let base = map.facts.base_only();
    let facts = super::world_map::DecoratedFacts {
        base: &base,
        changes: &changes,
    };
    let house = super::house_query::source_house_id(context.house().house_key())?;
    for dx in -1..=1 {
        for dy in -1..=1 {
            let x = u16::try_from(at[0].checked_add(dx)?).ok()?;
            let y = u16::try_from(at[1].checked_add(dy)?).ok()?;
            let floor = i8::try_from(at[2]).ok()?.checked_neg()?;
            let pos = crate::map::overlay::TilePos { x, y, floor };
            if map.facts.house_id(pos) != Some(house) {
                continue;
            }
            let mut stack = Vec::new();
            view::compose_into(map.overlay.base(), None, &facts, pos, &mut stack).ok()?;
            for entry in stack {
                let view::MapEntryKey::Base(key) = entry.source.key() else {
                    continue;
                };
                let MapDefinition::Item(reference) = entry.facts.definition else {
                    continue;
                };
                let definition = super::item_look::typed_definition(index, reference)?;
                if !rotatable(&definition.production_key) {
                    continue;
                }
                entries.push(wire::Entry {
                    label: label(&definition.production_key),
                    target: wire::Target::Base(WorldObjectTarget {
                        placement: map
                            .overlay
                            .base()
                            .digest()
                            .into_iter()
                            .chain(key.to_be_bytes())
                            .collect(),
                        expected_revision: facts.object_revision(key),
                    }),
                });
            }
        }
    }
    entries.sort_by(|a, b| {
        a.label.cmp(&b.label).then_with(|| {
            wire::Request::Rotate(a.target.clone())
                .encode()
                .cmp(&wire::Request::Rotate(b.target.clone()).encode())
        })
    });
    // Current authority is re-read after both independent owner projections.
    if !owner
        .root
        .house_action_current(owner.holder, context)
        .await
        .ok()?
    {
        return None;
    }
    let start = usize::from(page).checked_mul(wire::PAGE_SIZE)?;
    let more = entries.len() > start + wire::PAGE_SIZE;
    Some(wire::Catalogue {
        page,
        more,
        entries: entries
            .into_iter()
            .skip(start)
            .take(wire::PAGE_SIZE)
            .collect(),
    })
}
