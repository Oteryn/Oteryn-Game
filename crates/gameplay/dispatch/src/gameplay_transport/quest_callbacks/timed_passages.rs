//! M07: prepare exact remote snapshots before any durable input reservation.
use super::*;
use crate::map::{
    facts::{QuestCallbackProjection, QuestTimedPassage},
    view::EntryFacts,
};

pub(super) fn prepare(
    map: &BundleMap,
    callback: &Callback,
    source: (u64, u64),
    now: u64,
    projection: Option<QuestCallbackProjection>,
    passages: &mut Vec<u64>,
    replacements: &mut Vec<(u64, EntryFacts, bool)>,
) -> Result<Option<QuestCallbackProjection>, UseDisposition> {
    let Some(delay) = callback.effects.passages_reset_after_ms else {
        return Ok(projection);
    };
    let due = now
        .checked_add(u64::from(delay) * 1_000)
        .ok_or(UseDisposition::Exhausted)?;
    let source_reset = match projection {
        Some(QuestCallbackProjection::TimedTransform { due, restore }) => Some((due, restore)),
        None => None,
        _ => return Err(UseDisposition::Rejected),
    };
    let mut entries = Vec::new();
    for (key, after, passable) in passages.iter().map(|key| (*key, None, false)).chain(
        replacements
            .iter()
            .map(|(key, after, passable)| (*key, Some(*after), *passable)),
    ) {
        let (position, ordinal) =
            crate::map::view::placement_of(key).ok_or(UseDisposition::Rejected)?;
        let tile = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)
            .ok_or(UseDisposition::Rejected)?;
        let id = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(ordinal))
            .map(|(id, _)| *id)
            .ok_or(UseDisposition::Rejected)?;
        let before = map
            .facts
            .base_entry(position, ordinal, id)
            .ok_or(UseDisposition::Rejected)?;
        entries.push(QuestTimedPassage {
            key,
            expected_revision: map.facts.object_revision(key),
            before,
            after,
            passable,
        });
    }
    let projection = QuestCallbackProjection::TimedPassages {
        due,
        entries,
        source_reset,
    };
    if !map
        .facts
        .quest_timed_passages_ready(source.0, source.1, &projection)
    {
        return Err(UseDisposition::Rejected);
    }
    passages.clear();
    replacements.clear();
    Ok(Some(projection))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::boot::BundleCollisionIndexExt;

    #[test]
    fn m07_callback_adapter_preflights_real_bundle_before_publishing_remote_reset()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| scenario().map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "M07 callback adapter thread panicked")?
            .map_err(Into::into)
    }

    fn scenario() -> Result<(), Box<dyn std::error::Error>> {
        let id = |tag| [1, 0, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
        let world = crate::foundation::WorldId::decode(&id(1))?;
        let channel = crate::foundation::ChannelId::decode(&id(2))?;
        let room = crate::content::qualify_native_entry_room(world)?;
        let (bytes, pins) = crate::map::boot::tests::bundle();
        let map_revision = format!(
            "sha256:{}",
            pins.digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let booted = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                bundle: pins,
                map_revision,
                start: TilePos {
                    x: 2,
                    y: 0,
                    floor: -7,
                },
            },
            world,
            channel,
            crate::map::boot::tests::items,
        )?;
        let cells = booted.movement_cells(room.movement_cells())?;
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            cells.index()
        else {
            return Err("bundle collision index".into());
        };
        let map = index.map();
        let mut callback = world_callbacks::load()?
            .into_iter()
            .find(|callback| callback.key == "oteryn:quest-callback/l6/waterskin-of-mead")
            .ok_or("native callback")?;
        callback.effects.passages_reset_after_ms = Some(7000);
        let source = placement_key(
            TilePos {
                x: 3,
                y: 0,
                floor: -7,
            },
            0,
        )
        .ok_or("source key")?;
        let remote = placement_key(
            TilePos {
                x: 6,
                y: 0,
                floor: -7,
            },
            1,
        )
        .ok_or("remote key")?;
        let mut passages = vec![remote];
        let mut replacements = Vec::new();
        assert!(matches!(
            prepare(
                map,
                &callback,
                (source, 0),
                u64::MAX,
                None,
                &mut passages,
                &mut replacements
            ),
            Err(UseDisposition::Exhausted)
        ));
        assert_eq!(passages, vec![remote]);
        let projection = prepare(
            map,
            &callback,
            (source, 0),
            0,
            None,
            &mut passages,
            &mut replacements,
        )
        .map_err(|error| format!("{error:?}"))?
        .ok_or("timed projection")?;
        assert!(passages.is_empty());
        assert!(
            !map.facts.base_hidden(remote),
            "preflight cannot hide an object"
        );
        assert_eq!(map.facts.next_movement_decay_time(), None);
        let tile = map.overlay.base().tile(3, 0, -7).ok_or("source tile")?;
        let before = map
            .facts
            .base_entry(
                TilePos {
                    x: 3,
                    y: 0,
                    floor: -7,
                },
                0,
                *tile.ids().first().ok_or("source item")?,
            )
            .ok_or("source facts")?;
        assert!(map.facts.apply_quest_callback(
            source,
            0,
            before,
            &[],
            &[],
            false,
            Some(&projection)
        ));
        assert!(map.facts.base_hidden(remote));
        assert_eq!(map.facts.expire_movement_items(6_999_999), 0);
        assert_eq!(map.facts.expire_movement_items(7_000_000), 1);
        assert!(!map.facts.base_hidden(remote));
        passages.push(remote);
        assert!(matches!(
            prepare(
                map,
                &callback,
                (source, 0),
                8_000_000,
                None,
                &mut passages,
                &mut replacements
            ),
            Err(UseDisposition::Rejected)
        ));
        assert_eq!(passages, vec![remote]);
        Ok(())
    }
}
