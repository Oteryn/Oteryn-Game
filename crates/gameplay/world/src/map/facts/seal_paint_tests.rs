use super::*;
use crate::map::boot::tests as fixture;

fn facts(served: bool) -> Result<BundleFacts, String> {
    let (bytes, _) = fixture::bundle();
    let bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(|error| format!("{error:?}"))?;
    BundleFacts::palette(&bundle.manifest, |key| {
        if matches!(key, "oteryn:item.tibia.i7252" | "oteryn:item.tibia.i4996") {
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
fn barbarian_test_replacements_resolve_without_placed_bear_or_mammoth_states() -> Result<(), String>
{
    let (bytes, _) = fixture::bundle();
    let bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(|error| format!("{error:?}"))?;
    let facts = BundleFacts::palette(&bundle.manifest, |key| {
        let reference = match key {
            "oteryn:item.tibia.i7175" => NonZeroU32::new(7175),
            "oteryn:item.tibia.i7177" => NonZeroU32::new(7177),
            _ => return fixture::items(key),
        }?;
        Some(ItemDefinition {
            reference,
            solid: Some(true),
            blocks_projectile: true,
            pickupable: false,
            door: false,
        })
    })
    .map_err(|error| format!("{error:?}"))?;
    for (x, y, appearance) in [(32253, 31049, 7175), (32154, 31028, 7177)] {
        let placement =
            placement_key(TilePos { x, y, floor: -5 }, 1).ok_or("barbarian trial placement")?;
        let state = facts
            .quest_callback_replacement(placement, appearance)
            .ok_or("served barbarian trial replacement")?;
        assert_eq!(state.appearance_id, appearance);
        assert_eq!(
            state.definition,
            MapDefinition::Item(
                NonZeroU32::new(u32::from(appearance)).ok_or("definition reference")?
            )
        );
        assert!(!state.pickupable);
    }
    Ok(())
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

#[test]
fn ape_city_amphora_strikes_resolve_unplaced_broken_appearance() -> Result<(), String> {
    let facts = facts(true)?;
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    for uid in 40092..=40095 {
        let callback = callbacks
            .iter()
            .find(|callback| callback.selector.unique == Some(uid))
            .ok_or("amphora callback")?;
        let target = callback.selector.native.as_ref().ok_or("native amphora")?;
        let pos = TilePos {
            x: target.position.x,
            y: target.position.y,
            floor: target.position.floor,
        };
        let key = placement_key(pos, target.ordinal).ok_or("amphora placement")?;
        let appearance = callback.effects.transform.ok_or("broken appearance")?;
        assert_eq!(
            facts
                .quest_callback_replacement(key, appearance)
                .map(|entry| entry.appearance_id),
            Some(appearance),
            "amphora UID {uid}",
        );
    }
    Ok(())
}
