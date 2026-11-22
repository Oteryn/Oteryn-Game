use crate::map::{
    boot::{BootPins, boot, tests::bundle_with_items},
    facts::ItemDefinition,
    overlay::{TilePos, map_revision},
    view::{EntrySource, MapFacts, placement_key},
};
use oteryn_gameplay_foundation::foundation::{ChannelId, WorldId};
use oteryn_world_bundle::{bundle, writer};
use std::num::NonZeroU32;

#[test]
fn shrine_relocation_resets_the_lit_item_and_preserves_its_action()
-> Result<(), Box<dyn std::error::Error>> {
    let lit = "oteryn:item.tibia.i50244";
    let unlit = "oteryn:item.tibia.i50242";
    let (bytes, mut pins) = bundle_with_items(lit, unlit);
    let from = TilePos {
        x: 6,
        y: 0,
        floor: -7,
    };
    let to = TilePos { x: 3, ..from };
    let action = 7105;
    let mut data = bundle::read(&bytes)?;
    data.sectors[0]
        .tiles
        .iter_mut()
        .find(|tile| tile.x == from.x)
        .ok_or("shrine tile")?
        .items[1]
        .attrs
        .action = Some(action);
    let bytes = writer::write(&data.manifest, &data.sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(|e| format!("{e:?}"))?;
    let id = |tag| [1, 0, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
    let world = boot(
        &bytes,
        &BootPins {
            map_revision: map_revision(&base),
            bundle: pins,
            start: to,
        },
        WorldId::decode(&id(1)).map_err(|e| format!("{e:?}"))?,
        ChannelId::decode(&id(2)).map_err(|e| format!("{e:?}"))?,
        |key| {
            let reference = match key {
                key if key == lit => 1,
                key if key == unlit => 2,
                _ => return None,
            };
            Some(ItemDefinition {
                reference: NonZeroU32::new(reference)?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        },
    )?;
    let facts = world.facts();
    let source = placement_key(from, 1).ok_or("source key")?;
    assert_eq!(facts.action_at(source), Some(action));
    let batch = facts
        .prepare_quest_platform_batch(&base, from, to, &[50244], &[])
        .ok_or("scripted shrine relocation")?;
    assert!(facts.apply_quest_map_item_batch(&base, &batch));
    assert!(
        !facts.apply_quest_map_item_batch(&base, &batch),
        "a replay must be stale"
    );
    assert!(facts.base_hidden(source));
    let moved = facts.movement_items(to);
    let [entry] = moved.as_slice() else {
        return Err("expected one shrine arrival".into());
    };
    let EntrySource::Added(identity) = entry.source else {
        return Err("expected scenery identity".into());
    };
    assert_eq!(entry.facts.appearance_id, 50242);
    assert_eq!(facts.action_at(identity.get()), Some(action));
    Ok(())
}
