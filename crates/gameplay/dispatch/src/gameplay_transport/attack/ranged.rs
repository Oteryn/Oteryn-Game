//! RANGED-ATTACK-1: ranged facts and costs of the existing auto-attack owner.
use super::drain::SelectedSwing;
use super::*;
use crate::content::{ProjectV2RangedElement as Element, ProjectV2RangedWeapon};
use crate::content::{ReferenceItemField, ReferenceWeaponType};
use crate::durability::character_equipment::EquipmentSnapshot;
use crate::spell::formula::{Formula, FormulaInputs};
use oteryn_gameplay_combat_rules::attack::ranged::{
    player_distance_formula, shot_hit_chance, shot_hits, wand_formula,
};
use oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence;
use oteryn_gameplay_progression::hit_training::HitBlock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScriptedArrow {
    Poison,
    Burst,
    Diamond,
}

#[derive(Debug, Clone)]
pub(super) enum ShotImpact {
    Ordinary,
    Refused,
    Area(Vec<ExactActorRef>),
}

// Crystal weapons.cpp selects among the center and its eight neighbours.
const MISS_OFFSETS: [(i32, i32); 9] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (0, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

fn missile_can_land(tile: Option<(native_combat_cast::TileFlags, bool)>) -> bool {
    tile.is_some_and(|(flags, immovable)| flags.present && !immovable)
}

impl ScriptedArrow {
    fn radius(self) -> u32 {
        match self {
            Self::Poison => 0,
            Self::Burst => 1,
            Self::Diamond => 2,
        }
    }

    fn affects(self, center: MovementLocalPosition, at: MovementLocalPosition) -> bool {
        let (dx, dy) = (center.x.abs_diff(at.x), center.y.abs_diff(at.y));
        at.floor == center.floor
            && dx.max(dy) <= self.radius()
            // Crystal diamond_arrow.lua:1–7 excludes the four outer corners.
            && (self != Self::Diamond || dx + dy < 4)
    }

    // Crystal combat.cpp:229–237 and weapons.cpp:984–1018 implement the
    // skill formula for Poison and Burst. Diamond supplies its own skill callback.
    fn bounds(self, inputs: &FormulaInputs) -> (u32, u32) {
        match self {
            Self::Diamond => {
                let level = f64::from(inputs.level) * 0.2;
                let distance = 0.09
                    * inputs.attack_factor
                    * f64::from(inputs.attack_skill)
                    * f64::from(inputs.attack_value);
                (level as u32, (distance + level) as u32)
            }
            Self::Poison | Self::Burst => {
                let distance = (0.09_f32 * inputs.attack_factor as f32)
                    * inputs.attack_skill as f32
                    * inputs.attack_value as f32;
                (0, (distance + (inputs.level / 5) as f32).round() as u32)
            }
        }
    }

    pub(super) fn damage(
        self,
        inputs: &FormulaInputs,
        root: &GameplayDecisionRoot,
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId,
    ) -> Option<(i64, &'static str)> {
        let (minimum, maximum) = self.bounds(inputs);
        let draw = oteryn_simulation_determinism::deterministic_decision_u64(
            root,
            occurrence,
            "attack.scripted-arrow-damage",
            0,
        )
        .ok()?;
        let damage = oteryn_gameplay_domain::domain::source_normal_draw::source_normal_draw(
            draw, minimum, maximum,
        )?;
        let revision = match self {
            Self::Poison => "formula:poison-arrow",
            Self::Burst => "formula:burst-arrow",
            Self::Diamond => "formula:diamond-arrow",
        };
        Some((i64::from(damage), revision))
    }
}

fn area_candidates(
    entry: &ActorAttack,
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    center: MovementLocalPosition,
    radius: u32,
    now: SemanticTimeMicros,
) -> Vec<(ExactActorRef, MovementLocalPosition)> {
    let visible = runtime.visible_entities();
    visible
        .creatures
        .iter()
        .chain(visible.players.iter())
        .filter_map(|row| {
            let actor = row.actor;
            let at = runtime.read_actor_position(actor).ok()?.position();
            if actor == entry.actor
                || at.floor != center.floor
                || center.x.abs_diff(at.x).max(center.y.abs_diff(at.y)) > radius
            {
                return None;
            }
            let facts = target_facts(runtime, room, entry.actor, entry.session, actor, now)?;
            (facts.present_and_visible
                && facts.attackable_kind
                && facts.alive
                && !facts.attacker_in_protection_zone
                && !facts.target_in_protection_zone
                && !facts.attacker_reentry_protected
                && !facts.target_reentry_protected)
                .then_some((actor, at))
        })
        .collect()
}

// The miss tile may move by one cell. Prepare native health metadata for
// every possible blast target before the combat owner takes the real swing.
pub(super) fn possible_area_targets(
    entry: &ActorAttack,
    selected: &SelectedSwing,
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    now: SemanticTimeMicros,
) -> Vec<ExactActorRef> {
    let Some(arrow @ (ScriptedArrow::Burst | ScriptedArrow::Diamond)) = selected
        .weapon
        .ranged
        .as_ref()
        .and_then(|ranged| ranged.scripted_arrow())
    else {
        return Vec::new();
    };
    let Ok(center) = runtime.read_actor_position(selected.target) else {
        return Vec::new();
    };
    area_candidates(
        entry,
        runtime,
        room,
        center.position(),
        arrow.radius() + 1,
        now,
    )
    .into_iter()
    .map(|(actor, _)| actor)
    .collect()
}

pub(super) fn poison_definition() -> Option<crate::foundation::ConditionDefinition> {
    use crate::foundation::{ConditionDefinition, DotElement};
    use oteryn_gameplay_foundation::foundation::condition::{DamageSchedule, DamageSegment};

    // Crystal poison_arrow.lua:7–12. The first tick is delayed by the full interval.
    ConditionDefinition::new_damage_schedule(
        crate::gameplay_transport::pvp_live::POISON_ARROW_CONDITION,
        1,
        DotElement::Poison,
        DamageSchedule::Fixed {
            delayed: true,
            segments: [(4, 3), (9, 2), (20, 1)]
                .map(|(count, amount)| DamageSegment {
                    count,
                    amount,
                    interval_ms: 4_000,
                })
                .into(),
        },
    )
}

pub(super) fn apply_creature_poison(
    runtime: &mut ChannelRuntimeV1,
    target: ExactActorRef,
    source: ExactActorRef,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    now: SemanticTimeMicros,
) {
    let (Ok(before), Some(definition)) = (runtime.companion_snapshot(target), poison_definition())
    else {
        return;
    };
    let facts = crate::foundation::ApplicationFacts {
        now: now.get(),
        base_speed: 0,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: false,
        decision_root: root,
        occurrence,
    };
    let mut next = before.state.clone();
    if next
        .conditions
        .apply(
            &definition,
            Some(source),
            crate::foundation::ConditionSourceKind::Player,
            &next.policy.condition_immunities,
            &facts,
        )
        .is_ok()
    {
        let _ = runtime.compare_companion_state(&before, next);
    }
}

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
        /// Projectile or throwing-weapon attack, including proficiency.
        attack: u32,
        /// Launcher attack contributes only to the roll and displayed attack.
        launcher_attack: u32,
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
    quiver_entries: &[EquippedItem],
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
                    // Extra quiver precedes Shield; a quiver's empty/incompatible entries do
                    // not mask a later compatible stack. Legacy direct slot ammunition remains valid.
                    let candidates: Vec<_> = quiver_entries
                        .iter()
                        .chain(equipment.items.iter().filter(|i| i.slot == 10))
                        .filter_map(|item| {
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
                                } => Some(oteryn_gameplay_items::quick_loot::Ammunition {
                                    key: item,
                                    kind,
                                    quantity: item.quantity,
                                }),
                                _ => None,
                            }
                        })
                        .collect();
                    let ammo = oteryn_gameplay_items::quick_loot::select_quiver_ammunition(
                        &ammunition,
                        &candidates,
                    )
                    .map(|ammo| ammo.key);
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
                            attack: u32::from(ammo_attack),
                            launcher_attack: u32::from(*attack),
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
                        launcher_attack: 0,
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
    fn uses_launcher_ammunition(&self) -> bool {
        self.projectile
            .as_ref()
            .is_some_and(|projectile| projectile.item_instance_id != self.weapon.item_instance_id)
    }

    pub(super) fn scripted_arrow(&self) -> Option<ScriptedArrow> {
        match self.projectile.as_ref()?.definition.production_key.as_str() {
            "oteryn:item.tibia.i3448" => Some(ScriptedArrow::Poison),
            "oteryn:item.tibia.i3449" => Some(ScriptedArrow::Burst),
            "oteryn:item.tibia.i35901" => Some(ScriptedArrow::Diamond),
            _ => None,
        }
    }

    pub(super) fn hits_target(
        &self,
        entry: &ActorAttack,
        selected: &SelectedSwing,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
        root: &GameplayDecisionRoot,
        offence: &EquippedOffence,
    ) -> Option<bool> {
        let (dx, dy) = (from.x.abs_diff(to.x), from.y.abs_diff(to.y));
        let draw = swing_draw(
            root,
            entry.actor,
            selected.swing.lineage,
            selected.ordinal,
            b"hit-chance",
            "attack.hit-chance",
            0,
            99,
        )?;
        let perfect = matches!(self.damage, RangedDamage::Distance { .. })
            && (offence.perfect_damage(dx, dx) != 0 || offence.perfect_damage(dy, dy) != 0);
        Some(
            perfect
                || self.hit(
                    dx.max(dy),
                    selected.skills.attack,
                    u64::try_from(draw).ok()?,
                )?,
        )
    }

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
                launcher_attack,
                elemental_attack,
                ..
            } => {
                // Launcher ammo contributes its elemental attack to the roll.
                // A throwing weapon uses those points only to split that roll.
                let ammunition = self.uses_launcher_ammunition();
                Some((
                    player_distance_formula(),
                    attack
                        .checked_add(launcher_attack)?
                        .checked_add(if ammunition { elemental_attack } else { 0 })?,
                    "formula:player-distance",
                ))
            }
            RangedDamage::Magic { min, max, .. } => Some((
                wand_formula(u32::from(min), u32::from(max))?,
                0,
                "formula:player-wand",
            )),
        }
    }
    pub(super) fn roll_bounds(
        &self,
        formula: &Formula,
        inputs: &crate::spell::formula::FormulaInputs,
        creature: bool,
    ) -> Option<(i64, i64)> {
        let (minimum, maximum) = formula.bounds(inputs).ok()?;
        // Crystal weapons.cpp:1015–1025 scales elemental launcher ammo
        // before drawing. The player path still halves damage after the roll.
        if matches!(
            self.damage,
            RangedDamage::Distance {
                elemental_attack: 1..,
                ..
            }
        ) && self.uses_launcher_ammunition()
        {
            return Some(if creature {
                (minimum / 2, maximum / 2)
            } else {
                (minimum / 4, maximum)
            });
        }
        Some((minimum, maximum))
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
        self.damage.split(damage)
    }
}

impl RangedDamage {
    fn split(self, damage: i64) -> Option<(i64, i64, Element)> {
        match self {
            RangedDamage::Magic { element, .. } => Some((0, damage, element)),
            RangedDamage::Distance {
                attack,
                elemental_attack,
                element,
                ..
            } => {
                if elemental_attack == 0 {
                    return Some((damage, 0, element));
                }
                let total = i64::from(attack.checked_add(elemental_attack)?);
                Some((
                    damage.checked_mul(i64::from(attack))? / total,
                    damage.checked_mul(i64::from(elemental_attack))? / total,
                    element,
                ))
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
    impact: ShotImpact,
}

#[cfg(test)]
thread_local! {
    static LOSE_SHOT_ACK: std::cell::Cell<Option<GameSessionId>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(in crate::gameplay_transport) fn lose_shot_ack_for(session: GameSessionId) {
    LOSE_SHOT_ACK.with(|fault| fault.set(Some(session)));
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
    ) -> Option<ShotImpact> {
        let attempt = self
            .prepare_ranged_shot(
                permit, runtime, states, objects, room, content, entry, selected,
            )
            .await?;
        if attempt.request.operations.is_empty() {
            return states
                .commit(runtime, attempt.actor, attempt.session, attempt.paid)
                .then_some(attempt.impact);
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
                #[cfg(test)]
                if LOSE_SHOT_ACK.with(|fault| {
                    if fault.get() != Some(a.session) {
                        return false;
                    }
                    fault.set(None);
                    true
                }) {
                    window.park();
                    return None;
                }
                let attempt = window.install();
                states.analyser.supplies(
                    attempt.actor,
                    attempt.session,
                    &attempt.request.operations,
                );
                if !states.commit(runtime, attempt.actor, attempt.session, attempt.paid) {
                    return None;
                }
                entry.weapon = None;
                Some(attempt.impact)
            }
            Err(_) => {
                if !window.commit_called() {
                    window.release();
                }
                // An unknown commit parks the entire attempt through the existing lane.
                None
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
        let root = GameplayDecisionRoot::from_bytes(content.source_digest());
        let area = match ranged.scripted_arrow() {
            Some(arrow @ (ScriptedArrow::Burst | ScriptedArrow::Diamond)) => {
                let offence = super::equipped_offence::attacker_offence(
                    states,
                    runtime,
                    entry.actor,
                    entry.session,
                );
                Some((
                    arrow,
                    ranged.hits_target(entry, selected, from, to, &root, &offence)?,
                ))
            }
            _ => None,
        };
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, permit, &*runtime, objects, room, content, entry);
        let equipment = ranged.equipment.clone();
        let projectile = if runtime.is_tibiadrome(entry.actor)
            && ranged
                .projectile
                .as_ref()
                .is_some_and(|p| p.item_instance_id != ranged.weapon.item_instance_id)
        {
            None
        } else {
            match ranged.consumption {
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
            }
        };
        let command = selected.swing.lineage;
        let (ordinal, now) = (selected.ordinal, selected.swing.at);
        let (operations, impact) = pass
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    use crate::durability::DurabilityError;
                    use crate::durability::character_equipment as equipment_owner;
                    use crate::durability::native_world_items::{
                        QualifiedItemDefinition, SpellGroundTarget,
                    };
                    let (owner, permit, runtime, objects, room, content, entry) = context;
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
                    let actual = equipment_owner::read_current_equipment_in_transaction(
                        &mut tx,
                        owner.root,
                        owner.character,
                        owner.holder,
                        &fence,
                        content.source_digest(),
                    )
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
                    let impact = if let Some((arrow, hit)) = area {
                        if arrow == ScriptedArrow::Burst
                            && crate::durability::pvp_live::read_expert_pvp_context_in_transaction(
                                &mut tx, owner.root, &authority,
                            )
                            .await
                            .map_err(|_| DurabilityError::Unavailable)?
                            .1
                        {
                            // weapons.cpp:327,391: a false onUseWeapon still spends one.
                            ShotImpact::Refused
                        } else {
                            let center =
                                if hit || from.x.abs_diff(to.x).max(from.y.abs_diff(to.y)) <= 1 {
                                    to
                                } else {
                                    let mut landings = Vec::new();
                                    for (dx, dy) in MISS_OFFSETS {
                                        let (Some(x), Some(y)) =
                                            (to.x.checked_add(dx), to.y.checked_add(dy))
                                        else {
                                            continue;
                                        };
                                        let candidate = MovementLocalPosition { x, y, ..to };
                                        let tile = native_combat_cast::read_projectile_tile(
                                            &mut tx,
                                            &authority,
                                            room,
                                            runtime,
                                            objects,
                                            pos(candidate),
                                        )
                                        .await
                                        .map_err(|_| DurabilityError::Unavailable)?;
                                        if missile_can_land(tile) {
                                            landings.push(candidate);
                                        }
                                    }
                                    if landings.is_empty() {
                                        to
                                    } else {
                                        let pick = swing_draw(
                                            &root,
                                            entry.actor,
                                            command,
                                            ordinal,
                                            b"miss-tile",
                                            "attack.miss-tile",
                                            0,
                                            landings.len() as i64 - 1,
                                        )
                                        .and_then(|index| usize::try_from(index).ok())
                                        .ok_or(DurabilityError::Unavailable)?;
                                        landings[pick]
                                    }
                                };
                            // Crystal AreaCombat::getList evaluates sight from the cell
                            // immediately toward the caster from the impact center.
                            let (dx, dy) = (from.x - center.x, from.y - center.y);
                            let sight_origin = MovementLocalPosition {
                                x: center.x + if dx.abs() >= dy.abs() { dx.signum() } else { 0 },
                                y: center.y + if dy.abs() >= dx.abs() { dy.signum() } else { 0 },
                                ..center
                            };
                            let mut targets = Vec::new();
                            'targets: for (actor, at) in
                                area_candidates(entry, runtime, room, center, arrow.radius(), now)
                            {
                                if !arrow.affects(center, at) {
                                    continue;
                                }
                                let Some((tile, _)) = native_combat_cast::read_projectile_tile(
                                    &mut tx,
                                    &authority,
                                    room,
                                    runtime,
                                    objects,
                                    pos(at),
                                )
                                .await
                                .map_err(|_| DurabilityError::Unavailable)?
                                else {
                                    continue;
                                };
                                if !tile.present
                                    || tile.projectile
                                    || tile.protection
                                    || tile.floor_change
                                {
                                    continue;
                                }
                                for (step, exempt) in
                                    native_combat_cast::sight_steps(pos(sight_origin), pos(at))
                                        .map_err(|_| DurabilityError::Unavailable)?
                                {
                                    if !exempt
                                        && native_combat_cast::read_projectile_tile(
                                            &mut tx, &authority, room, runtime, objects, step,
                                        )
                                        .await
                                        .map_err(|_| DurabilityError::Unavailable)?
                                        .is_none_or(|(tile, _)| tile.projectile)
                                    {
                                        continue 'targets;
                                    }
                                }
                                targets.push(actor);
                            }
                            ShotImpact::Area(targets)
                        }
                    } else {
                        ShotImpact::Ordinary
                    };
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
                        let (_, remaining) = oteryn_gameplay_items::quick_loot::consume_one(
                            oteryn_gameplay_items::quick_loot::Ammunition {
                                key: projectile.item_instance_id,
                                kind: (),
                                quantity: projectile.quantity,
                            },
                        )
                        .ok_or(DurabilityError::Unavailable)?;
                        if source.state_revision != projectile.state_revision
                            || source.quantity_after != remaining
                            || source.quantity_before != projectile.quantity
                        {
                            return Err(DurabilityError::Unavailable);
                        }
                        operations.push(SpellItemOperation::ConsumeInventory(source));
                    }
                    Ok((operations, impact))
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
            impact,
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
        states
            .analyser
            .supplies(attempt.actor, attempt.session, &attempt.request.operations);
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
    offence: &oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence,
) -> bool {
    let (physical, elemental, element) = ranged
        .split(100)
        .and_then(|(p, s, e)| offence.convert_physical(p, s, e))
        .unwrap_or((0, 0, Element::Physical));
    (physical == 0 || policy.damage_immunities.iter().any(|key| key == "physical"))
        && (elemental == 0
            || policy
                .damage_immunities
                .iter()
                .any(|key| key == element_key(element)))
}

pub(super) fn element_key(element: Element) -> &'static str {
    match element {
        Element::Physical => "physical",
        Element::Death => "death",
        Element::Earth => "earth",
        Element::Energy => "energy",
        Element::Fire => "fire",
        Element::Ice => "ice",
    }
}

pub(super) struct CreatureShot {
    pub(super) parts: [i64; 2],
    pub(super) blocks: (Option<HitBlock>, HitBlock),
}

pub(super) fn creature_damage(
    ranged: &RangedLoadout,
    policy: &crate::content::creature_policy::CompiledCreaturePolicy,
    (physical, elemental, element): (i64, i64, Element),
    (root, constants, actor, lineage, ordinal): (
        &GameplayDecisionRoot,
        &AttackConstants,
        ExactActorRef,
        CommandRef,
        u16,
    ),
    wheel: Option<(&crate::spell::cast::PlayerSpellState, u64)>,
) -> Option<CreatureShot> {
    use oteryn_gameplay_combat_rules::resolution::{
        creature_damage_response_with_reduction, swing_mitigated_with_block,
    };
    let bow = wheel.is_some_and(|(state, now)| {
        state
            .wheel_effects_at(now)
            .is_some_and(|effects| effects.active_unique_perks.contains("ballistic_mastery"))
            && state.wheel_equipment().and_then(|hands| hands.ammunition())
                == Some(crate::content::ReferenceAmmoType::Arrow)
    });
    let is_immune = |element| policy.damage_immunities.iter().any(|key| key == element);
    let (physical, primary_block) = if matches!(ranged.damage, RangedDamage::Magic { .. }) {
        (0, HitBlock::Unblocked)
    } else if is_immune("physical") {
        (0, HitBlock::Immunity)
    } else {
        let (physical, block) = swing_mitigated_with_block(
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
        (
            creature_damage_response_with_reduction(
                policy,
                physical,
                "physical",
                if bow { 2 } else { 0 },
            )?,
            block,
        )
    };
    let key = element_key(element);
    let elemental = creature_damage_response_with_reduction(
        policy,
        elemental,
        key,
        if bow && key == "physical" { 2 } else { 0 },
    )?;
    let training = if element == Element::Physical {
        (None, primary_block)
    } else {
        (
            Some(primary_block),
            if is_immune(key) {
                HitBlock::Immunity
            } else {
                HitBlock::Unblocked
            },
        )
    };
    Some(CreatureShot {
        parts: [physical, elemental],
        blocks: training,
    })
}

// Distance weapons keep their elemental contribution in the secondary slot;
// wands/rods store their ONLY primary in the elemental slot instead.
pub(super) fn native_health_primary_components(
    damage: RangedDamage,
    [physical, elemental]: [i64; 2],
    observation: Option<&crate::foundation::NativeHealthChangeObservation>,
) -> Option<[i64; 2]> {
    let Some(observation) = observation else {
        return Some([physical, elemental]);
    };
    let primary = |value: i64| {
        observation
            .primary(value.checked_neg()?)
            .ok()?
            .checked_neg()
    };
    if matches!(damage, RangedDamage::Magic { .. }) {
        Some([physical, primary(elemental)?])
    } else {
        Some([primary(physical)?, elemental])
    }
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
        let room = self.rooms().gate_auto_attack().ok_or("room")?;
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

pub(super) fn analyser_element(
    element: Element,
) -> oteryn_protocol_oteryn::damage_element::DamageElement {
    use oteryn_protocol_oteryn::damage_element::DamageElement as D;
    match element {
        Element::Physical => D::Physical,
        Element::Death => D::Death,
        Element::Earth => D::Earth,
        Element::Energy => D::Energy,
        Element::Fire => D::Fire,
        Element::Ice => D::Ice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crystal_equipment_ranged_proficiency_split_truncates_each_part() {
        let arrow = RangedDamage::Distance {
            attack: 41,
            launcher_attack: 0,
            elemental_attack: 40,
            element: Element::Fire,
            chance: 0,
            cap: 100,
            launcher_bonus: 0,
        };
        assert_eq!(arrow.split(107), Some((54, 52, Element::Fire)));
    }

    #[test]
    fn area_arrow_miss_can_land_at_center_and_on_movable_blockers() {
        let open_ground = native_combat_cast::TileFlags {
            projectile: false,
            solid: false,
            floor_change: false,
            protection: false,
            present: true,
        };
        let wall = native_combat_cast::TileFlags {
            solid: true,
            ..open_ground
        };
        let missing_ground = native_combat_cast::TileFlags {
            present: false,
            ..open_ground
        };
        assert_eq!(
            MISS_OFFSETS
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>(),
            (-1..=1)
                .flat_map(|x| (-1..=1).map(move |y| (x, y)))
                .collect()
        );
        assert!(missile_can_land(Some((wall, false))));
        assert!(!missile_can_land(Some((wall, true))));
        assert!(!missile_can_land(Some((missing_ground, false))));
        assert!(!missile_can_land(None));
    }

    #[test]
    fn crystal_scripted_arrow_masks_and_damage_bounds() {
        let origin = MovementLocalPosition {
            x: 5,
            y: 5,
            floor: 7,
        };
        for (arrow, expected) in [(ScriptedArrow::Burst, 9), (ScriptedArrow::Diamond, 21)] {
            let count = (3..=7)
                .flat_map(|x| (3..=7).map(move |y| (x, y)))
                .filter(|&(x, y)| arrow.affects(origin, MovementLocalPosition { x, y, ..origin }))
                .count();
            assert_eq!(count, expected);
        }
        let inputs = FormulaInputs {
            level: 154,
            magic_level: 0,
            base_power: None,
            attack_skill: 80,
            attack_value: 37,
            attack_factor: 1.0,
            shielding_skill: 0,
            shield_defense: None,
        };
        assert_eq!(ScriptedArrow::Diamond.bounds(&inputs), (30, 297));
        assert_eq!(ScriptedArrow::Burst.bounds(&inputs), (0, 296));
    }

    #[test]
    fn t174_converted_arrow_analyser_keeps_resolved_fire_element() -> Result<(), &'static str> {
        let offence = oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence {
            elemental_conversion: Some((Element::Fire, 50)),
            ..Default::default()
        };
        let arrow = RangedDamage::Distance {
            attack: 40,
            launcher_attack: 0,
            elemental_attack: 0,
            element: Element::Physical,
            chance: 0,
            cap: 100,
            launcher_bonus: 0,
        };
        let (physical, elemental, element) = arrow.split(100).ok_or("arrow damage")?;
        let (physical, elemental, element) = offence
            .convert_physical(physical, elemental, element)
            .ok_or("conversion")?;
        assert_eq!((physical, elemental), (50, 50));
        assert_eq!(
            analyser_element(element),
            oteryn_protocol_oteryn::damage_element::DamageElement::Fire
        );
        Ok(())
    }

    #[test]
    fn t174_elemental_distance_split_uses_projectile_attack() -> Result<(), String> {
        use oteryn_gameplay_content::content::native_gameplay::{
            NativeGameplayInput, compile_native_gameplay, decode,
        };
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        let mut source =
            NativeGameplayInput::from_manifest(&manifest).map_err(|e| format!("{e:?}"))?;
        source.source_world = None;
        let mut id = [5; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let world = crate::foundation::WorldId::decode(&id).map_err(|e| format!("{e:?}"))?;
        let room =
            crate::content::qualify_native_entry_room(world).map_err(|e| format!("{e:?}"))?;
        let artefact =
            compile_native_gameplay(room.compiled(), &source).map_err(|e| format!("{e:?}"))?;
        let native = decode(&artefact.server_artifact)
            .map_err(|e| format!("{e:?}"))?
            .state;
        let item = |slot, production_key: &str| EquippedItem {
            tier: 0,
            slot,
            item_instance_id: [slot; 16],
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: production_key.into(),
                revision_ref: "definition-r1".into(),
            },
            quantity: 1,
            state_revision: 1,
        };
        let mut equipment = EquipmentSnapshot {
            character: id,
            content_digest: native.source_digest(),
            character_revision: 1,
            revision: 1,
            combat_mode: None,
            items: vec![
                item(5, "oteryn:item.tibia.i52332"),
                item(10, "oteryn:item.tibia.i763"),
            ],
        };
        let mut shot = loadout(&native, &equipment, &[])
            .flatten()
            .ok_or("Crypt Spine and flaming arrow loadout")?;
        let RangedDamage::Distance { attack, .. } = &mut Arc::make_mut(&mut shot).damage else {
            return Err("distance loadout".into());
        };
        *attack += 2;
        assert_eq!(shot.formula().map(|(_, attack, _)| attack), Some(38));
        assert_eq!(shot.split(100), Some((53, 46, Element::Fire)));
        let (formula, attack_value, _) = shot.formula().ok_or("distance formula")?;
        let inputs = crate::spell::formula::FormulaInputs {
            level: 200,
            magic_level: 0,
            base_power: None,
            attack_skill: 100,
            attack_value,
            attack_factor: 1.0,
            shielding_skill: 0,
            shield_defense: None,
        };
        let (minimum, maximum) = formula.bounds(&inputs).map_err(|err| format!("{err:?}"))?;
        assert_eq!(
            shot.roll_bounds(&formula, &inputs, true),
            Some((minimum / 2, maximum / 2))
        );
        assert_eq!(
            shot.roll_bounds(&formula, &inputs, false),
            Some((minimum / 4, maximum))
        );

        equipment.items = vec![item(5, "oteryn:item.tibia.i25735")];
        let throwing = loadout(&native, &equipment, &[])
            .flatten()
            .ok_or("leaf star loadout")?;
        let RangedDamage::Distance { attack, .. } = throwing.damage else {
            return Err("throwing loadout".into());
        };
        assert_eq!(throwing.formula().map(|(_, value, _)| value), Some(attack));
        assert_eq!(throwing.split(99), Some((95, 3, Element::Earth)));
        Ok(())
    }

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
    #[test]
    fn srx3_actual_health_forgotten_keeps_ranged_secondary_and_selects_magic_primary()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let world = crate::content::native_gameplay_test_world_id()
                    .map_err(|error| format!("{error:?}"))?;
                let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/spells.manifest.json");
                let native =
                    crate::content::native_gameplay::callback_native_policy_fixture_at(&manifest)
                        .map_err(|error| format!("{error:?}"))?;
                let id = |tag| {
                    let mut bytes = [tag; 16];
                    bytes[6] = 0x70;
                    bytes[8] = 0x80;
                    bytes
                };
                let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                    world,
                    crate::foundation::ChannelId::decode(&id(101))
                        .map_err(|error| format!("{error:?}"))?,
                    crate::foundation::NodeId::decode(&id(102))
                        .map_err(|error| format!("{error:?}"))?,
                    1,
                    1,
                    1,
                    "runtime-scope-assignment:1",
                    2,
                    crate::foundation::ChannelContentPin::from_activation(
                        world,
                        1,
                        native.source_digest(),
                        [2; 32],
                        [3; 32],
                        [4; 32],
                        (100, 100, 7),
                    ),
                )
                .map_err(|error| format!("{error:?}"))?;
                runtime
                    .install_companion_policies(native.policies().clone())
                    .map_err(|error| format!("{error:?}"))?;
                let key = "oteryn:creature.lady_tenebris";
                let maximum = runtime
                    .companion_policy(key)
                    .map_err(|error| format!("{error:?}"))?
                    .maximum_health;
                let actor = runtime
                    .admit_source_pinned_lab_creature(
                        crate::foundation::MovementLocalPosition {
                            x: 100,
                            y: 100,
                            floor: 7,
                        },
                        key,
                        maximum,
                    )
                    .map_err(|error| format!("{error:?}"))?;
                runtime
                    .install_creature_policy(actor, key)
                    .map_err(|error| format!("{error:?}"))?;
                let observation = runtime
                    .observe_native_health_change(
                        actor,
                        &|_: crate::foundation::ExactActorRef,
                          _: crate::foundation::MovementPositionSnapshot| {
                            None
                        },
                    )
                    .map_err(|error| format!("{error:?}"))?
                    .ok_or("actual attached native HealthForgotten observation")?;
                runtime
                    .validate_native_health_change_observation(&observation)
                    .map_err(|error| format!("{error:?}"))?;
                let distance = RangedDamage::Distance {
                    attack: 31,
                    launcher_attack: 0,
                    elemental_attack: 11,
                    element: Element::Fire,
                    chance: 100,
                    cap: 100,
                    launcher_bonus: 0,
                };
                let magic = RangedDamage::Magic {
                    min: 11,
                    max: 11,
                    element: Element::Fire,
                    mana: 0,
                };
                assert_eq!(
                    native_health_primary_components(distance, [31, 11], Some(&observation)),
                    Some([62, 11]),
                    "distance elemental contribution is SECONDARY and unchanged"
                );
                assert_eq!(
                    native_health_primary_components(distance, [0, 11], Some(&observation)),
                    Some([0, 11]),
                    "a blocked primary cannot turn its secondary into a primary"
                );
                assert_eq!(
                    native_health_primary_components(magic, [0, 11], Some(&observation)),
                    Some([0, 22]),
                    "wand/rod elemental slot is its only PRIMARY"
                );
                assert_eq!(
                    native_health_primary_components(distance, [31, 11], None),
                    Some([31, 11])
                );
                Ok(())
            })?
            .join()
            .map_err(|_| "native ranged HealthForgotten worker panicked")?
            .map_err(Into::into)
    }
}
