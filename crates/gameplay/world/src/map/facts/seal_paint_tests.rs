use super::*;
use crate::map::boot::tests as fixture;

fn facts(served: bool) -> Result<BundleFacts, String> {
    let (bytes, _) = fixture::bundle();
    let bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(|error| format!("{error:?}"))?;
    BundleFacts::palette(&bundle.manifest, |key| {
        if key == "oteryn:item.tibia.i7252" {
            served.then_some(ItemDefinition {
                reference: NonZeroU32::new(3)?,
                solid: Some(true),
                blocks_projectile: true,
                pickupable: false,
                door: false,
            })
        } else {
            fixture::items(key)
        }
    })
    .map_err(|error| format!("{error:?}"))
}

fn seal() -> TilePos {
    TilePos {
        x: 31964,
        y: 31203,
        floor: -7,
    }
}

#[test]
fn painted_seal_uses_served_facts_without_a_base_palette_entry() -> Result<(), String> {
    let facts = facts(true)?;
    assert!(
        !facts
            .palette
            .iter()
            .any(|entry| entry.facts.appearance_id == 7252)
    );
    let key = placement_key(seal(), 2).ok_or("seal placement")?;
    let painted = facts
        .quest_callback_replacement(key, 7252)
        .ok_or("served painted seal")?;
    assert_eq!(
        painted.definition,
        MapDefinition::Item(NonZeroU32::new(3).ok_or("painted definition reference")?)
    );
    assert_eq!(painted.appearance_id, 7252);
    assert!(!painted.pickupable);
    assert!(painted.blocks_projectile);
    Ok(())
}

#[test]
fn paint_projection_restores_the_original_seal_after_thirty_seconds() -> Result<(), String> {
    let facts = facts(true)?;
    let key = placement_key(seal(), 2).ok_or("seal placement")?;
    let painted = facts
        .quest_callback_replacement(key, 7252)
        .ok_or("served painted seal")?;
    let original = EntryFacts {
        definition: MapDefinition::Item(NonZeroU32::new(4).ok_or("original definition reference")?),
        appearance_id: 7178,
        ..painted
    };
    let reset = QuestCallbackProjection::TimedTransform {
        due: 30_000_000,
        restore: original,
    };
    assert!(facts.apply_quest_callback(key, 0, painted, &[], &[], false, Some(&reset)));
    assert_eq!(facts.base_entry(seal(), 2, 0), Some(painted));
    assert_eq!(facts.expire_movement_items(29_999_999), 0);
    assert_eq!(facts.base_entry(seal(), 2, 0), Some(painted));
    assert_eq!(facts.expire_movement_items(30_000_000), 1);
    assert_eq!(facts.base_entry(seal(), 2, 0), Some(original));
    assert_eq!(facts.object_revision(key), 2);
    Ok(())
}

#[test]
fn missing_painted_definition_never_fabricates_replacement_facts() -> Result<(), String> {
    let facts = facts(false)?;
    let key = placement_key(seal(), 2).ok_or("seal placement")?;
    assert!(facts.quest_callback_replacement(key, 7252).is_none());
    assert!(facts.quest_callback_replacement(key, 7251).is_none());
    Ok(())
}
