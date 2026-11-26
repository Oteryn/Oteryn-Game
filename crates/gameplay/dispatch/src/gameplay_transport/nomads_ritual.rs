//! The source drop and pour gestures enter the common fenced World callback owner.
use super::{ComposedFreshAdmission, connection::UseCommand};
use crate::foundation::ExactActorRef;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Position, nomads_ritual};
use oteryn_protocol_oteryn::world_object::WorldObjectTarget;

pub(super) fn accepts_callback(key: &str, ritual: Option<&str>) -> bool {
    match ritual {
        Some(gesture) => key == gesture,
        None => !matches!(
            key,
            nomads_ritual::PLACE_SCIMITAR | nomads_ritual::OPEN_ENTRANCE
        ),
    }
}

pub(super) fn is_water_placement(placement: &[u8]) -> bool {
    let water = nomads_ritual::WATER;
    let Some(key) = oteryn_world_bundle::bundle::placement_key(water.floor, water.x, water.y, 0)
    else {
        return false;
    };
    placement.len() == 40 && placement[32..] == key.to_be_bytes()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn advance_nomads_ritual(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        position: Position,
        gesture: &'static str,
    ) {
        let Some(map) = self.bundle_map() else {
            return;
        };
        let Some(key) =
            oteryn_world_bundle::bundle::placement_key(position.floor, position.x, position.y, 1)
        else {
            return;
        };
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        };
        let _ = self
            .use_quest_callback_from(actor, command, map, &target, None, Some(gesture))
            .await;
    }
}

#[cfg(test)]
#[test]
fn nomads_ritual_accepts_only_the_matching_native_gesture() {
    let place = nomads_ritual::PLACE_SCIMITAR;
    let open = nomads_ritual::OPEN_ENTRANCE;
    assert!(!accepts_callback(place, None));
    assert!(!accepts_callback(open, None));
    assert!(accepts_callback(place, Some(place)));
    assert!(accepts_callback(open, Some(open)));
    assert!(!accepts_callback(open, Some(place)));
    assert!(!accepts_callback(place, Some(open)));
    assert!(accepts_callback("ordinary-use", None));
}
