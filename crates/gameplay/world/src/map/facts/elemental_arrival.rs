//! Observe the complete Elemental Spheres altar ring from one pinned map view.
use super::*;
use oteryn_gameplay_quests::quest::{
    elemental_arrival::{self as rules, ArrivalFence, CorpseArrival, ObjectSnapshot, SummonPlan},
    world_callbacks::Position,
};

/// Retained with the Channel spell lane across the physical arrival transaction.
/// This value owns no mutex guard. Park it with an unresolved spell-lane attempt
/// before issuing SQL; it remains owned until that attempt has a known outcome.
#[must_use]
pub struct ElementalArrivalReservation {
    callbacks: Arc<Mutex<Arc<CallbackStates>>>,
    keys: Vec<u64>,
    wake: Arc<tokio::sync::Notify>,
}

impl BundleFacts {
    pub fn reserve_elemental_arrival(
        &self,
        base: &crate::map::WorldBase,
        plan: &SummonPlan,
        wake: Arc<tokio::sync::Notify>,
    ) -> Option<ElementalArrivalReservation> {
        let keys: Vec<_> = std::iter::once(plan.target().key)
            .chain(plan.readset().iter().map(|object| object.key))
            .collect();
        if keys.iter().any(|key| self.doors.contains(*key)) {
            return None;
        }
        let doors = self.doors.snapshot();
        let rotations = self.rotations.lock().ok()?;
        let mut states = self.callbacks.lock().ok()?;
        let pinned = PinnedFacts {
            facts: self,
            callbacks: Some(Arc::clone(&states)),
            doors,
            rotations: Some(Arc::clone(&rotations)),
        };
        let target = plan.target().position;
        let current = Self::observe_elemental_snapshot(
            base,
            plan.fence(),
            TilePos {
                x: target.x,
                y: target.y,
                floor: target.floor,
            },
            plan.corpse(),
            false,
            &pinned,
        )?;
        if &current != plan {
            return None;
        }
        if !states.has_capacity(keys.len()) || keys.iter().any(|key| states.reserved.contains(key))
        {
            return None;
        }
        Arc::make_mut(&mut states)
            .reserved
            .extend(keys.iter().copied());
        Some(ElementalArrivalReservation {
            callbacks: Arc::clone(&self.callbacks),
            keys,
            wake,
        })
    }

    pub fn observe_elemental_arrival(
        &self,
        base: &crate::map::WorldBase,
        scope: ArrivalFence,
        position: TilePos,
        corpse: CorpseArrival,
        boss_active: bool,
    ) -> Option<SummonPlan> {
        Self::observe_elemental_snapshot(base, scope, position, corpse, boss_active, &self.pinned())
    }

    fn observe_elemental_snapshot(
        base: &crate::map::WorldBase,
        scope: ArrivalFence,
        position: TilePos,
        corpse: CorpseArrival,
        boss_active: bool,
        pinned: &PinnedFacts<'_>,
    ) -> Option<SummonPlan> {
        if scope.map != base.digest() || pinned.house_tile(position) {
            return None;
        }
        let snapshot = |position: TilePos, entry: &view::ComposedEntry| {
            if entry.facts.pickupable {
                return None;
            }
            let key = match entry.source {
                view::EntrySource::Base { placement_key, .. } => placement_key,
                view::EntrySource::Added(id) if id.get() >> 62 == 2 => id.get(),
                _ => return None,
            };
            Some(ObjectSnapshot {
                key,
                revision: pinned.object_revision(key),
                position: Position {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
                appearance: entry.facts.appearance_id,
            })
        };
        let mut entries = Vec::new();
        view::compose_into(base, None, pinned, position, &mut entries).ok()?;
        let mut targets = entries
            .iter()
            .filter(|entry| rules::recipe(entry.facts.appearance_id).is_some());
        let target = snapshot(position, targets.next()?)?;
        if targets.next().is_some() {
            return None;
        }
        let recipe = rules::recipe(target.appearance)?;
        let mut altars = Vec::with_capacity(8);
        for position in recipe.positions {
            let position = TilePos {
                x: position.x,
                y: position.y,
                floor: position.floor,
            };
            if pinned.house_tile(position) {
                return None;
            }
            entries.clear();
            view::compose_into(base, None, pinned, position, &mut entries).ok()?;
            let mut candidates = entries
                .iter()
                .filter(|entry| entry.facts.appearance_id == recipe.charged);
            let altar = snapshot(position, candidates.next()?)?;
            if candidates.next().is_some() {
                return None;
            }
            altars.push(altar);
        }
        rules::prepare_arrival(scope, target, corpse, &altars, boss_active)
    }
}

impl Drop for ElementalArrivalReservation {
    fn drop(&mut self) {
        if let Ok(mut states) = self.callbacks.lock() {
            let states = Arc::make_mut(&mut states);
            for key in &self.keys {
                states.reserved.remove(key);
            }
        }
        self.wake.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_quests::quest::{elemental_arrival as rules, world_callbacks::Position};
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};

    #[test]
    fn elemental_arrival_observes_all_eight_native_cells_from_one_snapshot() -> Result<(), String> {
        let recipe = &rules::RECIPES[0];
        let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            ("oteryn:item.tibia.i7738", 0),
            ("oteryn:item.tibia.i2126", 1),
        );
        let mut data = oteryn_world_bundle::bundle::read(&bytes).map_err(|e| e.to_string())?;
        data.manifest.world.min_x = 33088;
        data.manifest.world.max_x = 33120;
        data.manifest.world.min_y = 32160;
        data.manifest.world.max_y = 32224;
        data.manifest.world.floors = vec![-13];
        let mut sectors = BTreeMap::new();
        for (position, palette) in
            std::iter::once((recipe.summon, 3)).chain(recipe.positions.into_iter().map(|p| (p, 4)))
        {
            let sector = sectors
                .entry((position.y / 32, position.x / 32))
                .or_insert_with(|| oteryn_world_bundle::bundle::Sector {
                    floor: -13,
                    sx: position.x / 32,
                    sy: position.y / 32,
                    tiles: Vec::new(),
                });
            sector.tiles.push(Tile {
                x: position.x,
                y: position.y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items: [0, palette]
                    .into_iter()
                    .map(|palette| Item {
                        palette,
                        depth: 0,
                        attrs: Attrs::default(),
                    })
                    .collect(),
            });
        }
        data.sectors = sectors.into_values().collect();
        for sector in &mut data.sectors {
            sector.tiles.sort_by_key(|tile| (tile.y, tile.x));
        }
        let bytes =
            oteryn_world_bundle::writer::write(&data.manifest, &data.sectors, &Default::default())
                .map_err(|e| e.to_string())?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(|e| e.to_string())?
            .digest;
        let base = crate::map::load(&bytes, &pins).map_err(|e| e.to_string())?;
        let mut facts = BundleFacts::palette(&data.manifest, |key| {
            Some(ItemDefinition {
                reference: NonZeroU32::new(match key {
                    "oteryn:item.tibia.i7738" => 1,
                    "oteryn:item.tibia.i2126" => 2,
                    _ => return None,
                })?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        })
        .map_err(|e| format!("{e:?}"))?;
        for sector in &data.sectors {
            facts.push(&data.manifest, sector, &mut BTreeSet::new());
        }
        facts.finish();
        let scope = ArrivalFence {
            world: [1; 16],
            channel: [2; 16],
            scope_generation: 1,
            map: base.digest(),
            content: [3; 32],
            reset_epoch: 0,
        };
        let corpse = CorpseArrival {
            instance: [4; 16],
            state_revision: 1,
            item: 8139,
            quantity: 1,
            position: recipe.summon,
        };
        let target = TilePos {
            x: recipe.summon.x,
            y: recipe.summon.y,
            floor: -13,
        };
        let observed = facts
            .observe_elemental_arrival(&base, scope, target, corpse, false)
            .ok_or("complete native ring")?;
        assert_eq!(observed.readset().len(), 8);
        assert_eq!(observed.recipe().creature, recipe.creature);
        assert!(
            facts
                .observe_elemental_arrival(&base, scope, target, corpse, true)
                .is_none()
        );
        assert!(
            facts
                .observe_elemental_arrival(
                    &base,
                    ArrivalFence {
                        map: [0; 32],
                        ..scope
                    },
                    target,
                    corpse,
                    false
                )
                .is_none()
        );
        let cell = recipe.positions[7];
        let pos = TilePos {
            x: cell.x,
            y: cell.y,
            floor: cell.floor,
        };
        let key = placement_key(pos, 1).ok_or("cell key")?;
        let before = facts.base_entry(pos, 1, 1).ok_or("charged cell")?;
        let wake = Arc::new(tokio::sync::Notify::new());
        let reservation = facts
            .reserve_elemental_arrival(&base, &observed, Arc::clone(&wake))
            .ok_or("arrival reservation")?;
        fn assert_send<T: Send + 'static>(_: &T) {}
        assert_send(&reservation);
        assert!(!facts.quest_callback_capacity(key, 0, 0));
        assert!(
            facts
                .reserve_elemental_arrival(&base, &observed, Arc::clone(&wake))
                .is_none(),
            "overlapping arrivals cannot consume the same ring"
        );
        assert!(
            !facts.apply_quest_callback(key, 0, before, &[], &[], false, None),
            "the physical transaction owns all observed cells"
        );
        drop(reservation);
        let reset = QuestCallbackProjection::TimedTransform {
            due: 100,
            restore: EntryFacts {
                appearance_id: 7737,
                ..before
            },
        };
        assert!(facts.apply_quest_callback(key, 0, before, &[], &[], false, Some(&reset)));
        let current = facts
            .observe_elemental_arrival(&base, scope, target, corpse, false)
            .ok_or("armed ring")?;
        assert!(
            facts
                .reserve_elemental_arrival(&base, &observed, Arc::clone(&wake))
                .is_none(),
            "a pre-arm plan is stale"
        );
        let reservation = facts
            .reserve_elemental_arrival(&base, &current, Arc::clone(&wake))
            .ok_or("frozen armed ring")?;
        assert!(facts.apply_quest_callback(1, 0, before, &[], &[], false, Some(&reset)));
        assert_eq!(
            facts.expire_movement_items(100),
            1,
            "unrelated timers still expire"
        );
        assert_eq!(
            facts.object_revision(key),
            1,
            "the reserved charged field cannot decay during SQL"
        );
        assert_eq!(
            facts.next_movement_decay_time(),
            None,
            "reserved deadlines do not spin the owner"
        );
        use std::future::Future;
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        let mut notified = Box::pin(wake.notified());
        assert!(notified.as_mut().poll(&mut context).is_ready()); // consume the first release
        let mut notified = Box::pin(wake.notified());
        assert!(notified.as_mut().poll(&mut context).is_pending());
        drop(reservation);
        assert!(
            notified.as_mut().poll(&mut context).is_ready(),
            "cancellation wakes the decay owner"
        );
        assert_eq!(facts.next_movement_decay_time(), Some(100));
        assert_eq!(facts.expire_movement_items(100), 1);
        assert_eq!(facts.object_revision(key), 2);
        assert!(
            facts
                .observe_elemental_arrival(&base, scope, target, corpse, false)
                .is_none(),
            "one missing charged cell refuses the whole ring"
        );
        assert_eq!(
            observed.corpse().position,
            Position {
                x: target.x,
                y: target.y,
                floor: target.floor
            }
        );
        Ok(())
    }
}
