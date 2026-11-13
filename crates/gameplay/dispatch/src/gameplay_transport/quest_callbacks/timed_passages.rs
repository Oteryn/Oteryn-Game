//! M07: prepare exact remote snapshots before any durable input reservation.
use super::*;
use crate::map::{
    facts::{QuestCallbackProjection, QuestTimedPassage},
    view::EntryFacts,
};

// Crystal 201189f8 actions_lions_rock.lua:13-79 handles an already-lit
// pagoda without consuming its offering or advancing personal state.
fn lion_busy_source(callback: &Callback) -> Option<Position> {
    let (source, ordinal, appearance, pagoda, item) = match callback.key.as_str() {
        "oteryn:quest-callback/m07/lion-strength" => (
            (33137, 32291),
            1,
            21390,
            (33134, 32289),
            "oteryn:item.tibia.i9634",
        ),
        "oteryn:quest-callback/m07/lion-beauty" => (
            (33138, 32369),
            3,
            2110,
            (33136, 32369),
            "oteryn:item.tibia.i21389",
        ),
        "oteryn:quest-callback/m07/lion-tears" => (
            (33154, 32279),
            2,
            21391,
            (33156, 32279),
            "oteryn:item.tibia.i21466",
        ),
        _ => return None,
    };
    let source = Position {
        x: source.0,
        y: source.1,
        floor: -8,
    };
    let pagoda = Position {
        x: pagoda.0,
        y: pagoda.1,
        floor: -8,
    };
    let [requirement] = callback.world_requirements.as_slice() else {
        return None;
    };
    let [passage] = callback.passages.as_slice() else {
        return None;
    };
    let tool = callback.item_on_target.as_ref()?;
    let native = callback.selector.native.as_ref()?;
    (callback.trigger == Trigger::Use
        && callback.actor_kind == world_callbacks::ActorKind::Player
        && callback.requires_world_admission
        && callback.lever.is_none()
        && !callback.reversible
        && callback.selector.position == Some(source)
        && callback.selector.appearance == Some(appearance)
        && callback.selector.unique.is_none()
        && callback.selector.action.is_none()
        && native.position == source
        && native.ordinal == ordinal
        && native.appearance == appearance
        && requirement.position == pagoda
        && requirement.appearance == 2074
        && requirement.ground.is_none()
        && requirement.ground_stack.is_none()
        && passage.position == pagoda
        && passage.appearance == 2074
        && passage.replacement == Some(2075)
        && !passage.if_present
        && passage.replacement_passable.is_none()
        && passage.create_on_close.is_none()
        && !passage.require_state
        && callback.effects.passages_reset_after_ms == Some(15_000)
        && tool.item == item
        && tool.count == 1
        && tool.alternatives.is_empty()
        && tool.replacement.is_none()
        && tool.target_replacement.is_none()
        && !tool.target_remove
        && !tool.reusable
        && !tool.on_step
        && !tool.selected_instance
        && tool.ingredients.is_empty())
    .then_some(pagoda)
}

/// Prove the one exact source-supported busy variant. Unknown/missing/hidden,
/// House or pickupable remote entries never authorize this no-effect result.
pub(super) fn lion_is_busy(map: &BundleMap, callback: &Callback) -> bool {
    let Some(position) = lion_busy_source(callback) else {
        return false;
    };
    let position = tile(position);
    let Some(key) = placement_key(position, 1) else {
        return false;
    };
    !map.facts.house_tile(position)
        && !map.facts.base_hidden(key)
        && map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)
            .and_then(|base| {
                base.ids()
                    .iter()
                    .zip(base.depths())
                    .filter(|(_, depth)| **depth == 0)
                    .nth(1)
                    .and_then(|(id, _)| map.facts.base_entry(position, 1, *id))
            })
            .is_some_and(|entry| !entry.pickupable && entry.appearance_id == 2075)
}

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
    if entries.is_empty() {
        // Source loops do not create objects whose conditional lookup found nothing.
        return Ok(source_reset
            .map(|(due, restore)| QuestCallbackProjection::TimedTransform { due, restore }));
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
