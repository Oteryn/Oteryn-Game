//! Crystal movement scripts on the committed production paced-movement path.
//!
//! Donor values are from Crystal `data/scripts/movements/swimming.lua` lines 1-3, 20-37 and
//! 46-53, `trap.lua` lines 1-8, 23-44 and 59-69, `yellow_pillow.lua` lines 3-10,
//! `bath_tub.lua` lines 3-17,25-33 and `snow.lua` lines 3-23 at `201189f8`.
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
    appearances.extend(
        map.facts
            .movement_items(pos)
            .into_iter()
            .filter_map(|entry| match entry.source {
                crate::map::view::EntrySource::Added(id) if id.get() >> 62 == 2 => {
                    Some((id.get(), entry.facts))
                }
                _ => None,
            }),
    );
    Some(TileEntries {
        protection_zone: tile.protection_zone(),
        entries: appearances,
    })
}

pub(super) fn transform(map: &BundleMap, key: u64, appearance: u16) -> bool {
    if key >> 62 == 2 {
        return map.facts.compare_transform_movement_item(
            key,
            map.facts.object_revision(key),
            appearance,
        );
    }
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
        None,
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

    pub(in crate::gameplay_transport) async fn add_snowbunny_progress(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        session: GameSessionId,
        event_id: [u8; 16],
    ) -> bool {
        use crate::durability::{DurabilityError, spell_item_transaction as item_tx};
        use crate::foundation::{CommandId, CommandRef};
        let pin = {
            let runtime = self.runtime.lock().await;
            runtime.content_pin().server_artifact_digest()
        };
        let Some(fence) = self.current_quest_fence(session).await.ok().flatten() else {
            return false;
        };
        let Ok(command_id) = CommandId::new(self.owner_now().get().max(1)) else {
            return false;
        };
        let command = CommandRef::new(session, command_id);
        let character_id = fence.character_id;
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return false;
        };
        let mut context = self;
        pass.run_with_context(&mut context, move |holder, deadline, owner| {
            Box::pin(async move {
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                item_tx::assert_spell_item_authority_in_transaction(
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
                // The authority read and Snowbunny +1 share this transaction and its only
                // commit. Crystal donor: data/scripts/movements/snow.lua:13-20
                // (commit 201189f871d23597b82caf88fd611454a2ddefb8).
                crate::durability::account_achievement::commit_snowbunny_progress(
                    tx,
                    character_id,
                    &event_id,
                    deadline,
                )
                .await?;
                Ok(())
            })
        })
        .await
        .is_ok()
    }
}

pub(in crate::gameplay_transport) fn snow_at(
    map: &BundleMap,
    position: MovementLocalPosition,
) -> bool {
    tile_entries(map, position).is_some_and(|tile| {
        tile.entries
            .iter()
            .any(|(_, entry)| snow_transform(entry.appearance_id, true, false).is_some())
    })
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

/// Crystal yellow_pillow.lua:3-10 and utils_definitions.hpp:390 (MONSTER_SAY = 36).
fn pillow_speech(id: u16, player: bool, ghost: bool) -> Option<(&'static str, u8, u16)> {
    (id == 2397 && player && !ghost).then_some(("Faaart!", 36, 3))
}

fn snow_transform(id: u16, player: bool, ghost: bool) -> Option<u16> {
    if !player || ghost {
        return None;
    }
    match id {
        799 => Some(6594),
        6580..=6593 => Some(id + 15),
        _ => None,
    }
}

fn bath_definition(runtime: &ChannelRuntimeV1, map: &BundleMap) -> Option<ConditionDefinition> {
    // Boot resolves this Item through the active served definition index, including unplaced items.
    let item = map.facts.movement_item_facts(26087)?;
    if !matches!(
        item.definition,
        oteryn_protocol_oteryn::world_map::MapDefinition::Item(_)
    ) {
        return None;
    }
    ConditionDefinition::new_permanent_item_outfit("environment.bath", 1)?.with_item_appearance(
        "oteryn:item.tibia.i26087",
        "definition-r1",
        runtime.content_pin().server_artifact_digest(),
    )
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
        if map.overlay.runtime_scope() != runtime.binding().runtime_scope() {
            return false;
        }
        let origin_entries = origin.and_then(|at| tile_entries(map, at));
        let from_swimming = origin_entries.as_ref().is_some_and(|tile| {
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
        for (_, entry) in &ids {
            // This path has proved a current player. The current actor model admits no
            // ghost mode; invisibility is deliberately not treated as ghost mode.
            if let Some((text, talk_type, effect)) = pillow_speech(entry.appearance_id, true, false)
                && let Some(position) = tile_position(destination)
            {
                changed |= map
                    .facts
                    .emit_movement_speech(crate::map::facts::MovementSpeech {
                        speaker: actor,
                        position,
                        talk_type,
                        text: text.to_owned(),
                        captured_name: None,
                    });
                changed |= map.facts.emit_movement_effect(position, effect);
            }
        }
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
        // Crystal movements_crate.lua's final source cell removes CONDITION_OUTFIT
        // unconditionally, revealing the selected base without changing other families.
        if let Some(rules) = oteryn_gameplay_quests::quest::item_disguises::rules()
            && let Some(rule) = rules.iter().find(|rule| {
                rule.restore.is_some_and(|restore| {
                    i32::from(restore.x) == destination.x
                        && i32::from(restore.y) == destination.y
                        && -i16::from(restore.floor) == destination.floor
                })
            })
            && next.conditions.remove_type(ConditionType::Outfit)
            && let Some(position) = tile_position(destination)
        {
            changed |= map
                .facts
                .emit_movement_speech(crate::map::facts::MovementSpeech {
                    speaker: actor,
                    position,
                    talk_type: rule.speech_type,
                    text: rule.restore_text.to_owned(),
                    captured_name: None,
                });
        }
        let from_bath = origin_entries.as_ref().is_some_and(|tile| {
            tile.entries
                .iter()
                .any(|(_, entry)| entry.appearance_id == 26100)
        });
        if from_swimming || from_bath {
            next.conditions.remove_type(ConditionType::Outfit);
        }
        if to_swimming {
            // Crystal data/scripts/movements/swimming.lua:24 emits CONST_ME_WATERSPLASH (53).
            if let Some(position) = tile_position(destination) {
                changed |= map.facts.emit_movement_effect(position, 53);
            }
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
        let mut enter_bath = false;
        if ids.iter().any(|(_, entry)| entry.appearance_id == 26077)
            && map.facts.movement_item_facts(26100).is_some()
            && let Some(definition) = bath_definition(runtime, map)
        {
            next.conditions.remove_type(ConditionType::Outfit);
            let root =
                GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
            let facts = ApplicationFacts {
                now,
                base_speed: 1,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes([0x42; 16]),
            };
            enter_bath = next
                .conditions
                .apply(
                    &definition,
                    Some("environment:bath".to_owned()),
                    ConditionSourceKind::Field,
                    &[],
                    &facts,
                )
                .is_ok();
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
        // Crystal bath_tub.lua:25-33 StepOut precedes StepIn, including adjacent tubs.
        if let Some(tile) = &origin_entries {
            for (key, entry) in &tile.entries {
                // Crystal snow.lua:3-23. No current actor can enter ghost mode; the
                // rule keeps that distinct from ordinary invisible player conditions.
                if let Some(appearance) = snow_transform(entry.appearance_id, true, false)
                    && map
                        .facts
                        .start_movement_decay(*key, *entry, appearance, now, 10_000_000)
                {
                    changed = true;
                    self.owner_wake.notify_one();
                    // TODO(M10-MOVE): fenced, idempotent Snowbunny +1/10000 progress.
                    // durability/account_achievement.rs:add_progress is transaction-private.
                }
                if entry.appearance_id == 26100 {
                    changed |= map.facts.transform_movement_item(*key, 26077);
                }
            }
        }
        if enter_bath {
            for (key, entry) in &ids {
                if entry.appearance_id == 26077 {
                    changed |= map.facts.transform_movement_item(*key, 26100);
                }
            }
            if let Some(pos) = tile_position(destination) {
                map.facts.emit_movement_effect(pos, 54);
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
                if !origin_protection_zone
                    && matches!(entry.appearance_id, 2146 | 2148 | 3945 | 44530)
                {
                    transform(map, key, entry.appearance_id - 1);
                }
            }
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
                // Crystal data/scripts/movements/trap.lua:36 emits CONST_ME_HITAREA (11).
                if let Some(position) = tile_position(destination) {
                    changed |= map.facts.emit_movement_effect(position, 11);
                }
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

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    /// M06-CLOSE: run while the successful step still owns the Channel lane. The runtime
    /// census is taken after leaving; durable item moves finish before the door transition.
    pub(super) async fn close_departed_door(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementLocalPosition,
        movement_revision: u64,
    ) {
        use crate::gameplay_transport::{connection::UseCommand, item_view::ItemKey, loop_move};
        use crate::map::door::{DoorStepOut, DoorTileItem, door_step_out};
        use crate::map::overlay::AddedItem;
        let (Some(map), Some(position)) = (self.bundle_map(), tile_position(origin)) else {
            return;
        };
        let Some(entries) = tile_entries(map, origin) else {
            return;
        };
        let doors: Vec<_> = entries
            .entries
            .iter()
            .filter(|(key, facts)| {
                map.facts.doors().contains(*key)
                    && matches!(
                        door_step_out::<ItemKey>(facts.appearance_id, position, false, true, &[]),
                        DoorStepOut::Close { .. }
                    )
            })
            .copied()
            .collect();
        if doors.is_empty() {
            return;
        }
        let (index, east_enterable) = {
            let runtime = self.runtime.lock().await;
            if !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return;
            }
            let Ok(at) = runtime.read_actor_position(actor) else {
                return;
            };
            let Some(at) = tile_position(at.position()) else {
                return;
            };
            if crate::gameplay_transport::map_door::doorway_occupied(&runtime, actor, at, position)
            {
                return;
            }
            let east = position.x.checked_add(1).map(|x| TilePos { x, ..position });
            let east_enterable = east.is_some_and(|east| {
                self.movement_cells()
                    .index()
                    .lookup(
                        self.movement_cells().scope(),
                        crate::content::LogicalCell {
                            x: i32::from(east.x),
                            y: i32::from(east.y),
                            z: -i32::from(east.floor),
                        },
                    )
                    .ok()
                    == Some(crate::content::CollisionClass::Walkable)
                    && !map.facts.doors().blocks(east)
                    && !crate::gameplay_transport::map_door::doorway_occupied(
                        &runtime, actor, at, east,
                    )
            });
            (
                crate::gameplay_transport::item_ref_admission::pinned_index(
                    &runtime,
                    self.active_generation,
                ),
                east_enterable,
            )
        };
        let Some(index) = index else {
            return;
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return;
        };
        let movable = |facts: &EntryFacts| {
            use crate::content::ReferenceItemField;
            let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) =
                facts.definition
            else {
                return false;
            };
            let Some((key, revision)) = index.definition_at(reference) else {
                return false;
            };
            native.item_policy(key, revision).is_some_and(|policy| {
                matches!(&policy.record().semantics.physical, ReferenceItemField::Known(physical) if physical.movable == ReferenceItemField::Known(true))
            })
        };
        let Some(ground) = loop_move::loop_ground(self, map.overlay.base(), Some(index)).await
        else {
            return;
        };
        let mut items = Vec::new();
        for (key, facts) in &entries.entries {
            if map.facts.doors().contains(*key) || !movable(facts) {
                continue;
            }
            let Some((_, ordinal)) = crate::map::view::placement_of(*key) else {
                return;
            };
            if ground
                .overlay()
                .tile(position)
                .is_some_and(|tile| tile.is_hidden(ordinal))
            {
                continue;
            }
            items.push(DoorTileItem {
                item: ItemKey::MapBase {
                    bundle_digest: map.overlay.base().digest(),
                    placement_key: *key,
                    reset_epoch: self.reset_epoch,
                },
                movable: true,
                corpse: false,
            });
        }
        if let Some(tile) = ground.overlay().tile(position) {
            for entry in tile.added() {
                let AddedItem::Ground(item) = entry.item() else {
                    continue;
                };
                // Corpses have a separate runtime/custody owner and never enter this loop-drop
                // projection. Temporal fields are immovable and are left on their own lane.
                let Some(facts) = ground.facts(&map.facts).added_entry(entry.item()) else {
                    return;
                };
                items.push(DoorTileItem {
                    item: ItemKey::Instance(item.item_instance_id),
                    movable: facts.pickupable && movable(&facts),
                    corpse: false,
                });
            }
        }
        for (key, facts) in doors {
            let revision = map.facts.doors().revision(key);
            let DoorStepOut::Close {
                item_moves,
                closed_id: _,
            } = door_step_out(facts.appearance_id, position, false, east_enterable, &items)
            else {
                continue;
            };
            let fence = if item_moves.is_empty() {
                None
            } else {
                let Ok(Some(fence)) = self.current_quest_fence(session).await else {
                    return;
                };
                Some(fence.item_fence())
            };
            for (item, to) in item_moves {
                let mut hasher = Sha256::new()
                    .chain_update(b"oteryn:door-step-out:v1")
                    .chain_update(actor.placement_identity())
                    .chain_update(self.reset_epoch.to_be_bytes())
                    .chain_update(movement_revision.to_be_bytes())
                    .chain_update(key.to_be_bytes());
                match item {
                    ItemKey::Instance(id) => hasher.update(id),
                    ItemKey::MapBase {
                        bundle_digest,
                        placement_key,
                        ..
                    } => {
                        hasher.update(bundle_digest);
                        hasher.update(placement_key.to_be_bytes());
                    }
                    _ => return,
                }
                let digest = hasher.finalize();
                let mut id = [0; 8];
                id.copy_from_slice(&digest[..8]);
                let command = UseCommand {
                    game_session_id: session,
                    command_id: u64::from_be_bytes(id).max(1),
                    item_fence: fence,
                };
                let result =
                    loop_move::relocate_door_item(self, permit, actor, command, item, position, to)
                        .await;
                if result != crate::gameplay_transport::item_move::LoopMoveResult::Committed {
                    return;
                }
            }
            let runtime = self.runtime.lock().await;
            if !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return;
            }
            let Ok(at) = runtime.read_actor_position(actor) else {
                return;
            };
            let Some(at) = tile_position(at.position()) else {
                return;
            };
            let occupied = crate::gameplay_transport::map_door::doorway_occupied(
                &runtime, actor, at, position,
            );
            // Same CAS/revision/cache invalidation as explicit USE. Closing requires no
            // level/storage predicate: only opening is gated by those donor requirements.
            if !matches!(
                map.facts
                    .doors()
                    .toggle(key, revision, occupied, u32::MAX, true),
                Some(crate::map::door::DoorUse::Toggled { open: false, .. })
            ) {
                return;
            }
            items.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_domain::domain::condition::ConditionValues;

    // Exercise the same bundle projection and actor owner as committed paced movement.
    fn movement_case(
        before: u16,
        after: u16,
        protection_zone: bool,
        reverses: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            if tile.x == 6 && before == 26077 {
                tile.house = 42;
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
                    reference: NonZeroU32::new(if key == keys.0 {
                        1
                    } else if key == keys.1 {
                        2
                    } else if (before == 26077 && key == "oteryn:item.tibia.i26087")
                        || ((before == 799 || (6580..=6593).contains(&before))
                            && key
                                == format!(
                                    "oteryn:item.tibia.i{}",
                                    if before == 799 { 6594 } else { before + 15 }
                                ))
                    {
                        3
                    } else {
                        return None;
                    })?,
                    solid: Some(false),
                    // Model an unplaced footprint with conservative unknown physical metadata.
                    blocks_projectile: (before == 799 || (6580..=6593).contains(&before))
                        && key
                            == format!(
                                "oteryn:item.tibia.i{}",
                                if before == 799 { 6594 } else { before + 15 }
                            ),
                    pickupable: before == 26077 && key == keys.0,
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
        let map = collision.map();
        let at = MovementLocalPosition {
            x: 6,
            y: 0,
            floor: 7,
        };
        let away = MovementLocalPosition { x: 1, ..at };
        let key = placement_key(pos, 1).ok_or("placement")?;
        let current = || map.facts.base_entry(pos, 1, 0).map(|e| e.appearance_id);
        let health = states
            .get(&runtime, actor, session)
            .ok_or("player")?
            .vitals()
            .health;
        let revision = map.facts.object_revision(key);
        assert_eq!(current(), Some(before));
        assert_eq!(
            states.stage_special_movement_step(
                &runtime,
                map,
                actor,
                session,
                1,
                Some(away),
                at,
                Some(false)
            ),
            // Crystal yellow_pillow.lua:9-10 changes presentation with an unchanged item.
            before != after || before == 2397
        );
        assert_eq!(current(), Some(after));
        if trap(before).is_some_and(|rule| !rule.player_immune) && !protection_zone {
            assert_eq!(
                map.facts
                    .movement_effects_since(0)
                    .iter()
                    .map(|(_, _, effect)| *effect)
                    .collect::<Vec<_>>(),
                vec![11],
                "Crystal trap.lua:36 effect reaches the movement occurrence stream"
            );
        }
        if before == 26077 {
            assert!(map.facts.house_tile(pos), "bath tubs are house furniture");
            let state = states.get(&runtime, actor, session).ok_or("state")?;
            use oteryn_gameplay_foundation::foundation::condition::TemporaryDisplayedAppearance;
            assert!(
                matches!(state.owned_conditions().displayed_temporary_appearance_at(60_000_000),
                Some(TemporaryDisplayedAppearance::Item(item)) if item.definition_key() == "oteryn:item.tibia.i26087")
            );
            assert!(!map.facts.base_entry(pos, 1, 0).ok_or("tub")?.pickupable);
            assert_eq!(
                map.facts
                    .movement_effects_since(0)
                    .iter()
                    .map(|(_, _, effect)| *effect)
                    .collect::<Vec<_>>(),
                vec![54]
            );
        }
        if before == 2397 {
            let speech = map.facts.movement_speech_since(0);
            assert_eq!(speech.len(), 1);
            assert_eq!(
                (
                    speech[0].1.speaker,
                    speech[0].1.text.clone(),
                    speech[0].1.talk_type
                ),
                (actor, "Faaart!".to_owned(), 36)
            );
            assert_eq!(
                map.facts
                    .movement_effects_since(0)
                    .iter()
                    .map(|(_, _, effect)| *effect)
                    .collect::<Vec<_>>(),
                vec![3]
            );
        }
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("player")?
                .vitals()
                .health,
            health
        );
        if before == after {
            assert_eq!(map.facts.object_revision(key), revision);
        }
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            2,
            Some(at),
            away,
            Some(false),
        );
        if before == 799 || (6580..=6593).contains(&before) {
            assert_eq!(
                current(),
                Some(if before == 799 { 6594 } else { before + 15 }),
                "Crystal snow.lua:14-17 transforms only on StepOut"
            );
            assert!(
                !map.facts
                    .base_entry(pos, 1, 0)
                    .ok_or("snow")?
                    .blocks_projectile,
                "footprints retain the original ground's projectile policy"
            );
            assert_eq!(map.facts.next_movement_decay_time(), Some(10_000_002));
            assert_eq!(map.facts.expire_movement_items(10_000_001), 0);
            assert_eq!(
                current(),
                Some(if before == 799 { 6594 } else { before + 15 })
            );
            assert_eq!(map.facts.expire_movement_items(10_000_002), 1);
            assert_eq!(current(), Some(before));
            assert_eq!(map.facts.next_movement_decay_time(), None);
            assert_eq!(map.facts.expire_movement_items(10_000_002), 0);
            states.stage_special_movement_step(
                &runtime,
                map,
                actor,
                session,
                20_000_000,
                Some(at),
                away,
                Some(false),
            );
            assert!(map.facts.transform_movement_item(key, before));
            let revision = map.facts.object_revision(key);
            assert_eq!(
                map.facts.expire_movement_items(30_000_000),
                0,
                "stale decay cannot overwrite a later transform"
            );
            assert_eq!(map.facts.object_revision(key), revision);
        }
        if before == 26077 {
            assert!(
                map.facts.base_entry(pos, 1, 0).ok_or("tub")?.pickupable,
                "leaving restores the original movable tub"
            );
            assert!(
                states
                    .get(&runtime, actor, session)
                    .ok_or("state")?
                    .owned_conditions()
                    .displayed_temporary_appearance_at(2)
                    .is_none()
            );
        }
        assert_eq!(current(), Some(if reverses { before } else { after }));
        Ok(())
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

    #[test]
    fn m10_wolf_trap_player_step_never_transforms_or_damages()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| movement_case(12368, 12368, false, false))?
            .join()
            .map_err(|_| "movement test thread")?
    }

    #[test]
    fn m10_move_yellow_pillow_emits_poff_on_player_step()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        assert_eq!(pillow_speech(2397, true, true), None);
        assert_eq!(pillow_speech(2397, false, false), None);
        assert_eq!(pillow_speech(2398, true, false), None);
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| movement_case(2397, 2397, false, false))?
            .join()
            .map_err(|_| "pillow test thread")?
    }

    #[test]
    fn m10_move_bath_tub_item_outfit_and_house_tub_restore_on_exit()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| movement_case(26077, 26100, false, true))?
            .join()
            .map_err(|_| "bath tub test thread")?
    }

    #[test]
    fn m10_move_snow_step_out_all_ids_decay_back()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        assert_eq!(snow_transform(799, true, true), None);
        assert_eq!(snow_transform(799, false, false), None);
        assert_eq!(snow_transform(6594, true, false), None);
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                for before in [799].into_iter().chain(6580..=6593) {
                    movement_case(before, before, false, false)?;
                }
                Ok(())
            })?
            .join()
            .map_err(|_| "snow test thread")?
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
