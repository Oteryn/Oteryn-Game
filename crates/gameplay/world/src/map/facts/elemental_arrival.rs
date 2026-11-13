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
    positions: Vec<TilePos>,
    wake: Arc<tokio::sync::Notify>,
}

impl BundleFacts {
    pub fn reserve_elemental_arrival(
        &self,
        base: &crate::map::WorldBase,
        plan: &SummonPlan,
        wake: Arc<tokio::sync::Notify>,
    ) -> Option<ElementalArrivalReservation> {
        self.reserve_elemental_arrival_with_ground(base, plan, wake, None)
    }

    /// Retain both durable field identities and every observed tile until the SQL outcome.
    pub fn reserve_elemental_arrival_with_ground(
        &self,
        base: &crate::map::WorldBase,
        plan: &SummonPlan,
        wake: Arc<tokio::sync::Notify>,
        ground: Option<(&crate::map::overlay::ChannelOverlay, &dyn MapFacts)>,
    ) -> Option<ElementalArrivalReservation> {
        let positions: Vec<_> = std::iter::once(plan.target().position)
            .chain(plan.readset().iter().map(|object| object.position))
            .map(|position| TilePos {
                x: position.x,
                y: position.y,
                floor: position.floor,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
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
            ground,
        )?;
        if &current != plan {
            return None;
        }
        if !states.has_capacity(keys.len() + positions.len())
            || keys.iter().any(|key| states.key_reserved(*key))
            || positions
                .iter()
                .any(|position| states.position_reserved(*position))
        {
            return None;
        }
        let states = Arc::make_mut(&mut states);
        states.reserved.extend(keys.iter().copied());
        states.reserved_positions.extend(positions.iter().copied());
        Some(ElementalArrivalReservation {
            callbacks: Arc::clone(&self.callbacks),
            keys,
            positions,
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
        self.observe_elemental_arrival_with_ground(base, scope, position, corpse, boss_active, None)
    }

    /// Include the actual durable Ground view held by the caller's item lane.
    pub fn observe_elemental_arrival_with_ground(
        &self,
        base: &crate::map::WorldBase,
        scope: ArrivalFence,
        position: TilePos,
        corpse: CorpseArrival,
        boss_active: bool,
        ground: Option<(&crate::map::overlay::ChannelOverlay, &dyn MapFacts)>,
    ) -> Option<SummonPlan> {
        Self::observe_elemental_snapshot(
            base,
            scope,
            position,
            corpse,
            boss_active,
            &self.pinned(),
            ground,
        )
    }

    fn observe_elemental_snapshot(
        base: &crate::map::WorldBase,
        scope: ArrivalFence,
        position: TilePos,
        corpse: CorpseArrival,
        boss_active: bool,
        pinned: &PinnedFacts<'_>,
        ground: Option<(&crate::map::overlay::ChannelOverlay, &dyn MapFacts)>,
    ) -> Option<SummonPlan> {
        if scope.map != base.digest() || pinned.house_tile(position) {
            return None;
        }
        if ground.is_some_and(|(overlay, _)| {
            overlay.base().digest() != scope.map
                || overlay.world_id().as_bytes() != &scope.world
                || overlay.channel_id().map(|id| *id.as_bytes()) != Some(scope.channel)
        }) {
            return None;
        }
        let facts = ElementalViewFacts {
            pinned,
            ground: ground.map(|(_, facts)| facts),
        };
        let snapshot = |position: TilePos, entry: &view::ComposedEntry| {
            if entry.facts.pickupable {
                return None;
            }
            let key = match entry.source {
                view::EntrySource::Base { placement_key, .. } => placement_key,
                view::EntrySource::Added(id) if id.get() >> 62 == 2 => id.get(),
                view::EntrySource::Ground(item)
                    if ground.is_some()
                        && item != [0; 16]
                        && entry.facts.appearance_id == 2126
                        && entry.facts.count == 1 =>
                {
                    ground_readset_key(item)
                }
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
        view::compose_into(
            base,
            ground.and_then(|(overlay, _)| overlay.tile(position)),
            &facts,
            position,
            &mut entries,
        )
        .ok()?;
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
            view::compose_into(
                base,
                ground.and_then(|(overlay, _)| overlay.tile(position)),
                &facts,
                position,
                &mut entries,
            )
            .ok()?;
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

/// Base and callback facts come from one locked snapshot. Ground definitions
/// come from the caller's already rebuilt durable overlay under the item lane.
struct ElementalViewFacts<'a> {
    pinned: &'a dyn MapFacts,
    ground: Option<&'a dyn MapFacts>,
}
impl MapFacts for ElementalViewFacts<'_> {
    fn movement_item(&self, id: u64) -> Option<(TilePos, EntryId)> {
        self.pinned.movement_item(id)
    }
    fn movement_items(&self, pos: TilePos) -> Vec<view::ComposedEntry> {
        self.pinned.movement_items(pos)
    }
    fn base_hidden(&self, key: u64) -> bool {
        self.pinned.base_hidden(key)
    }
    fn base_entry(&self, pos: TilePos, ordinal: u8, id: u32) -> Option<EntryFacts> {
        self.pinned.base_entry(pos, ordinal, id)
    }
    fn added_entry(&self, item: &AddedItem) -> Option<EntryFacts> {
        self.ground
            .and_then(|facts| facts.added_entry(item))
            .or_else(|| self.pinned.added_entry(item))
    }
    fn house_tile(&self, pos: TilePos) -> bool {
        self.pinned.house_tile(pos)
    }
    fn base_text(&self, key: u64) -> Option<&str> {
        self.pinned.base_text(key)
    }
    fn object_revision(&self, key: u64) -> u64 {
        self.pinned.object_revision(key)
    }
}

fn ground_readset_key(item: [u8; 16]) -> u64 {
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"oteryn:elemental-ground-readset:v1\0")
        .chain_update(item)
        .finalize();
    let mut key = [0; 8];
    key.copy_from_slice(&digest[..8]);
    // Prefix 01 cannot alias base placements (00) or callback objects (10/11).
    (u64::from_be_bytes(key) & ((1 << 62) - 1)) | (1 << 62)
}

impl Drop for ElementalArrivalReservation {
    fn drop(&mut self) {
        if let Ok(mut states) = self.callbacks.lock() {
            let states = Arc::make_mut(&mut states);
            for key in &self.keys {
                states.reserved.remove(key);
            }
            for position in &self.positions {
                states.reserved_positions.remove(position);
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
        let base = Arc::new(crate::map::load(&bytes, &pins).map_err(|e| e.to_string())?);
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
        let world = crate::foundation::WorldId::decode(&[
            1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ])
        .map_err(|e| format!("{e:?}"))?;
        let channel = crate::foundation::ChannelId::decode(&[
            1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2,
        ])
        .map_err(|e| format!("{e:?}"))?;
        let scope = ArrivalFence {
            world: *world.as_bytes(),
            channel: *channel.as_bytes(),
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
        let mut ground =
            crate::map::overlay::ChannelOverlay::new(Arc::clone(&base), world, channel);
        for position in recipe.positions {
            ground
                .hide(
                    TilePos {
                        x: position.x,
                        y: position.y,
                        floor: position.floor,
                    },
                    1,
                    crate::map::overlay::Admission::Durable,
                )
                .map_err(|e| format!("{e:?}"))?;
        }
        assert!(
            facts
                .observe_elemental_arrival_with_ground(
                    &base,
                    scope,
                    target,
                    corpse,
                    false,
                    Some((&ground, &facts))
                )
                .is_none(),
            "hidden fields must not charge an altar ring"
        );
        {
            // These are the already-bridged durable Ground rows. This map-layer
            // test issues no SQL authority; the live caller supplies its fenced rebuild.
            struct GroundFacts(EntryFacts);
            impl MapFacts for GroundFacts {
                fn base_entry(&self, _: TilePos, _: u8, _: u32) -> Option<EntryFacts> {
                    None
                }
                fn added_entry(&self, item: &AddedItem) -> Option<EntryFacts> {
                    matches!(item, AddedItem::Ground(item) if item.definition.production_key == "oteryn:item.tibia.i2126" && item.quantity == 1).then_some(self.0)
                }
                fn house_tile(&self, _: TilePos) -> bool {
                    false
                }
                fn base_text(&self, _: u64) -> Option<&str> {
                    None
                }
                fn object_revision(&self, _: u64) -> u64 {
                    0
                }
            }
            let first = recipe.positions[0];
            let first = TilePos {
                x: first.x,
                y: first.y,
                floor: first.floor,
            };
            let mut entries = Vec::new();
            view::compose_into(&base, None, &facts, first, &mut entries)
                .map_err(|e| format!("{e:?}"))?;
            let field = entries
                .iter()
                .find(|entry| entry.facts.appearance_id == 2126)
                .ok_or("field facts")?
                .facts;
            let ground_facts = GroundFacts(field);
            let mut ids = Vec::new();
            for (index, position) in recipe.positions.iter().enumerate() {
                let mut item = [1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
                item[15] = u8::try_from(index + 1).map_err(|e| e.to_string())?;
                let mut spatial = i32::from(position.x).to_be_bytes().to_vec();
                spatial.extend(i32::from(position.y).to_be_bytes());
                spatial.extend(i16::from(position.floor).to_be_bytes());
                ground
                    .add_ground(crate::durability::item_mint::GroundItemInstance {
                        item_instance_id: item,
                        world_id: world,
                        runtime_scope: crate::foundation::RuntimeScopeRefV1::channel(
                            world, channel,
                        ),
                        definition: crate::durability::item_mint::TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: "oteryn:item.tibia.i2126".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        quantity: 1,
                        runtime_scope_ownership_generation: 1,
                        ground: crate::durability::item_mint::GroundPlacement {
                            spatial_position: spatial,
                            corpse_ref: item.to_vec(),
                            map_revision: crate::map::overlay::map_revision(&base),
                            content_revision: base.content_revision().into(),
                            native_room_placement_context: vec![3; 32],
                        },
                        minted_transaction_id: item,
                    })
                    .map_err(|e| format!("{e:?}"))?;
                ids.push(item);
            }
            let plan = facts
                .observe_elemental_arrival_with_ground(
                    &base,
                    scope,
                    target,
                    corpse,
                    false,
                    Some((&ground, &ground_facts)),
                )
                .ok_or("all eight durable fields")?;
            assert!(plan.readset().iter().all(|cell| cell.key >> 62 == 1));
            assert_ne!(plan.readset(), observed.readset());
            let portable = GroundFacts(EntryFacts {
                pickupable: true,
                ..field
            });
            assert!(
                facts
                    .observe_elemental_arrival_with_ground(
                        &base,
                        scope,
                        target,
                        corpse,
                        false,
                        Some((&ground, &portable))
                    )
                    .is_none()
            );
            let mut foreign = scope;
            foreign.channel[15] = 3;
            assert!(
                facts
                    .observe_elemental_arrival_with_ground(
                        &base,
                        foreign,
                        target,
                        corpse,
                        false,
                        Some((&ground, &ground_facts))
                    )
                    .is_none()
            );
            let (at, entry) = ground.ground_entry(&ids[0]).ok_or("first durable field")?;
            let removed = ground.remove(at, entry).map_err(|e| format!("{e:?}"))?;
            let wake = Arc::new(tokio::sync::Notify::new());
            assert!(
                facts
                    .reserve_elemental_arrival_with_ground(
                        &base,
                        &plan,
                        Arc::clone(&wake),
                        Some((&ground, &ground_facts))
                    )
                    .is_none(),
                "a missing field invalidates the retained ring"
            );
            let AddedItem::Ground(item) = removed else {
                return Err("Ground field source".into());
            };
            ground.add_ground(*item).map_err(|e| format!("{e:?}"))?;
            let restored = facts
                .observe_elemental_arrival_with_ground(
                    &base,
                    scope,
                    target,
                    corpse,
                    false,
                    Some((&ground, &ground_facts)),
                )
                .ok_or("restored durable ring")?;
            assert_eq!(
                restored, plan,
                "field identities survive overlay entry renumbering"
            );
            let reservation = facts
                .reserve_elemental_arrival_with_ground(
                    &base,
                    &plan,
                    wake,
                    Some((&ground, &ground_facts)),
                )
                .ok_or("durable field reservation")?;
            let base_key = placement_key(first, 1).ok_or("hidden base field key")?;
            assert!(
                !facts.apply_quest_callback(base_key, 0, field, &[], &[], false, None),
                "a Ground field reservation also freezes its tile's callback objects"
            );
            drop(reservation);
            assert!(facts.quest_callback_capacity(base_key, 0, 0));
        }
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
