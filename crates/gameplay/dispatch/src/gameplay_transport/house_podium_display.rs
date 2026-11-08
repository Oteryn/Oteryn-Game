//! Independently negotiated visibility; all writes retain the existing fenced owner.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::domain::appearance::Sex;
use crate::durability::house_podium::{HousePodiumSet, HousePodiumWrite};
use crate::durability::house_scope_handoff::HouseId;
use oteryn_protocol_oteryn::{house_podium as legacy, house_podium_display as wire};
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut offers: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_HOUSE_PODIUM_DISPLAY_V1)
        .collect();
    if available {
        let at = offers.partition_point(|c| c.id < wire::CAPABILITY_HOUSE_PODIUM_DISPLAY_V1);
        offers.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_HOUSE_PODIUM_DISPLAY_V1,
                requires: &[150],
            },
        );
    }
    offers
}
macro_rules! authority_methods {
    () => {
        fn house_podium_display_available(&self) -> bool {
            false
        }
        fn house_podium_display(
            &self,
            _: $crate::gameplay_transport::connection::AdmittedSession,
            _: oteryn_protocol_oteryn::house_podium_display::Request,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn house_podium_display_available(&self) -> bool {
            true
        }
        async fn house_podium_display(
            &self,
            admitted: connection::AdmittedSession,
            request: oteryn_protocol_oteryn::house_podium_display::Request,
        ) -> Option<Vec<u8>> {
            house_podium_display::execute(self, admitted, request).await
        }
    };
}
pub(super) use composed_methods;
/// Resolve a live connection-local handle without ever treating it as a UUID.
pub(super) fn resolve(
    request: wire::Request,
    view: Option<&super::item_view::SessionItemView>,
) -> Option<wire::Request> {
    use super::item_view::ItemKey;
    let instance = |handle| match view?.resolve(std::num::NonZeroU64::new(handle)?)? {
        ItemKey::Instance(item) => Some(item),
        _ => None,
    };
    Some(match request {
        wire::Request::QueryHandle(handle) => wire::Request::Query(instance(handle)?),
        wire::Request::SetHandle {
            handle,
            mut value,
            visibility,
        } => {
            value.item = instance(handle)?;
            wire::Request::Set { value, visibility }
        }
        wire::Request::PreviewHandle {
            handle,
            mut value,
            visibility,
        } => {
            value.item = instance(handle)?;
            wire::Request::Preview { value, visibility }
        }
        other => other,
    })
}

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    request: wire::Request,
) -> Option<Vec<u8>> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_PODIUM_DISPLAY_V1)
    {
        return None;
    }
    let fence = admitted.item_fence?;
    let actor = admitted.runtime_actor?;
    let house = {
        let runtime = owner.runtime.lock().await;
        runtime
            .house_actor_state(actor, admitted.game_session_id)
            .ok()?
            .house
    };
    let house = HouseId::new(admitted.world_id, &house)?;
    let item = match &request {
        wire::Request::Query(item) => *item,
        wire::Request::Set { value, .. } | wire::Request::Preview { value, .. } => value.item,
        _ => return None,
    };
    let refused = |reason| wire::Reply::Refused(super::house_podium::refusal(reason)).encode();
    let context = match owner
        .root
        .house_podium_context(owner.holder, fence, house.clone(), item)
        .await
        .ok()?
    {
        Ok(context) => context,
        Err(reason) => return refused(reason),
    };
    let gameplay = owner.active_generation?.native_gameplay()?;
    let committed = !matches!(request, wire::Request::Preview { .. });
    let proposal = match request {
        wire::Request::Query(_) => None,
        wire::Request::Set { value, visibility } | wire::Request::Preview { value, visibility } => {
            let content = super::house_podium::PodiumContentView {
                cosmetics: gameplay.cosmetics()?.clone(),
                rewards: gameplay.reward_table().clone(),
            };
            let write = HousePodiumWrite {
                value: HousePodiumSet {
                    item_instance_id: value.item,
                    expected_revision: value.expected_revision,
                    display_kind: value.display_kind,
                    display_key: value.key.clone(),
                    direction: value.direction,
                },
                visibility: Some((visibility.display, visibility.platform)),
                preview: !committed,
            };
            match owner
                .root
                .set_house_podium_display(owner.holder, fence, house.clone(), write, content)
                .await
                .ok()?
            {
                Ok(revision) => Some((
                    legacy::Projection {
                        podium_kind: context.podium_kind,
                        display_kind: value.display_kind,
                        direction: value.direction,
                        revision,
                        key: value.key,
                    },
                    visibility,
                )),
                Err(reason) => return refused(reason),
            }
        }
        _ => return None,
    };
    let (projection, visibility) = if !committed {
        let (projection, visibility) = proposal?;
        (Some(projection), visibility)
    } else {
        let displays = match owner
            .root
            .load_house_podiums(owner.holder, fence, house)
            .await
            .ok()?
        {
            Ok(displays) => displays,
            Err(reason) => return refused(reason),
        };
        displays
            .into_iter()
            .find(|display| display.item_instance_id == item)
            .map_or((None, wire::Visibility::default()), |display| {
                (
                    Some(legacy::Projection {
                        podium_kind: display.podium_kind,
                        display_kind: display.display_kind,
                        direction: display.direction,
                        revision: display.revision,
                        key: display.display_key,
                    }),
                    wire::Visibility {
                        display: display.show_display,
                        platform: display.show_platform,
                    },
                )
            })
    };
    let platform_appearance = gameplay
        .item_policy(&context.definition_key, &context.definition_revision)
        .and_then(|policy| policy.record().authoring.presentation.as_ref())
        .and_then(|reference| {
            gameplay
                .presentation_profiles()
                .records
                .iter()
                .find(|profile| &profile.target == reference)
        })
        .and_then(|profile| match &profile.data {
            crate::content::ProjectV2AuthoringProfileData::Presentation(value) => value
                .asset_binding
                .as_deref()?
                .strip_prefix("canary.appearance:object/")?
                .parse::<u32>()
                .ok(),
            _ => None,
        })
        .filter(|id| (1..=65535).contains(id))
        .unwrap_or(0);
    let (appearance_kind, appearance) = projection
        .as_ref()
        .and_then(|display| match display.display_kind {
            1 => gameplay
                .cosmetics()?
                .look_type(&display.key, Sex::Male)
                .map(|id| (1, id)),
            2 => gameplay
                .cosmetics()?
                .mount_metadata(&display.key)
                .map(|(id, _, _)| (1, id)),
            3 => gameplay
                .policies()
                .policies()
                .iter()
                .find(|p| p.definition_key == display.key)
                .map(|p| {
                    p.object_look_type
                        .map_or((1, p.outfit_look_type), |id| (2, id))
                }),
            _ => None,
        })
        .filter(|(_, id)| (1..=65535).contains(id))
        .unwrap_or((0, 0));
    wire::Reply::State(wire::State {
        committed,
        podium_kind: context.podium_kind,
        item_definition_key: context.definition_key,
        item_definition_revision: context.definition_revision,
        platform_appearance,
        appearance_kind,
        appearance,
        visibility,
        projection,
    })
    .encode()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_transport::item_view::{ItemKey, SessionItemView};
    #[test]
    fn podium_handles_require_the_live_connection_instance_view()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut view = SessionItemView::resume(Default::default()).with_map_view();
        let key = ItemKey::Instance([7; 16]);
        view.map_view(&[key], |_| Ok(()))
            .map_err(|e| format!("{e:?}"))?;
        let handle = view.table().handle(&key).ok_or("handle")?;
        assert_eq!(
            resolve(wire::Request::QueryHandle(handle.get()), Some(&view)),
            Some(wire::Request::Query([7; 16]))
        );
        assert!(resolve(wire::Request::QueryHandle(handle.get()), None).is_none());
        view.map_view(&[], |_| Ok(()))
            .map_err(|e| format!("{e:?}"))?;
        assert!(resolve(wire::Request::QueryHandle(handle.get()), Some(&view)).is_none());
        let key = ItemKey::Entity([9; 16]);
        view.map_view(&[key], |_| Ok(()))
            .map_err(|e| format!("{e:?}"))?;
        let handle = view.table().handle(&key).ok_or("entity handle")?;
        assert!(resolve(wire::Request::QueryHandle(handle.get()), Some(&view)).is_none());
        Ok(())
    }
}
