//! Closed target arms for the carried quest owner. A corpse is a current map
//! ItemInstance; actor, base-object and self-use callbacks remain separate.
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent};

pub(super) fn accepts(callback: &RewardCallback, intent: &UseIntent) -> bool {
    if intent.use_with_position.is_some() {
        return false;
    }
    if callback.carried_target.is_some() {
        !callback.fresh_corpse
            && callback.corpse_target.is_none()
            && callback.creature_target.is_none()
            && callback.peer_requires.is_none()
            && callback.self_target.is_none()
            && intent.use_with.is_none()
            && matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
    } else if callback.fresh_corpse || callback.corpse_target.is_some() {
        callback.creature_target.is_some()
            && intent.use_with.is_none()
            && matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
    } else {
        intent.use_with_map_item.is_none()
            && (callback.peer_requires.is_some()
                || callback.creature_target.is_some()
                || callback.self_target.is_some())
                == intent.use_with.is_some()
    }
}

/// Receipt-first probe for carried quest subjects at the connection ingress.
pub(crate) fn probes(intent: &UseIntent) -> bool {
    intent.use_with_position.is_none()
        && matches!(
            (intent.use_with.is_some(), intent.use_with_map_item.as_ref()),
            (true, None) | (false, Some(MapUseTarget::Item(_)))
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::world_object::UseTarget;
    use oteryn_protocol_oteryn::world_spatial_entities::{ENTITY_IDENTITY_BYTES, EntityRef};
    #[test]
    fn registered_recipe_and_slug_targets_keep_their_closed_item_arms() -> Result<(), String> {
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let slug = callbacks
            .iter()
            .find(|callback| callback.key == "oteryn:quest-reward/goblin-merchant/slug-sample")
            .ok_or("registered Slug sample")?;
        assert!(slug.fresh_corpse);
        let recipes: Vec<_> = callbacks
            .iter()
            .filter(|callback| {
                callback
                    .key
                    .starts_with("oteryn:quest-reward/m20/cursed-crystal-final-")
            })
            .collect();
        assert_eq!(recipes.len(), 2);
        for callback in recipes.into_iter().chain(std::iter::once(slug)) {
            let mut intent = UseIntent {
                target: UseTarget::ItemByDefinition(std::num::NonZeroU32::MIN),
                use_with: None,
                use_with_map_item: Some(MapUseTarget::Item(std::num::NonZeroU64::MIN)),
                use_with_position: None,
            };
            assert!(accepts(callback, &intent), "{}", callback.key);
            intent.use_with = Some(EntityRef {
                identity: [3; ENTITY_IDENTITY_BYTES],
                generation: 1,
            });
            assert!(!accepts(callback, &intent), "hybrid target");
            intent.use_with_map_item = None;
            assert!(!accepts(callback, &intent), "live target");
            intent.use_with = None;
            assert!(!accepts(callback, &intent), "missing selected Item");
            intent.use_with_map_item = Some(MapUseTarget::Object(
                oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                    placement: vec![1; 40],
                    expected_revision: 0,
                },
            ));
            assert!(!accepts(callback, &intent), "base object is not an Item");
        }
        let mut hybrid = callbacks
            .iter()
            .find(|callback| callback.carried_target.is_some())
            .ok_or("registered recipe")?
            .clone();
        hybrid.fresh_corpse = true;
        let intent = UseIntent {
            target: UseTarget::ItemByDefinition(std::num::NonZeroU32::MIN),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(std::num::NonZeroU64::MIN)),
            use_with_position: None,
        };
        assert!(
            !accepts(&hybrid, &intent),
            "recipe cannot admit a corpse policy"
        );
        Ok(())
    }

    #[test]
    fn corpse_quest_routing_keeps_target_arms_exclusive() -> Result<(), String> {
        let mut callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
            .into_iter()
            .find(|c| c.creature_target.is_some())
            .ok_or("creature callback")?;
        let mut intent = UseIntent {
            target: UseTarget::ItemByDefinition(std::num::NonZeroU32::MIN),
            use_with: Some(EntityRef {
                identity: [3; ENTITY_IDENTITY_BYTES],
                generation: 1,
            }),
            use_with_map_item: None,
            use_with_position: None,
        };
        assert!(accepts(&callback, &intent));
        callback.fresh_corpse = true;
        assert!(!accepts(&callback, &intent));
        intent.use_with_map_item = Some(MapUseTarget::Item(std::num::NonZeroU64::MIN));
        assert!(!accepts(&callback, &intent), "hybrid target");
        intent.use_with = None;
        assert!(accepts(&callback, &intent));
        callback.fresh_corpse = false;
        assert!(
            !accepts(&callback, &intent),
            "live policy cannot become corpse policy"
        );
        intent.use_with_map_item = None;
        assert!(
            !accepts(&callback, &intent),
            "creature policy cannot become self-use"
        );
        callback.creature_target = None;
        assert!(accepts(&callback, &intent));
        callback.fresh_corpse = true;
        assert!(
            !accepts(&callback, &intent),
            "corpse policy always needs exact race"
        );
        Ok(())
    }
    #[test]
    fn live_hotkey_and_corpse_routes_probe_the_quest_receipt_first() {
        let mut intent = UseIntent {
            target: UseTarget::ItemByDefinition(std::num::NonZeroU32::MIN),
            use_with: Some(EntityRef {
                identity: [3; ENTITY_IDENTITY_BYTES],
                generation: 1,
            }),
            use_with_map_item: None,
            use_with_position: None,
        };
        assert!(probes(&intent), "live hotkeys must reach the quest caller");
        intent.use_with_map_item = Some(MapUseTarget::Item(std::num::NonZeroU64::MIN));
        assert!(
            !probes(&intent),
            "hybrid actors and map targets are rejected"
        );
        intent.use_with = None;
        assert!(probes(&intent));
        intent.use_with_map_item = None;
        assert!(
            !probes(&intent),
            "ordinary self-use keeps its existing owner"
        );
    }
}
