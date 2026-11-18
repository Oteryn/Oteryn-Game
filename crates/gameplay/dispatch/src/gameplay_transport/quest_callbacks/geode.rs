use super::*;
use crate::map::facts::QuestCallbackProjection;
use oteryn_gameplay_quests::quest::dangerous_depth_geodes::Geode;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn project_warzone_geode_opening(
        &self,
        permit: &SpellLanePermit,
        map: &BundleMap,
        geode: Geode,
        callback: &str,
        command: crate::foundation::CommandRef,
    ) -> Result<(), ()> {
        let observed_at = self.owner_now().get();
        let Some(remaining) = self
            .root
            .warzone_geode_opening(
                permit,
                self.world_id,
                self.channel_id,
                self.reset_epoch,
                callback.into(),
                command,
            )
            .await
            .map_err(|_| ())?
        else {
            return Ok(());
        };
        let Some(due) = observed_at.checked_add(remaining) else {
            return Err(());
        };
        let position = tile(geode.entrance);
        let destination = tile(geode.landing);
        let now = self.owner_now().get();
        if now >= due || map.facts.quest_portal_destination(position, now) == Some(destination) {
            return Ok(());
        }
        let key = crate::map::view::placement_key(position, geode.entrance_ordinal).ok_or(())?;
        let source = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)
            .ok_or(())?;
        let id = *source
            .ids()
            .iter()
            .zip(source.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(geode.entrance_ordinal))
            .ok_or(())?
            .0;
        let restore = map
            .facts
            .base_entry(position, geode.entrance_ordinal, id)
            .filter(|entry| entry.appearance_id == 388)
            .ok_or(())?;
        let portal = map
            .facts
            .movement_item_facts(1949)
            .filter(|entry| !entry.pickupable)
            .ok_or(())?;
        if map.facts.base_hidden(key) || map.facts.house_tile(position) {
            return Err(());
        }
        map.facts
            .apply_quest_callback(
                key,
                map.facts.object_revision(key),
                portal,
                &[],
                &[(key, portal, true)],
                false,
                Some(&QuestCallbackProjection::TimedEntrance {
                    due,
                    restore,
                    position,
                    destination,
                }),
            )
            .then_some(())
            .ok_or(())
    }
}
