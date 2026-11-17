//! Projects the shared World owner through the existing home-spawn owner.
use super::*;
use oteryn_gameplay_durability::durability::world_event::changes::counters;
use oteryn_gameplay_entities::carrier::WorldSpawnProjection;
use oteryn_gameplay_world_change::daily::DailyWorldChanges;

pub(in crate::node) fn project(
    runtime: &mut ChannelRuntimeV1,
    day: &DailyWorldChanges,
) -> Result<(), crate::foundation::CarrierError> {
    if !day.world_counters {
        return Ok(());
    }
    let bindings = counters::bindings()
        .map_err(|_| crate::foundation::CarrierError::InvalidSpawnDefinition)?;
    let mut profiles = std::collections::BTreeMap::new();
    for rule in bindings
        .spawn_rules
        .iter()
        .filter(|rule| oteryn_gameplay_world_change::owner::counter_admitted(rule.kind))
    {
        for key in rule.stages.iter().flatten() {
            let policy = runtime.companion_policy(key)?;
            profiles.insert(
                key.as_str(),
                (policy.maximum_health, policy.blocked_by_nearby_players),
            );
        }
    }
    use oteryn_gameplay_world::terrain_tools;
    use oteryn_gameplay_world_change::counter::CounterKind;
    let voice = CounterKind::TheirMastersVoice.index();
    let draw = |ordinal: u8| {
        let mut hash = Sha256::new();
        hash.update(b"oteryn.world_change.voice.wave.v1\0");
        hash.update(runtime.binding().runtime_scope().world_id().as_bytes());
        hash.update(day.counter_generations[voice].to_be_bytes());
        hash.update([ordinal]);
        let bytes: [u8; 32] = hash.finalize().into();
        u64::from_be_bytes(bytes[..8].try_into().unwrap_or([0; 8]))
    };
    let servants = terrain_tools::voice_servants(draw);
    let mage = terrain_tools::voice_mage(draw(255));
    for key in [
        "oteryn:creature.diamond_servant",
        "oteryn:creature.golden_servant",
        "oteryn:creature.iron_servant",
        "oteryn:creature.mad_mage",
    ] {
        if oteryn_gameplay_world_change::owner::counter_admitted(CounterKind::TheirMastersVoice) {
            let policy = runtime.companion_policy(key)?;
            profiles.insert(
                key,
                (policy.maximum_health, policy.blocked_by_nearby_players),
            );
        }
    }
    runtime.project_world_spawn_cohorts(|base, at, cohort| {
        if let Some((key, _)) = cohort.filter(|(key, _)| key.starts_with("their_masters_voice.")) {
            let home = (at.x, at.y, at.floor);
            let target = if key == "their_masters_voice.mage" {
                (day.their_masters_voice && day.counter_stages[voice] == 3 && home == mage.1)
                    .then_some(mage.0)
            } else {
                terrain_tools::voice_servant_ordinal(home).and_then(|ordinal| {
                    (day.their_masters_voice
                        && day.counter_stages[voice] == 2
                        && day.counter_progress[voice][2] & (1 << ordinal) == 0)
                        .then_some(servants[usize::from(ordinal)].0)
                })
            };
            return Some(WorldSpawnProjection {
                change_key: if key == "their_masters_voice.mage" {
                    key.to_owned()
                } else {
                    format!(
                        "their_masters_voice.wave.{}",
                        day.counter_progress[voice][1]
                    )
                },
                generation: day.counter_generations[voice],
                creature: target.and_then(|key| {
                    profiles
                        .get(key)
                        .map(|&(health, blocked)| (key.to_owned(), health, blocked))
                }),
            });
        }

        let rule = bindings.spawn_rules.iter().find(|rule| {
            let [x0, x1, y0, y1, z0, z1] = rule.area;
            rule.creature == base
                && (x0..=x1).contains(&at.x)
                && (y0..=y1).contains(&at.y)
                && (z0..=z1).contains(&i32::from(at.floor))
        })?;
        if !oteryn_gameplay_world_change::owner::counter_admitted(rule.kind) {
            return None;
        }
        let target = rule
            .stages
            .get(usize::from(day.counter_stages[rule.kind.index()]))?
            .as_deref();
        let creature = target.and_then(|key| {
            profiles
                .get(key)
                .map(|&(health, blocked)| (key.to_owned(), health, blocked))
        });
        Some(WorldSpawnProjection {
            change_key: rule.kind.key().to_owned(),
            generation: day.counter_generations[rule.kind.index()],
            creature,
        })
    })?;
    runtime.install_world_change_cycles(
        day.reset_epoch,
        counters::CounterKind::ALL
            .into_iter()
            .map(|kind| (kind.key().to_owned(), day.counter_generations[kind.index()]))
            .collect(),
    )?;
    Ok(())
}

pub(super) async fn observe(
    scheduler: &RaidScheduler<'_, '_, '_>,
    generation: ScopeOwnershipGeneration,
) {
    let scope = crate::foundation::RuntimeScopeRefV1::channel(scheduler.world, scheduler.channel);
    // Database fences are acquired before the brief synchronous runtime projection.
    // A busy Channel retries on the next supervision tick; no DB await holds its mutex.
    let mut context = (scheduler.runtime, scheduler.map, generation);
    let result = scheduler
        .root
        .project_world_change_counters(
            scheduler.character,
            scheduler.node,
            scope,
            generation,
            scheduler.reset_epoch,
            &mut context,
            |context, day| {
                let mut runtime = context
                    .0
                    .try_lock()
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                if runtime.binding().scope_generation() != context.2
                    || runtime.owner_fence().is_err()
                {
                    return Err(crate::durability::DurabilityError::Unavailable);
                }
                project(&mut runtime, day)
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                if let Some(map) = context.1 {
                    project_map(map, day)?;
                }
                Ok(())
            },
        )
        .await;
    if let Err(error) = result {
        eprintln!("event=world_change_projection state=retry error={error}");
    }
}

pub(in crate::node) fn project_map_parts(
    facts: &crate::map::facts::BundleFacts,
    base: &crate::map::WorldBase,
    day: &DailyWorldChanges,
) -> Result<(), crate::durability::DurabilityError> {
    use crate::map::view::MapFacts;
    use oteryn_gameplay_quests::quest::{
        deepling_boss_access, fire_feathered_native as serpent, shared_world_progress as shared,
    };
    use oteryn_gameplay_world_change::{counter::CounterKind as K, owner::counter_admitted};
    let refused = || crate::durability::DurabilityError::InvalidStoredState;
    if !day.world_counters {
        return Ok(());
    }
    // World decisions cover every Channel, including regional bundles. Only
    // project physical sources on tiles this Channel actually loaded; a loaded
    // source still has to satisfy its exact appearance/revision checks.
    if counter_admitted(K::MagesTower)
        && base.tile(33143, 31527, -2).is_some()
        && !facts.project_mage_tower_entry(base, day.counter_stages[K::MagesTower.index()] == 4)
    {
        return Err(refused());
    }
    let install = |position: oteryn_gameplay_quests::quest::world_callbacks::Position,
                   appearance: u16,
                   active: bool| {
        let pos = crate::map::overlay::TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        if base.tile(pos.x, pos.y, pos.floor).is_none() {
            return Ok(());
        }
        let z = u8::try_from(-i16::from(pos.floor)).map_err(|_| refused())?;
        let key = oteryn_gameplay_items::event_planting::world_change_item_key(
            appearance, pos.x, pos.y, z,
        )
        .ok_or_else(refused)?;
        if active {
            if !facts.create_world_change_callback_item(pos, appearance) {
                return Err(refused());
            }
        } else if facts.movement_item(key).is_some()
            && !facts.remove_world_change_callback_item(key, facts.object_revision(key))
        {
            return Err(refused());
        }
        Ok(())
    };
    for callback in shared::callbacks() {
        let hive = callback.quest == shared::HIVE_QUEST;
        let kind = if hive { K::HiveBorn } else { K::Deeplings };
        if !counter_admitted(kind) {
            continue;
        }
        let position = callback.selector.position.ok_or_else(refused)?;
        let appearance = callback.selector.appearance.ok_or_else(refused)?;
        let active = if hive {
            true
        } else {
            let binding = shared::binding(&callback.key).ok_or_else(refused)?;
            let bit = shared::BINDINGS
                .iter()
                .filter(|binding| !matches!(binding.action, shared::SharedAction::HiveTask(_)))
                .position(|source| source.key == binding.key)
                .ok_or_else(refused)?;
            let stage = day.counter_stages[kind.index()];
            let eligible = match binding.action {
                shared::SharedAction::HeartDestroyed | shared::SharedAction::SoulNetDestroyed => {
                    stage == 1
                }
                shared::SharedAction::CoralPathCompleted
                | shared::SharedAction::StairPathCompleted => stage == 2,
                _ => false,
            };
            eligible && day.shared_used_sources_mask & (1 << bit) == 0
        };
        install(position, appearance, active)?;
    }
    if counter_admitted(K::Deeplings) {
        for binding in deepling_boss_access::BINDINGS {
            install(
                binding.position,
                binding.appearance,
                day.counter_stages[K::Deeplings.index()] == 3,
            )?;
        }
    }
    if counter_admitted(K::FireFeathered) {
        for callback in serpent::callbacks() {
            install(
                callback.selector.position.ok_or_else(refused)?,
                callback.selector.appearance.ok_or_else(refused)?,
                day.counter_stages[K::FireFeathered.index()] == 3,
            )?;
        }
    }
    Ok(())
}
fn project_map(
    map: &crate::map::boot::BundleMap,
    day: &DailyWorldChanges,
) -> Result<(), crate::durability::DurabilityError> {
    project_map_parts(&map.facts, map.overlay.base(), day)
}

#[cfg(test)]
mod tests {
    //! A persisted World epoch also projects into Channels serving a regional bundle.
    #[test]
    fn node_boot_world_projection_replays_on_a_regional_bundle()
    -> Result<(), Box<dyn std::error::Error>> {
        let (bytes, pins) = crate::map::boot::tests::bundle();
        let map_revision = crate::map::overlay::map_revision(&crate::map::load(&bytes, &pins)?);
        let world = crate::foundation::WorldId::decode(&[
            1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ])?;
        let channel = crate::foundation::ChannelId::decode(&[
            1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2,
        ])?;
        let booted = crate::map::boot::check(
            bytes,
            crate::map::boot::BootPins {
                map_revision,
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 3,
                    y: 0,
                    floor: -7,
                },
            },
        )?
        .boot(world, channel, crate::map::boot::tests::items)?;
        let mut day =
            oteryn_gameplay_world_change::daily::roll_calendar(world.as_bytes(), 0, 6, 13);
        day.world_counters = true;
        for stage in 0..=4 {
            day.counter_stages
                [oteryn_gameplay_world_change::counter::CounterKind::MagesTower.index()] = stage;
            for _ in 0..2 {
                super::project_map_parts(booted.facts(), booted.base(), &day)?;
            }
        }
        Ok(())
    }

    #[test]
    fn node_boot_world_projection_rejects_a_malformed_loaded_mage_entry()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_world_bundle::{bundle, sector};
        let (bytes, mut pins) = crate::map::boot::tests::bundle();
        let mut manifest = bundle::read(&bytes)?.manifest;
        manifest.world = bundle::Extent {
            min_x: 33143,
            max_x: 33144,
            min_y: 31527,
            max_y: 31528,
            floors: vec![-2],
        };
        let bytes = oteryn_world_bundle::writer::write(
            &manifest,
            &[bundle::Sector {
                floor: -2,
                sx: 33143 / 32,
                sy: 31527 / 32,
                tiles: vec![sector::Tile {
                    x: 33143,
                    y: 31527,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    // A loaded tower tile without its required 10840 entry is corrupt.
                    items: vec![sector::Item {
                        palette: 0,
                        depth: 0,
                        attrs: Default::default(),
                    }],
                }],
            }],
            &Default::default(),
        )?;
        pins.digest = bundle::read(&bytes)?.digest;
        let map_revision = crate::map::overlay::map_revision(&crate::map::load(&bytes, &pins)?);
        let world = crate::foundation::WorldId::decode(&[
            1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ])?;
        let channel = crate::foundation::ChannelId::decode(&[
            1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2,
        ])?;
        let booted = crate::map::boot::check(
            bytes,
            crate::map::boot::BootPins {
                map_revision,
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 33143,
                    y: 31527,
                    floor: -2,
                },
            },
        )?
        .boot(world, channel, crate::map::boot::tests::items)?;
        let mut day =
            oteryn_gameplay_world_change::daily::roll_calendar(world.as_bytes(), 0, 6, 13);
        day.world_counters = true;
        assert!(super::project_map_parts(booted.facts(), booted.base(), &day,).is_err());
        Ok(())
    }
}
