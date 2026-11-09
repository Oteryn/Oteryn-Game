//! Closed target arms for the carried quest owner. A corpse is a current map
//! ItemInstance; actor, base-object and self-use callbacks remain separate.
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent};

pub(super) fn accepts(callback: &RewardCallback, intent: &UseIntent) -> bool {
    if intent.use_with_position.is_some() {
        return false;
    }
    if callback.fresh_corpse {
        callback.creature_target.is_some()
            && intent.use_with.is_none()
            && matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
    } else {
        intent.use_with_map_item.is_none()
            && (callback.peer_requires.is_some() || callback.creature_target.is_some())
                == intent.use_with.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::world_object::UseTarget;
    use oteryn_protocol_oteryn::world_spatial_entities::{ENTITY_IDENTITY_BYTES, EntityRef};
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
}
