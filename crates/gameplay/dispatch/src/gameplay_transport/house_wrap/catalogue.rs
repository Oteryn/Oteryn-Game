//! Current House selection, projected from the existing custody and content owners.
use super::*;

pub(in crate::gameplay_transport) async fn read(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    page: u16,
) -> Option<Result<wire::Catalogue, wire::Refusal>> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_FURNITURE_WRAP_V1)
    {
        return None;
    }
    wire::encode_catalogue_query(page)?;
    let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
        return Some(Err(wire::Refusal::NotInHouse));
    };
    let _lane = owner.spell_lane_permit().await?;
    let (house, position) = {
        let runtime = owner.runtime.lock().await;
        let Ok(state) = runtime.house_actor_state(actor, admitted.game_session_id) else {
            return Some(Err(wire::Refusal::NotInHouse));
        };
        if state.scope != fence.runtime_scope
            || state.generation != fence.scope_ownership_generation
            || state.lease.generation() != fence.character_lease_generation
            || state.lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return Some(Err(wire::Refusal::Stale));
        }
        (
            HouseId::new(admitted.world_id, &state.house)?,
            [
                state.position.x,
                state.position.y,
                i32::from(state.position.floor),
            ],
        )
    };
    let snapshot = match owner
        .root
        .house_furniture_catalogue(owner.holder, fence, house)
        .await
        .ok()?
    {
        Ok(snapshot) => snapshot,
        Err(refusal) => return Some(Err(refusal)),
    };
    let native = owner.active_generation?.native_gameplay()?;
    let mut entries = Vec::new();
    for item in snapshot.items {
        let request = wire::Request {
            action: item.action,
            item: item.item,
            expected_revision: item.revision,
            acl_revision: snapshot.acl_revision,
            position,
        };
        let Some(output) = draft(request, &item.definition, item.target.as_ref()) else {
            continue;
        };
        let label_key = if item.action == wire::Action::Unwrap {
            &output
        } else {
            &item.definition
        };
        let number = label_key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse().ok());
        let name = number.and_then(furniture_catalog::name).or_else(|| {
            let policy = native.item_policy(label_key, "definition-r1")?;
            let ReferenceItemField::Known(presentation) = &policy.record().semantics.presentation
            else {
                return None;
            };
            let ReferenceItemField::Known(name) = &presentation.name else {
                return None;
            };
            Some(name.as_str())
        });
        let Some(name) = name else {
            continue;
        };
        let mut end = name.len().min(if item.action == wire::Action::Unwrap {
            116
        } else {
            120
        });
        while !name.is_char_boundary(end) {
            end -= 1;
        }
        if end == 0 {
            continue;
        }
        let label = if item.action == wire::Action::Unwrap {
            format!("{} kit", &name[..end])
        } else {
            name[..end].into()
        };
        entries.push(wire::CatalogueEntry {
            item: item.item,
            revision: item.revision,
            action: item.action,
            name: label,
        });
    }
    let start = usize::from(page) * wire::CATALOGUE_PAGE_SIZE;
    let more = entries.len() > start + wire::CATALOGUE_PAGE_SIZE;
    Some(Ok(wire::Catalogue {
        page,
        more,
        acl_revision: snapshot.acl_revision,
        position,
        entries: entries
            .into_iter()
            .skip(start)
            .take(wire::CATALOGUE_PAGE_SIZE)
            .collect(),
    }))
}
