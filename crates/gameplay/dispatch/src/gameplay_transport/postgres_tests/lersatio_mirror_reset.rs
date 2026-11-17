//! Exercise the production revision-fenced mirror reset projection owner.
use crate::map::facts::{BundleFacts, QuestCallbackProjection};
use crate::map::view::{EntryFacts, MapFacts};
use oteryn_protocol_oteryn::world_map::MapDefinition;

fn mirror(appearance_id: u16) -> EntryFacts {
    EntryFacts {
        definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
        terrain_kind: None,
        appearance_id,
        blocks_projectile: false,
        pickupable: false,
        bound: true,
        count: 1,
        sub_type: 0,
    }
}

#[test]
fn lersatio_mirror_reset_waits_thirty_seconds_and_fences_newer_revision() {
    let facts = BundleFacts::default();
    let intact = mirror(8667);
    let broken = mirror(8721);
    let reset = QuestCallbackProjection::TimedTransform {
        due: 30_000_000,
        restore: intact,
    };
    assert!(facts.apply_quest_callback(1, 0, broken, &[], &[], false, Some(&reset)));
    assert_eq!(facts.expire_movement_items(29_999_999), 0);
    assert_eq!(facts.object_revision(1), 1);
    assert_eq!(facts.expire_movement_items(30_000_000), 1);
    assert_eq!(facts.object_revision(1), 2);
    let later = QuestCallbackProjection::TimedTransform {
        due: 60_000_000,
        restore: intact,
    };
    assert!(facts.apply_quest_callback(1, 2, broken, &[], &[], false, Some(&later)));
    assert!(facts.apply_quest_callback(1, 3, intact, &[], &[], false, None));
    assert_eq!(facts.object_revision(1), 4);
    facts.expire_movement_items(60_000_000);
    assert_eq!(facts.object_revision(1), 4, "stale reset must not publish");
}
