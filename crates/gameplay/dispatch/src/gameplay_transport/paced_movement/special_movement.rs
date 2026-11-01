//! Crystal-compatible swimming and trap step hooks on the production paced-movement path.
//!
//! Donor values are from Crystal `data/scripts/movements/swimming.lua` lines 1-3, 20-37 and
//! 46-53, and `data/scripts/movements/trap.lua` lines 1-8, 23-44 and 59-69 at `201189f8`.
use crate::content::ReferenceResistanceKind;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::map::boot::BundleMap;
use crate::map::overlay::TilePos;
use crate::map::view::{EntryFacts, MapFacts, placement_key};
use oteryn_gameplay_domain::domain::appearance::{AppearanceSelection, Colours};
use oteryn_gameplay_domain::domain::condition::{ConditionType, DotElement, StatusKind};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(in crate::gameplay_transport) const SWIMMING_LOOK_TYPE: u32 = 267;
pub(in crate::gameplay_transport) const SWIMMING_GROUND_IDS: [u16; 12] = [
    629, 630, 631, 632, 633, 634, 4809, 4810, 4811, 4812, 4813, 4814,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gameplay_transport) struct Trap {
    pub closed: u16,
    pub minimum_damage: u32,
    pub maximum_damage: u32,
    pub element: ReferenceResistanceKind,
    pub optional_pvp_player_immune: bool,
    pub player_immune: bool,
}

pub(in crate::gameplay_transport) fn trap(id: u16) -> Option<Trap> {
    Some(match id {
        2145 => Trap {
            closed: 2146,
            minimum_damage: 50,
            maximum_damage: 100,
            element: ReferenceResistanceKind::Physical,
            optional_pvp_player_immune: false,
            player_immune: false,
        },
        2147 => Trap {
            closed: 2148,
            minimum_damage: 50,
            maximum_damage: 100,
            element: ReferenceResistanceKind::Physical,
            optional_pvp_player_immune: false,
            player_immune: false,
        },
        3482 => Trap {
            closed: 3481,
            minimum_damage: 15,
            maximum_damage: 30,
            element: ReferenceResistanceKind::Physical,
            optional_pvp_player_immune: true,
            player_immune: false,
        },
        3944 => Trap {
            closed: 3945,
            minimum_damage: 15,
            maximum_damage: 30,
            element: ReferenceResistanceKind::Earth,
            optional_pvp_player_immune: false,
            player_immune: false,
        },
        // Crystal trap.lua:6,31-42: players never trigger the magic wolf trap.
        // Monster stepping is owned by the creature-movement path, not this player owner turn.
        12368 => Trap {
            closed: 12368,
            minimum_damage: 0,
            maximum_damage: 0,
            element: ReferenceResistanceKind::Physical,
            optional_pvp_player_immune: false,
            player_immune: true,
        },
        44529 => Trap {
            closed: 44530,
            minimum_damage: 50,
            maximum_damage: 100,
            element: ReferenceResistanceKind::Physical,
            optional_pvp_player_immune: false,
            player_immune: false,
        },
        _ => return None,
    })
}

/// Crystal `constants.lua:5-6`.
const BATHTUB_FILLED: u16 = 26077;
const BATHTUB_FILLED_NOTMOVABLE: u16 = 26100;
/// `items.xml:18277,18283`: every snow footprint 6594..=6608 lasts 10 s.
const SNOW_FOOTPRINT_DECAY_US: u64 = 10_000_000;

/// `snow.lua:11-15,23`: 799 leaves 6594 and 6580..=6593 leave `id + 15`; each decays back to the
/// snow it came from (`items.xml:18278,18284-18362`).
fn snow_footprint(id: u16) -> Option<u16> {
    match id {
        799 => Some(6594),
        6580..=6593 => Some(id + 15),
        _ => None,
    }
}

fn tile_position(position: MovementLocalPosition) -> Option<TilePos> {
    Some(TilePos {
        x: position.x.try_into().ok()?,
        y: position.y.try_into().ok()?,
        floor: i8::try_from(position.floor).ok()?.checked_neg()?,
    })
}

pub(super) struct TileEntries {
    pub(super) protection_zone: bool,
    pub(super) entries: Vec<(u64, EntryFacts)>,
}

pub(super) fn tile_entries(
    map: &BundleMap,
    position: MovementLocalPosition,
) -> Option<TileEntries> {
    let pos = tile_position(position)?;
    let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
    let mut ordinal = 0_u8;
    let mut appearances = Vec::new();
    for (&id, &depth) in tile.ids().iter().zip(tile.depths()) {
        if depth != 0 {
            continue;
        }
        if let Some(entry) = map.facts.base_entry(pos, ordinal, id) {
            let key = placement_key(pos, ordinal)?;
            if !map.facts.base_hidden(key) {
                appearances.push((key, entry));
            }
        }
        ordinal = ordinal.checked_add(1)?;
    }
    Some(TileEntries {
        protection_zone: tile.protection_zone(),
        entries: appearances,
    })
}

fn transform(map: &BundleMap, key: u64, appearance: u16) -> bool {
    let Some(replacement) = map.facts.quest_callback_replacement(key, appearance) else {
        return false;
    };
    map.facts.apply_quest_callback(
        key,
        map.facts.object_revision(key),
        replacement,
        &[],
        &[],
        false,
    )
}

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    /// Sample the fenced World policy before taking the movement runtime/state guards.
    pub(in crate::gameplay_transport) async fn special_movement_policy(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<bool> {
        self.bundle_map()?;
        use crate::durability::{DurabilityError, spell_item_transaction as item_tx};
        use crate::foundation::{CommandId, CommandRef};
        let pin = {
            let runtime = self.runtime.lock().await;
            runtime.player_control_facts(actor, session).ok()?;
            runtime.content_pin().server_artifact_digest()
        };
        let fence = self.current_quest_fence(session).await.ok()??;
        let command = CommandRef::new(session, CommandId::new(self.owner_now().get().max(1)).ok()?);
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = self;
        pass.run_with_context(&mut context, move |holder, deadline, owner| {
            Box::pin(async move {
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = item_tx::assert_spell_item_authority_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    &fence.item_fence(),
                    command,
                    pin,
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let policy =
                    crate::durability::spell_field_policy::read_world_field_policy_in_transaction(
                        &mut tx,
                        owner.root,
                        authority.runtime_scope(),
                        authority.scope_generation(),
                    )
                    .await?
                    .ok_or(DurabilityError::Unavailable)?;
                let optional = policy.bind(&tx).mode()
                    == crate::durability::spell_field_policy::FieldWorldType::NoPvp;
                tx.rollback().await?;
                Ok(optional)
            })
        })
        .await
        .ok()
    }
}

fn swimming_definition() -> Option<ConditionDefinition> {
    ConditionDefinition::new_permanent_outfit("environment.swimming", 1, SWIMMING_LOOK_TYPE)?
        .with_appearance(AppearanceSelection {
            outfit_key: "environment.swimming".to_owned(),
            colours: Colours::default(),
            addons: 0,
            mount_key: None,
        })
}

impl ChannelSpellStates {
    #[allow(
        clippy::too_many_arguments,
        reason = "one movement owner turn binds both positions and actor authority"
    )]
    pub(in crate::gameplay_transport) fn stage_special_movement_step(
        &mut self,
        runtime: &ChannelRuntimeV1,
        map: &BundleMap,
        actor: ExactActorRef,
        session: GameSessionId,
        now: u64,
        origin: Option<MovementLocalPosition>,
        destination: MovementLocalPosition,
        optional_pvp: Option<bool>,
    ) -> bool {
        let from_swimming = origin
            .and_then(|at| tile_entries(map, at))
            .is_some_and(|tile| {
                tile.entries
                    .iter()
                    .any(|(_, entry)| SWIMMING_GROUND_IDS.contains(&entry.appearance_id))
            });
        let Some(TileEntries {
            protection_zone,
            entries: ids,
        }) = tile_entries(map, destination)
        else {
            return false;
        };
        let Some(current) = self.get(runtime, actor, session).cloned() else {
            return false;
        };
        let mut changed = false;
        // Crystal special_tiles.lua:1-2,6-16 and decay_to.lua:3-15. These map
        // mutations run on the committed player StepIn route.
        for (key, entry) in &ids {
            let transformed = match entry.appearance_id {
                293 | 475 | 1066 => Some(entry.appearance_id + 1),
                419 => Some(420),
                431 => Some(430),
                452 => Some(453),
                563 => Some(564),
                549 => Some(562),
                10145 => Some(10146),
                // bath_tub.lua:3-14: the filled bath tub becomes its not-movable form.
                BATHTUB_FILLED => Some(BATHTUB_FILLED_NOTMOVABLE),
                _ => None,
            };
            if let Some(transformed) = transformed {
                changed |= transform(map, *key, transformed);
            }
        }
        let to_swimming = ids
            .iter()
            .any(|(_, entry)| SWIMMING_GROUND_IDS.contains(&entry.appearance_id));
        let mut next = current.clone();
        if from_swimming {
            next.conditions.remove_type(ConditionType::Outfit);
        }
        if to_swimming {
            // Crystal removes harmful conditions before installing the swimming outfit.
            for condition in [
                ConditionType::DamageOverTime(DotElement::Poison),
                ConditionType::DamageOverTime(DotElement::Fire),
                ConditionType::DamageOverTime(DotElement::Energy),
                ConditionType::DamageOverTime(DotElement::Bleeding),
                ConditionType::DamageOverTime(DotElement::Drown),
                ConditionType::DamageOverTime(DotElement::Freezing),
                ConditionType::DamageOverTime(DotElement::Dazzled),
                ConditionType::DamageOverTime(DotElement::Cursed),
                ConditionType::Paralysis,
                ConditionType::Status(StatusKind::Drunk),
                ConditionType::Outfit,
            ] {
                next.conditions.remove_type(condition);
            }
            if let Some(definition) = swimming_definition() {
                let root = GameplayDecisionRoot::from_bytes(
                    runtime.content_pin().server_artifact_digest(),
                );
                let facts = ApplicationFacts {
                    now,
                    base_speed: 1,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: true,
                    decision_root: &root,
                    occurrence: DecisionOccurrenceId::from_bytes([0x53; 16]),
                };
                let _ = next.conditions.apply(
                    &definition,
                    Some("environment:swimming".to_owned()),
                    ConditionSourceKind::Field,
                    &[],
                    &facts,
                );
            }
        }
        changed |= next != current;
        if next != current {
            let Some(revision) = current.revision().checked_add(1) else {
                return false;
            };
            next.source_set_batch_revision(revision);
            if !self.commit(runtime, actor, session, next) {
                return false;
            }
        }
        // trap.lua:59-69 reopens these four closed spike types on every step-out.
        if let Some(TileEntries {
            protection_zone: origin_protection_zone,
            entries,
        }) = origin.and_then(|at| tile_entries(map, at))
        {
            for (key, entry) in entries {
                let pressure = match entry.appearance_id {
                    420 => Some(419),
                    430 => Some(431),
                    453 => Some(452),
                    564 => Some(563),
                    562 => Some(549),
                    10146 => Some(10145),
                    _ => None,
                };
                if let Some(open) = pressure {
                    changed |= transform(map, key, open);
                }
                // bath_tub.lua:25-31: leaving the not-movable bath tub restores the filled one.
                if entry.appearance_id == BATHTUB_FILLED_NOTMOVABLE {
                    changed |= transform(map, key, BATHTUB_FILLED);
                }
                // snow.lua:3-21: stepping off snow leaves a footprint that decays back after its
                // items.xml duration (:18274-18363). Oteryn has no ghost mode, so the ghost check
                // (snow.lua:9) never returns early; the "Snowbunny" achievement (:19) is not kept.
                if let Some(footprint) = snow_footprint(entry.appearance_id)
                    && transform(map, key, footprint)
                {
                    changed = true;
                    map.facts.schedule_entry_decay(
                        key,
                        map.facts.object_revision(key),
                        now.saturating_add(SNOW_FOOTPRINT_DECAY_US),
                        entry.appearance_id,
                    );
                }
                if !origin_protection_zone
                    && matches!(entry.appearance_id, 2146 | 2148 | 3945 | 44530)
                {
                    transform(map, key, entry.appearance_id - 1);
                }
            }
        }
        // closing_door.lua:58-109: a level or quest door shuts behind its walker.
        if let Some(door) = origin.and_then(tile_position)
            && map.facts.doors().at(door)
        {
            let occupied = map.overlay.has_added_item(door)
                || tile_position(destination).is_none_or(|at| {
                    crate::gameplay_transport::map_door::doorway_occupied(runtime, actor, at, door)
                });
            changed |= map.facts.doors().close_vacated(door, occupied);
        }
        let enabled =
            !protection_zone && ids.iter().filter(|(_, e)| e.appearance_id == 3482).count() <= 1;
        if enabled {
            for (key, entry) in &ids {
                let Some(rule) = trap(entry.appearance_id) else {
                    continue;
                };
                if rule.player_immune {
                    continue;
                }
                // An unavailable policy read cannot grant permission to harm with a jaw trap.
                if rule.optional_pvp_player_immune && optional_pvp != Some(false) {
                    continue;
                }
                // trap.lua:35 transforms before combat (:44), including a blocked hit.
                if !transform(map, *key, rule.closed) {
                    continue;
                }
                changed = true;
                let occurrence = Sha256::new()
                    .chain_update(actor.placement_identity())
                    .chain_update(session.as_bytes())
                    .chain_update(now.to_be_bytes())
                    .chain_update(key.to_be_bytes())
                    .chain_update(map.facts.object_revision(*key).to_be_bytes())
                    .finalize();
                let mut bytes = [0; 16];
                bytes.copy_from_slice(&occurrence[..16]);
                let root = GameplayDecisionRoot::from_bytes(
                    runtime.content_pin().server_artifact_digest(),
                );
                let Ok(draw) = deterministic_decision_u64(
                    &root,
                    DecisionOccurrenceId::from_bytes(bytes),
                    "movement.trap.damage",
                    0,
                ) else {
                    continue;
                };
                let damage = crate::spell::uniform_draw(
                    draw,
                    i64::from(rule.minimum_damage),
                    i64::from(rule.maximum_damage),
                );
                let damage = self
                    .get(runtime, actor, session)
                    .and_then(|state| state.equipped_element_protection(rule.element))
                    .and_then(|protection| {
                        oteryn_gameplay_items::equipment_effects::apply_protection(
                            damage, protection,
                        )
                    })
                    .unwrap_or(damage);
                let Ok(damage) = u32::try_from(damage) else {
                    continue;
                };
                let _ = self.apply_creature_damage_native(
                    runtime,
                    actor,
                    session,
                    damage,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
                );
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_domain::domain::condition::ConditionValues;

    type CaseResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// A bundle row whose tile at x 6 holds `before` (and `after` in the palette), and a player.
    fn movement_map(
        before: u16,
        after: u16,
        protection_zone: bool,
        check: impl FnOnce(
            &mut ChannelSpellStates,
            &ChannelRuntimeV1,
            &BundleMap,
            ExactActorRef,
            GameSessionId,
        ) -> CaseResult,
    ) -> CaseResult {
        use crate::gameplay_transport::actor_spell::tests::{FACTS, runtime_with_player};
        use crate::map::boot::{self, BundleCollisionIndexExt};
        use std::num::NonZeroU32;
        let (mut runtime, actor, session) = runtime_with_player(0x71);
        runtime
            .initialize_first_entry_position(actor)
            .map_err(|e| format!("{e:?}"))?;
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("state")?;
        let keys = (
            format!("oteryn:item.tibia.i{before}"),
            format!(
                "oteryn:item.tibia.i{}",
                if before == after { 100 } else { after }
            ),
        );
        let (bytes, mut pins) = boot::tests::bundle_with_item_ids((&keys.0, 0), (&keys.1, 1));
        let mut bundle = oteryn_world_bundle::writer::read(&bytes)?;
        for tile in &mut bundle.sectors[0].tiles {
            if tile.x == 6 && protection_zone {
                tile.flags = 1;
            }
        }
        let bytes = oteryn_world_bundle::writer::write(
            &bundle.manifest,
            &bundle.sectors,
            &Default::default(),
        )?;
        pins.digest = oteryn_world_bundle::writer::read(&bytes)?.digest;
        let pos = TilePos {
            x: 6,
            y: 0,
            floor: -7,
        };
        let base = crate::map::load(&bytes, &pins).map_err(|e| format!("{e:?}"))?;
        let booted = boot::boot(
            &bytes,
            &boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: pos,
            },
            actor.world_id(),
            actor.channel_id().ok_or("channel")?,
            |key| {
                Some(crate::map::facts::ItemDefinition {
                    reference: NonZeroU32::new(if key == keys.0 { 1 } else { 2 })?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
        .map_err(|e| format!("{e:?}"))?;
        let room = crate::content::qualify_native_entry_room(actor.world_id())
            .map_err(|e| format!("{e:?}"))?;
        let cells = booted
            .movement_cells(room.movement_cells())
            .map_err(|e| format!("{e:?}"))?;
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(collision) =
            cells.index()
        else {
            return Err("bundle collision".into());
        };
        check(&mut states, &runtime, collision.map(), actor, session)
    }

    // Exercise the same bundle projection and actor owner as committed paced movement.
    fn movement_case(before: u16, after: u16, protection_zone: bool, reverses: bool) -> CaseResult {
        movement_map(
            before,
            after,
            protection_zone,
            |states, runtime, map, actor, session| {
                let pos = TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                };
                let at = MovementLocalPosition {
                    x: 6,
                    y: 0,
                    floor: 7,
                };
                let away = MovementLocalPosition { x: 1, ..at };
                let key = placement_key(pos, 1).ok_or("placement")?;
                let current = || map.facts.base_entry(pos, 1, 0).map(|e| e.appearance_id);
                let health = states
                    .get(runtime, actor, session)
                    .ok_or("player")?
                    .vitals()
                    .health;
                let revision = map.facts.object_revision(key);
                assert_eq!(current(), Some(before));
                assert_eq!(
                    states.stage_special_movement_step(
                        runtime,
                        map,
                        actor,
                        session,
                        1,
                        Some(away),
                        at,
                        Some(false)
                    ),
                    before != after
                );
                assert_eq!(current(), Some(after));
                assert_eq!(
                    states
                        .get(runtime, actor, session)
                        .ok_or("player")?
                        .vitals()
                        .health,
                    health
                );
                if before == after {
                    assert_eq!(map.facts.object_revision(key), revision);
                }
                states.stage_special_movement_step(
                    runtime,
                    map,
                    actor,
                    session,
                    2,
                    Some(at),
                    away,
                    Some(false),
                );
                assert_eq!(current(), Some(if reverses { before } else { after }));
                Ok(())
            },
        )
    }

    #[test]
    fn m10_special_tiles_step_in_and_out_include_protection_zones()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                for pz in [false, true] {
                    for (before, after) in [
                        (419, 420),
                        (431, 430),
                        (452, 453),
                        (563, 564),
                        (549, 562),
                        (10145, 10146),
                    ] {
                        movement_case(before, after, pz, true)?;
                    }
                }
                Ok(())
            })?
            .join()
            .map_err(|_| "movement test thread")?
    }

    #[test]
    fn m10_decay_tiles_step_in_without_pressure_step_out()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                for before in [293, 475, 1066] {
                    movement_case(before, before + 1, false, false)?;
                }
                Ok(())
            })?
            .join()
            .map_err(|_| "movement test thread")?
    }

    /// Crystal snow.lua:3-21 and items.xml:18274-18363: a step off snow leaves its footprint,
    /// which decays back to that snow after 10 s; a step onto snow changes nothing.
    #[test]
    fn m10_snow_step_out_leaves_a_footprint_that_decays_after_ten_seconds() -> CaseResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                for (snow, footprint) in [(799, 6594), (6580, 6595), (6593, 6608)] {
                    movement_map(
                        snow,
                        footprint,
                        false,
                        |states, runtime, map, actor, session| {
                            let pos = TilePos {
                                x: 6,
                                y: 0,
                                floor: -7,
                            };
                            let at = MovementLocalPosition {
                                x: 6,
                                y: 0,
                                floor: 7,
                            };
                            let away = MovementLocalPosition { x: 1, ..at };
                            let current =
                                || map.facts.base_entry(pos, 1, 0).map(|e| e.appearance_id);
                            let step = |states: &mut ChannelSpellStates, now, from, to| {
                                states.stage_special_movement_step(
                                    runtime,
                                    map,
                                    actor,
                                    session,
                                    now,
                                    Some(from),
                                    to,
                                    Some(false),
                                )
                            };
                            step(states, 1_000_000, away, at);
                            assert_eq!(current(), Some(snow));
                            assert!(step(states, 2_000_000, at, away));
                            assert_eq!(current(), Some(footprint));
                            assert_eq!(map.facts.pending_entry_decays(), 1);
                            assert!(!map.facts.expire_entry_decays(11_999_999));
                            assert_eq!(current(), Some(footprint));
                            assert!(map.facts.expire_entry_decays(12_000_000));
                            assert_eq!(current(), Some(snow));
                            assert_eq!(map.facts.pending_entry_decays(), 0);
                            Ok(())
                        },
                    )?;
                }
                Ok(())
            })?
            .join()
            .map_err(|_| "movement test thread")?
    }

    /// Crystal bath_tub.lua:3-14,25-31 with constants.lua:5-6: entering the filled bath tub
    /// 26077 turns it into 26100, and leaving 26100 turns it back.
    #[test]
    fn m10_bath_tub_step_in_and_out_transform() -> CaseResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| movement_case(26077, 26100, false, true))?
            .join()
            .map_err(|_| "movement test thread")?
    }

    #[test]
    fn m10_wolf_trap_player_step_never_transforms_or_damages()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| movement_case(12368, 12368, false, false))?
            .join()
            .map_err(|_| "movement test thread")?
    }

    /// Crystal closing_door.lua:58-109 on the committed player step owner: the level door 1646
    /// (action 1001) closes once its walker leaves the threshold; a plain door does not.
    fn door_case(
        closed: u32,
        open: u32,
        action: Option<u16>,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        use crate::gameplay_transport::actor_spell::tests::{FACTS, runtime_with_player};
        use crate::map::boot::{self, BundleCollisionIndexExt};
        use std::num::NonZeroU32;
        let (mut runtime, actor, session) = runtime_with_player(0x72);
        runtime
            .initialize_first_entry_position(actor)
            .map_err(|e| format!("{e:?}"))?;
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("state")?;
        let door = format!("oteryn:item.tibia.i{closed}");
        let (bytes, mut pins) =
            boot::tests::bundle_with_items(&door, &format!("oteryn:item.tibia.i{open}"));
        let mut bundle = oteryn_world_bundle::writer::read(&bytes)?;
        for tile in &mut bundle.sectors[0].tiles {
            if tile.x == 6 {
                tile.items.get_mut(1).ok_or("door")?.attrs.action = action;
            }
        }
        let bytes = oteryn_world_bundle::writer::write(
            &bundle.manifest,
            &bundle.sectors,
            &Default::default(),
        )?;
        pins.digest = oteryn_world_bundle::writer::read(&bytes)?.digest;
        let pos = TilePos {
            x: 6,
            y: 0,
            floor: -7,
        };
        let base = crate::map::load(&bytes, &pins).map_err(|e| format!("{e:?}"))?;
        let booted = boot::boot(
            &bytes,
            &boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: TilePos { x: 1, ..pos },
            },
            actor.world_id(),
            actor.channel_id().ok_or("channel")?,
            |key| {
                let closed = key == door;
                Some(crate::map::facts::ItemDefinition {
                    reference: NonZeroU32::new(if closed { 1 } else { 2 })?,
                    solid: Some(closed),
                    blocks_projectile: closed,
                    pickupable: false,
                    door: closed,
                })
            },
        )
        .map_err(|e| format!("{e:?}"))?;
        let room = crate::content::qualify_native_entry_room(actor.world_id())
            .map_err(|e| format!("{e:?}"))?;
        let cells = booted
            .movement_cells(room.movement_cells())
            .map_err(|e| format!("{e:?}"))?;
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(collision) =
            cells.index()
        else {
            return Err("bundle collision".into());
        };
        let map = collision.map();
        let key = boot::tests::door_key();
        let doors = map.facts.doors();
        if !matches!(
            doors.toggle(key, 0, false, u32::MAX, true),
            Some(crate::map::door::DoorUse::Toggled { open: true, .. })
        ) {
            return Err("door did not open".into());
        }
        let at = MovementLocalPosition {
            x: 6,
            y: 0,
            floor: 7,
        };
        let away = MovementLocalPosition { x: 1, ..at };
        // Stepping onto the threshold never closes it.
        states.stage_special_movement_step(&runtime, map, actor, session, 1, Some(away), at, None);
        assert_eq!(map.facts.object_revision(key), 1);
        states.stage_special_movement_step(&runtime, map, actor, session, 2, Some(at), away, None);
        let closed = doors.blocks(pos);
        assert_eq!(map.facts.object_revision(key), if closed { 2 } else { 1 });
        Ok(closed)
    }

    #[test]
    fn m06_level_door_closes_behind_its_walker_and_a_plain_door_stays_open()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                // Level door 1646/1647 with Crystal action 1001 (level 1).
                assert!(door_case(1646, 1647, Some(1001))?);
                // A plain door (1629/1630) stays open after its walker leaves.
                assert!(!door_case(1629, 1630, None)?);
                Ok(())
            })?
            .join()
            .map_err(|_| "door test thread")?
    }

    #[test]
    fn every_crystal_trap_has_transform_damage_element_and_optional_pvp_rule() {
        assert_eq!(
            trap(2145).map(|r| (r.closed, r.minimum_damage, r.maximum_damage)),
            Some((2146, 50, 100))
        );
        assert_eq!(trap(2147).map(|r| r.closed), Some(2148));
        assert!(trap(3482).is_some_and(|r| r.optional_pvp_player_immune));
        assert!(
            trap(3944)
                .is_some_and(|r| r.element == ReferenceResistanceKind::Earth && r.closed == 3945)
        );
        assert!(trap(12368).is_some_and(|r| r.player_immune));
        assert_eq!(trap(44529).map(|r| r.closed), Some(44530));
    }

    #[test]
    fn swimming_uses_crystals_permanent_look_267() {
        assert_eq!(
            swimming_definition().and_then(|d| match d.values() {
                ConditionValues::Outfit { look_type, .. } => Some(look_type),
                _ => None,
            }),
            Some(267)
        );
    }
}
