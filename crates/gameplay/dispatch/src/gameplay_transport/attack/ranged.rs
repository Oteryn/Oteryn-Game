//! RANGED-ATTACK-1: ranged facts and costs of the existing auto-attack owner.
use super::*;
use crate::content::{ProjectV2RangedElement as Element, ProjectV2RangedWeapon};
use crate::content::{ReferenceItemField, ReferenceWeaponType};
use crate::durability::character_equipment::EquipmentSnapshot;
use crate::spell::formula::Formula;
use oteryn_gameplay_combat_rules::attack::ranged::{
    player_distance_formula, shot_hit_chance, shot_hits, wand_formula,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RangedLoadout {
    pub(super) equipment: EquipmentSnapshot,
    pub(super) weapon: EquippedItem,
    pub(super) projectile: Option<EquippedItem>,
    pub(super) range: u8,
    consumption: Consumption,
    pub(super) damage: RangedDamage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Consumption {
    Always,
    Break(u8),
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RangedDamage {
    Distance {
        attack: u32,
        elemental_attack: u32,
        element: Element,
        chance: i8,
        cap: u8,
        launcher_bonus: i8,
    },
    Magic {
        min: u16,
        max: u16,
        element: Element,
        mana: u16,
    },
}

pub(super) fn loadout(
    content: &NativeGameplayState,
    equipment: &EquipmentSnapshot,
) -> Option<Option<Arc<RangedLoadout>>> {
    for slot in [6, 5] {
        let Some(weapon) = equipment.items.iter().find(|item| item.slot == slot) else {
            continue;
        };
        let policy = content.item_policy(
            &weapon.definition.production_key,
            &weapon.definition.revision_ref,
        )?;
        if let Some(profile) = &policy.record().authoring.ranged {
            let (range, damage, projectile) = match profile {
                ProjectV2RangedWeapon::Distance {
                    attack,
                    range_cells,
                    hit_chance,
                    ammunition,
                    ..
                } => {
                    // Ammunition is a direct equipment slot (durable slot 10), never a bag search.
                    let ammo = equipment
                        .items
                        .iter()
                        .find(|item| item.slot == 10 && item.quantity > 0);
                    let ammo = ammo.and_then(|item| {
                        let profile = content
                            .item_policy(
                                &item.definition.production_key,
                                &item.definition.revision_ref,
                            )?
                            .record()
                            .authoring
                            .ranged
                            .as_ref()?;
                        match profile {
                            ProjectV2RangedWeapon::Ammunition {
                                ammunition: kind, ..
                            } if kind == ammunition => Some((item, profile)),
                            _ => None,
                        }
                    });
                    let (ammo_attack, chance, cap, element, elemental_attack) = match ammo {
                        Some((
                            _,
                            ProjectV2RangedWeapon::Ammunition {
                                attack,
                                hit_chance,
                                max_hit_chance,
                                element,
                                elemental_attack,
                                ..
                            },
                        )) => (
                            *attack,
                            *hit_chance,
                            *max_hit_chance,
                            *element,
                            *elemental_attack,
                        ),
                        _ => (0, 0, 0, Element::Physical, 0),
                    };
                    (
                        *range_cells,
                        RangedDamage::Distance {
                            attack: u32::from(*attack) + u32::from(ammo_attack),
                            elemental_attack: u32::from(elemental_attack),
                            element,
                            chance,
                            cap,
                            launcher_bonus: *hit_chance,
                        },
                        ammo.map(|(item, _)| item.clone()),
                    )
                }
                ProjectV2RangedWeapon::Throwing {
                    attack,
                    range_cells,
                    hit_chance,
                    max_hit_chance,
                    element,
                    elemental_attack,
                } => (
                    *range_cells,
                    RangedDamage::Distance {
                        attack: u32::from(*attack),
                        elemental_attack: u32::from(*elemental_attack),
                        element: *element,
                        chance: *hit_chance,
                        cap: *max_hit_chance,
                        launcher_bonus: 0,
                    },
                    Some(weapon.clone()),
                ),
                ProjectV2RangedWeapon::Wand(magic) | ProjectV2RangedWeapon::Rod(magic) => (
                    magic.range_cells,
                    RangedDamage::Magic {
                        min: magic.damage_min,
                        max: magic.damage_max,
                        element: magic.element,
                        mana: magic.mana_per_shot,
                    },
                    None,
                ),
                ProjectV2RangedWeapon::Ammunition { .. } => continue,
            };
            return Some(Some(Arc::new(RangedLoadout {
                equipment: equipment.clone(),
                weapon: weapon.clone(),
                projectile,
                range,
                consumption: match profile {
                    ProjectV2RangedWeapon::Distance { .. } => Consumption::Always,
                    ProjectV2RangedWeapon::Throwing { .. } => {
                        throwing_consumption(&weapon.definition.production_key)?
                    }
                    _ => Consumption::Never,
                },
                damage,
            })));
        }
        let ReferenceItemField::Known(semantics) = &policy.record().semantics.weapon else {
            if policy.record().semantics.weapon == ReferenceItemField::NotApplicable {
                continue;
            }
            return None;
        };
        match semantics.weapon_type {
            ReferenceItemField::Known(
                ReferenceWeaponType::Shield
                | ReferenceWeaponType::Spellbook
                | ReferenceWeaponType::Ammunition,
            ) => continue,
            // Missing ranged authoring cannot silently turn a weapon into a fist attack.
            ReferenceItemField::Known(
                ReferenceWeaponType::Distance | ReferenceWeaponType::Wand,
            ) => return None,
            _ => return Some(None),
        }
    }
    Some(None)
}

/// Pinned Canary items.xml nested weapon registration. At 47dfd51 these items use
/// WEAPONACTION_NONE and break in the hand; none registers WEAPONACTION_MOVE.
/// Viper star overrides its XML registration in weapons/scripts/viper_star.lua:33.
fn throwing_consumption(key: &str) -> Option<Consumption> {
    Some(match key {
        "oteryn:item.tibia.i2992" => Consumption::Always,
        "oteryn:item.tibia.i1781"
        | "oteryn:item.tibia.i3277"
        | "oteryn:item.tibia.i7378"
        | "oteryn:item.tibia.i17110" => Consumption::Break(3),
        "oteryn:item.tibia.i3287" => Consumption::Break(10),
        "oteryn:item.tibia.i3298" => Consumption::Break(7),
        "oteryn:item.tibia.i3347" => Consumption::Break(6),
        "oteryn:item.tibia.i7366" => Consumption::Break(9),
        "oteryn:item.tibia.i7367" => Consumption::Break(1),
        "oteryn:item.tibia.i7368" => Consumption::Break(33),
        "oteryn:item.tibia.i21158" => Consumption::Break(2),
        "oteryn:item.tibia.i25735" => Consumption::Break(40),
        "oteryn:item.tibia.i22870" | "oteryn:item.tibia.i40535" => Consumption::Never,
        _ => return None,
    })
}

impl RangedLoadout {
    fn mana(&self) -> u32 {
        match self.damage {
            RangedDamage::Magic { mana, .. } => u32::from(mana),
            _ => 0,
        }
    }
    pub(super) fn ready(&self, distance: u32, sight: bool, mana: u32) -> Result<(), ShotRefusal> {
        shot_gate(
            self.range,
            distance,
            sight,
            !matches!(self.damage, RangedDamage::Distance { .. }) || self.projectile.is_some(),
            mana,
            self.mana(),
        )
    }
    pub(super) fn formula(&self) -> Option<(Formula, u32, &'static str)> {
        match self.damage {
            RangedDamage::Distance {
                attack,
                elemental_attack,
                ..
            } => Some((
                player_distance_formula(),
                attack.checked_add(elemental_attack)?,
                "formula:player-distance",
            )),
            RangedDamage::Magic { min, max, .. } => Some((
                wand_formula(u32::from(min), u32::from(max))?,
                0,
                "formula:player-wand",
            )),
        }
    }
    pub(super) fn hit(&self, distance: u32, skill: u32, draw: u64) -> Option<bool> {
        match self.damage {
            RangedDamage::Distance {
                cap,
                chance,
                launcher_bonus,
                ..
            } => Some(shot_hits(
                shot_hit_chance(
                    u32::from(cap),
                    u32::try_from(chance).ok(),
                    Some(i32::from(launcher_bonus)),
                    distance.max(1),
                    skill,
                )?,
                draw,
            )),
            RangedDamage::Magic { .. } => Some(true),
        }
    }
    pub(super) fn split(&self, damage: i64) -> Option<(i64, i64, Element)> {
        match self.damage {
            RangedDamage::Magic { element, .. } => Some((0, damage, element)),
            RangedDamage::Distance {
                attack,
                elemental_attack,
                element,
                ..
            } => {
                let total = i64::from(attack.checked_add(elemental_attack)?);
                let physical = if total == 0 {
                    damage
                } else {
                    damage.checked_mul(i64::from(attack))? / total
                };
                Some((physical, damage - physical, element))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShotRefusal {
    OutOfRange,
    NotVisible,
    NoAmmunition,
    NotEnoughMana,
}

fn shot_gate(
    range: u8,
    distance: u32,
    sight: bool,
    ammunition: bool,
    mana: u32,
    cost: u32,
) -> Result<(), ShotRefusal> {
    if distance > u32::from(range) {
        Err(ShotRefusal::OutOfRange)
    } else if !sight {
        Err(ShotRefusal::NotVisible)
    } else if !ammunition {
        Err(ShotRefusal::NoAmmunition)
    } else if mana < cost {
        Err(ShotRefusal::NotEnoughMana)
    } else {
        Ok(())
    }
}

fn cost_binding(
    before: &crate::spell::cast::PlayerSpellState,
    after: &crate::spell::cast::PlayerSpellState,
) -> crate::durability::spell_items_abi::CastCostBinding {
    use sha2::{Digest, Sha256};
    let digest = |state: &crate::spell::cast::PlayerSpellState| -> [u8; 32] {
        Sha256::new()
            .chain_update(b"oteryn:ranged-cost:v1")
            .chain_update(state.revision().to_be_bytes())
            .chain_update(format!("{:?}", state.vitals()))
            .finalize()
            .into()
    };
    let a = before.vitals();
    let b = after.vitals();
    crate::durability::spell_items_abi::CastCostBinding {
        vitals_revision_before: before.revision(),
        vitals_revision_after: after.revision(),
        mana_before: a.mana,
        mana_after: b.mana,
        soul_before: a.soul,
        soul_after: b.soul,
        cooldowns_before: before.source_cooldown_deadlines(),
        cooldowns_after: after.source_cooldown_deadlines(),
        caster_digest_before: digest(before),
        caster_digest_after: digest(after),
    }
}

use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_item_transaction as items;
use crate::durability::spell_items_abi::{
    SourceCasterOrigin, SpellFieldOrigin, SpellItemOperation, SpellItemTransactionRequest,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::gameplay_transport::actor_spell::{ChannelSpellStates, native_combat_cast};
use crate::spell::cast::PlayerSpellState;

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct PendingShot {
    actor: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterItemFence,
    before: PlayerSpellState,
    paid: PlayerSpellState,
    request: SpellItemTransactionRequest,
}

fn park(attempt: PendingShot) -> Box<dyn std::any::Any + Send> {
    Box::new(native_combat_cast::UnresolvedSpellCommit::Ranged(Box::new(
        attempt,
    )))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn pay_ranged_shot(
        &self,
        permit: &mut SpellLanePermit,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        objects: &crate::world_runtime::LocalObjectRuntime,
        room: &QualifiedNativeEntryRoom,
        content: &NativeGameplayState,
        entry: &mut ActorAttack,
        selected: &super::drain::SelectedSwing,
    ) -> bool {
        let Some(attempt) = self
            .prepare_ranged_shot(
                permit, runtime, states, objects, room, content, entry, selected,
            )
            .await
        else {
            return false;
        };
        if attempt.request.operations.is_empty() {
            return states.commit(runtime, attempt.actor, attempt.session, attempt.paid);
        }
        let mut window = permit.open_commit_window(attempt, park);
        let a = window.attempt().clone();
        match self
            .root
            .commit_world_item_spell(
                &mut window,
                self.character,
                self.holder,
                a.fence,
                a.request.clone(),
                None,
                SpellFieldOrigin::System,
            )
            .await
        {
            Ok(_) => {
                let attempt = window.install();
                if !states.commit(runtime, attempt.actor, attempt.session, attempt.paid) {
                    return false;
                }
                entry.weapon = None;
                true
            }
            Err(_) => {
                if !window.commit_called() {
                    window.release();
                }
                // An unknown commit parks the entire attempt through the existing lane.
                false
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn prepare_ranged_shot(
        &self,
        permit: &SpellLanePermit,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        objects: &crate::world_runtime::LocalObjectRuntime,
        room: &QualifiedNativeEntryRoom,
        content: &NativeGameplayState,
        entry: &ActorAttack,
        selected: &super::drain::SelectedSwing,
    ) -> Option<PendingShot> {
        let ranged = selected.weapon.ranged.as_ref()?;
        let before = states.get(runtime, entry.actor, entry.session)?.clone();
        let paid = before.after_weapon_shot(ranged.mana(), selected.swing.at.get())?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(entry.session)
                .await
                .ok()?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current)?;
        let lease = runtime
            .borrow_exact_actor_commit()
            .bound_attacker_lease(entry.actor, selected.swing.lineage)
            .ok()?;
        if fence.character_id.as_bytes() != lease.character_id().as_bytes()
            || fence.character_lease_generation != lease.generation()
        {
            return None;
        }
        let from = runtime.read_actor_position(entry.actor).ok()?.position();
        let to = runtime
            .read_actor_position(selected.target)
            .ok()?
            .position();
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, permit, &*runtime, objects, room, content);
        let equipment = ranged.equipment.clone();
        let projectile = match ranged.consumption {
            Consumption::Always => ranged.projectile.clone(),
            Consumption::Never => None,
            Consumption::Break(chance) => {
                let root = GameplayDecisionRoot::from_bytes(content.source_digest());
                let draw = swing_draw(
                    &root,
                    entry.actor,
                    selected.swing.lineage,
                    selected.ordinal,
                    b"break",
                    "attack.break",
                    0,
                    99,
                )?;
                if i64::from(chance) > draw {
                    ranged.projectile.clone()
                } else {
                    None
                }
            }
        };
        let command = selected.swing.lineage;
        let operations = pass
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    use crate::durability::DurabilityError;
                    use crate::durability::character_equipment as equipment_owner;
                    use crate::durability::native_world_items::{
                        QualifiedItemDefinition, SpellGroundTarget,
                    };
                    let (owner, permit, runtime, objects, room, content) = context;
                    let mut tx = items::begin_spell_owner_transaction(holder, deadline).await?;
                    let authority = items::assert_spell_item_authority_in_transaction(
                        &mut tx,
                        permit,
                        owner.root,
                        owner.character,
                        owner.holder,
                        &fence,
                        command,
                        content.source_digest(),
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let eq_authority = equipment_owner::assert_equipment_authority_in_transaction(
                        &mut tx,
                        owner.root,
                        owner.character,
                        owner.holder,
                        &fence,
                        command,
                        content.source_digest(),
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let actual =
                        equipment_owner::read_equipment_in_transaction(&mut tx, &eq_authority)
                            .await
                            .map_err(|_| DurabilityError::Unavailable)?;
                    if actual != equipment {
                        return Err(DurabilityError::Unavailable);
                    }
                    let pos = |p: MovementLocalPosition| crate::spell::chain::TilePosition {
                        x: p.x,
                        y: p.y,
                        floor: p.floor,
                    };
                    for (cell, exempt) in native_combat_cast::sight_steps(pos(from), pos(to))
                        .map_err(|_| DurabilityError::Unavailable)?
                    {
                        if !exempt
                            && native_combat_cast::read_tile(
                                &mut tx, &authority, room, runtime, objects, cell,
                            )
                            .await
                            .map_err(|_| DurabilityError::Unavailable)?
                            .projectile
                        {
                            return Err(DurabilityError::Unavailable);
                        }
                    }
                    let mut operations = Vec::new();
                    if let Some(projectile) = projectile {
                        let policy = content
                            .item_policy(
                                &projectile.definition.production_key,
                                &projectile.definition.revision_ref,
                            )
                            .ok_or(DurabilityError::Unavailable)?;
                        let qualified = QualifiedItemDefinition::from_native_policy(policy)
                            .map_err(|_| DurabilityError::Unavailable)?;
                        let placement =
                            SpellGroundTarget::for_native_tile_read(room, runtime, from)
                                .map_err(|_| DurabilityError::Unavailable)?
                                .placement(Vec::new());
                        let source = items::prepare_inventory_consumption_in_transaction(
                            &mut tx,
                            &authority,
                            &qualified,
                            1,
                            Some(projectile.item_instance_id),
                            &placement,
                        )
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                        if source.state_revision != projectile.state_revision
                            || source.quantity_before != projectile.quantity
                        {
                            return Err(DurabilityError::Unavailable);
                        }
                        operations.push(SpellItemOperation::ConsumeInventory(source));
                    }
                    Ok(operations)
                })
            })
            .await
            .ok()?;
        Some(PendingShot {
            actor: entry.actor,
            session: entry.session,
            fence,
            request: SpellItemTransactionRequest {
                swing_ordinal: Some(selected.ordinal),
                command,
                spell: ranged.weapon.definition.clone(),
                catalog_digest: content.source_digest(),
                transaction_id: crate::operator_files::uuid_v7().ok()?,
                event_id: crate::operator_files::uuid_v7().ok()?,
                cost: cost_binding(&before, &paid),
                caster_origin: Some(SourceCasterOrigin {
                    actor: entry.actor,
                    character_lease_generation: fence.character_lease_generation,
                }),
                operations,
                companion: None,
                direct_companion: None,
            },
            before,
            paid,
        })
    }

    pub(in crate::gameplay_transport) async fn resolve_ranged_shot(
        &self,
        permit: &mut SpellLanePermit,
        attempt: PendingShot,
    ) {
        // The lane owns the attempt before either lock is awaited, including cancellation.
        let mut window = permit.open_commit_window(attempt, park);
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let a = window.attempt().clone();
        let same = states
            .get(&runtime, a.actor, a.session)
            .is_some_and(|state| {
                state.revision() == a.before.revision() && state.vitals() == a.before.vitals()
            });
        if !same {
            window.park();
            return;
        }
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(a.session)
                .await
                .ok()
                .and_then(|(current, _)| crate::gameplay_transport::item_fence_of(current));
        let Some(fresh) = current else {
            window.park();
            return;
        };
        if fresh != a.fence {
            // Only the admission journal may advance a retained same-session connection.
            // A lease, session or scope replacement cannot acquire this old shot.
            let mut expected = a.fence;
            expected.connection_generation = fresh.connection_generation;
            if expected != fresh {
                window.park();
                return;
            }
            let Ok(character) = self
                .root
                .read_current_character(self.character, a.fence.character_id)
                .await
            else {
                window.park();
                return;
            };
            let original =
                crate::durability::character_progression::CurrentCharacterGameplayFence {
                    character_id: a.fence.character_id,
                    game_session_id: a.fence.game_session_id,
                    connection_generation: a.fence.connection_generation,
                    character_lease_generation: a.fence.character_lease_generation,
                    runtime_scope: a.fence.runtime_scope,
                    scope_ownership_generation: a.fence.scope_ownership_generation,
                    expected_character_revision: character.revision,
                };
            let Ok(pass) = self.root.try_issue_semantic_pass() else {
                window.park();
                return;
            };
            let mut context = (self, window.permit());
            let command = a.request.command;
            let digest = a.request.catalog_digest;
            let proof = pass.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
                let (owner, permit) = context;
                let mut tx = items::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = items::assert_spell_item_authority_in_transaction(&mut tx, permit, owner.root, owner.character, owner.holder, &fresh, command, digest).await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                crate::durability::admission_journal::spell_reconnect::prove_pending_spell_reconnect(&mut tx, &authority, &original, &fresh).await
            })).await;
            if !matches!(proof, Ok(Some(proof)) if proof.matches(a.fence.connection_generation, fresh.connection_generation,
                command, fresh.character_id, fresh.character_lease_generation, fresh.runtime_scope, fresh.scope_ownership_generation.get()))
            {
                window.park();
                return;
            }
            window.attempt_mut().fence = fresh;
        }
        let result = self
            .root
            .commit_world_item_spell(
                &mut window,
                self.character,
                self.holder,
                fresh,
                a.request.clone(),
                None,
                SpellFieldOrigin::System,
            )
            .await;
        if result.is_err() {
            window.park();
            return;
        }
        let mut attack = self.states.attack.lock().await;
        let attempt = window.install();
        // Receipt reconciliation releases the item reservation. It never delivers a late hit.
        if states.commit(&runtime, attempt.actor, attempt.session, attempt.paid)
            && let Some(index) = attack.index(attempt.actor, attempt.session)
        {
            let entry = &mut attack.entries[index];
            entry.weapon = None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn train_distance(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    content: &NativeGameplayState,
    entry: &ActorAttack,
    hit: Option<oteryn_gameplay_progression::hit_training::HitBlock>,
    now: SemanticTimeMicros,
    trained: &mut super::skill_training::TrainingDue,
) {
    super::skill_training::train_hit(
        states,
        runtime,
        content.training_formula(),
        entry.actor,
        entry.session,
        oteryn_gameplay_progression::hit_training::SkillFamily::Distance,
        now.get(),
        |hits| hits.distance_shot(hit),
        trained,
    );
}

pub(super) fn immune(
    ranged: &RangedLoadout,
    policy: &crate::content::creature_policy::CompiledCreaturePolicy,
) -> bool {
    let (physical, elemental, element) = ranged.split(100).unwrap_or((0, 0, Element::Physical));
    (physical == 0 || policy.damage_immunities.iter().any(|key| key == "physical"))
        && (elemental == 0
            || policy
                .damage_immunities
                .iter()
                .any(|key| key == element_key(element)))
}

fn element_key(element: Element) -> &'static str {
    match element {
        Element::Physical => "physical",
        Element::Death => "death",
        Element::Earth => "earth",
        Element::Energy => "energy",
        Element::Fire => "fire",
        Element::Ice => "ice",
    }
}

pub(super) fn creature_damage(
    ranged: &RangedLoadout,
    policy: &crate::content::creature_policy::CompiledCreaturePolicy,
    damage: i64,
    (root, constants, actor, lineage, ordinal): (
        &GameplayDecisionRoot,
        &AttackConstants,
        ExactActorRef,
        CommandRef,
        u16,
    ),
) -> Option<i64> {
    use oteryn_gameplay_combat_rules::resolution::{creature_damage_response, swing_mitigated};
    let (physical, elemental, element) = ranged.split(damage)?;
    let physical = if physical > 0 {
        let physical = swing_mitigated(
            root,
            constants,
            actor.placement_identity(),
            (
                *lineage.game_session_id().as_bytes(),
                lineage.command_id().get(),
            ),
            ordinal,
            physical,
            None,
            policy.armor?,
        )?;
        creature_damage_response(policy, physical, "physical")?
    } else {
        0
    };
    let element = match element {
        Element::Physical => "physical",
        Element::Death => "death",
        Element::Earth => "earth",
        Element::Energy => "energy",
        Element::Fire => "fire",
        Element::Ice => "ice",
    };
    physical.checked_add(creature_damage_response(policy, elemental, element)?)
}

#[cfg(test)]
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn park_committed_ranged_shot_for_test(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<(), &'static str> {
        let mut permit = self.spell_lane_permit().await.ok_or("lane")?;
        let mut runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let objects = self.door.lock().await;
        let mut attack = self.states.attack.lock().await;
        let index = attack.index(actor, session).ok_or("attack entry")?;
        let entry = &mut attack.entries[index];
        let content = self
            .active_generation
            .and_then(|a| a.native_gameplay())
            .ok_or("content")?;
        let room = self.rooms.gate_auto_attack().ok_or("room")?;
        let constants = AttackConstants::checked_in().ok_or("constants")?;
        let selected = super::drain::select_swing(
            entry,
            &runtime,
            &states,
            room,
            content.source_digest(),
            self.owner_now(),
            constants,
        )
        .ok_or("selected")?;
        let attempt = self
            .prepare_ranged_shot(
                &permit,
                &mut runtime,
                &states,
                &objects,
                room,
                content,
                entry,
                &selected,
            )
            .await
            .ok_or("prepared")?;
        let mut window = permit.open_commit_window(attempt, park);
        let a = window.attempt().clone();
        self.root
            .commit_world_item_spell(
                &mut window,
                self.character,
                self.holder,
                a.fence,
                a.request.clone(),
                None,
                SpellFieldOrigin::System,
            )
            .await
            .map_err(|_| "commit")?;
        let attempt = window.install();
        // Lose the acknowledgement before installing runtime state. The ordinary next pass
        // must reconcile the exact receipt, without another unit, hit or training credit.
        permit.open_commit_window(attempt, park).park();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranged_range_los_ammunition_and_mana_gates() {
        assert_eq!(
            shot_gate(6, 7, true, true, 5, 1),
            Err(ShotRefusal::OutOfRange)
        );
        assert_eq!(
            shot_gate(6, 4, false, true, 5, 1),
            Err(ShotRefusal::NotVisible)
        );
        assert_eq!(
            shot_gate(6, 4, true, false, 5, 1),
            Err(ShotRefusal::NoAmmunition)
        );
        assert_eq!(
            shot_gate(6, 4, true, true, 0, 1),
            Err(ShotRefusal::NotEnoughMana)
        );
        assert_eq!(shot_gate(6, 4, true, true, 1, 1), Ok(()));
    }
}
