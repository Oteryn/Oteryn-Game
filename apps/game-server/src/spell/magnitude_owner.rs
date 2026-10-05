//! Qualified source-ordered magnitude finishing for the real owner batch.
//! Canary 99902524e052f37574194466c2949c576e4ab269: combat.cpp 873-878,
//! 2999-3151; creature.cpp 910-1005; monster.cpp 1415-1462;
//! game.cpp 8035-8067, 8392-8483; player.cpp 7410-7425.
//! Strict policy refuses unknown owner fields; qualified test Content may explicitly
//! omit optional numerical stages while retaining raw unknowns and real owner fences.
//! No target legality/PvP permission is granted by this numerical adapter.

use super::cast::PlayerSpellState;
use super::combat_batch::{Error, LiveActorBinding, MAX_EFFECTS};
use super::native_combat::{Element, MagnitudePlan, NativeCombatPlan};
use crate::ability::condition::ConditionValues;
use crate::foundation::{
    ChannelRuntimeV1, CharacterId, CompanionSnapshot, ExactActorRef, GameSessionId,
};

/// Content-bound test policy. Numerical approximation never grants target or session authority.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MagnitudePolicy {
    #[default]
    Strict,
    BaselineTest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactRatio {
    pub(crate) numerator: i64,
    pub(crate) denominator: u64,
}

/// These active source systems require their own stage adapters. Empty is meaningful only
/// when every actual provider has qualified their absence; None is unknown, not empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtraStage {
    CombatCallbacks,
    ElementalImbuement,
    Charms,
    WeaponProficiency,
    Prey,
    WheelCombatMastery,
    BattleHealing,
    BlessingOfTheGrove,
    HealingLink,
    SharedConservation,
    VirtueParty,
    SustainHealing,
    SanctuaryAdjacent,
    ExposedWeakness,
    BallisticMastery,
    TargetAbsorbReflect,
    ArmorConfiguration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MagnitudeOwnerBinding {
    pub(crate) actor: ExactActorRef,
    pub(crate) session: GameSessionId,
    pub(crate) character: CharacterId,
    pub(crate) lease_generation: u64,
    pub(crate) player_revision: u64,
    pub(crate) content_digest: [u8; 32],
}

/// Data projected by actual equipment/build/config owners under their current binding.
/// Private prepared constructors below independently bind it to the actual actor and
/// PlayerSpellState. A data snapshot alone is never a current ownership grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MagnitudeOwnedAttributes {
    pub(crate) policy: MagnitudePolicy,
    /// Actual durable observations, retained across numerical revalidation.
    pub(crate) owner_revisions: Option<(u64, u64, Option<u64>)>,
    pub(crate) binding: MagnitudeOwnerBinding,
    pub(crate) base_critical_chance_permyriad: Option<u16>,
    pub(crate) base_critical_extra_permyriad: Option<i32>,
    pub(crate) equipment_critical_chance_permyriad: Option<u16>,
    pub(crate) equipment_critical_extra_permyriad: Option<i32>,
    pub(crate) fatal_chance_permyriad: Option<ExactRatio>,
    pub(crate) armor_penetration_permyriad: Option<u32>,
    pub(crate) elemental_pierce_permyriad: Option<Vec<(Element, u32)>>,
    pub(crate) wheel_flat_damage: Option<i32>,
    pub(crate) wheel_flat_healing: Option<i32>,
    pub(crate) source_damage_multiplier_percent: Option<u32>,
    pub(crate) source_healing_multiplier_percent: Option<u32>,
    pub(crate) healing_dealt_percent: Option<u32>,
    pub(crate) monster_armor_disabled: Option<bool>,
    pub(crate) inactive_extra_stages: Option<Vec<ExtraStage>>,
}

impl MagnitudeOwnedAttributes {
    /// Unknown modifiers remain unknown in the owner facts. These names describe
    /// each omitted stage in the explicitly selected test baseline.
    pub(crate) fn omitted_modifiers(&self) -> Vec<&'static str> {
        let mut omitted = Vec::new();
        for (unknown, name) in [
            (
                self.base_critical_chance_permyriad.is_none(),
                "base_critical_chance",
            ),
            (
                self.base_critical_extra_permyriad.is_none(),
                "base_critical_extra",
            ),
            (
                self.equipment_critical_chance_permyriad.is_none(),
                "equipment_critical_chance",
            ),
            (
                self.equipment_critical_extra_permyriad.is_none(),
                "equipment_critical_extra",
            ),
            (self.fatal_chance_permyriad.is_none(), "fatal_chance"),
            (
                self.armor_penetration_permyriad.is_none(),
                "armor_penetration",
            ),
            (
                self.elemental_pierce_permyriad.is_none(),
                "elemental_pierce",
            ),
            (self.wheel_flat_damage.is_none(), "wheel_flat_damage"),
            (self.wheel_flat_healing.is_none(), "wheel_flat_healing"),
            (
                self.source_damage_multiplier_percent.is_none(),
                "source_damage_multiplier",
            ),
            (
                self.source_healing_multiplier_percent.is_none(),
                "source_healing_multiplier",
            ),
            (
                self.healing_dealt_percent.is_none(),
                "healing_dealt_multiplier",
            ),
            (self.monster_armor_disabled.is_none(), "armor_configuration"),
        ] {
            if unknown {
                omitted.push(name);
            }
        }
        if self.inactive_extra_stages.is_none() {
            omitted.extend([
                "combat_callbacks",
                "elemental_imbuement",
                "charms",
                "weapon_proficiency",
                "prey",
                "wheel_combat_mastery",
                "battle_healing",
                "blessing_of_the_grove",
                "healing_link",
                "shared_conservation",
                "virtue_party",
                "sustain_healing",
                "sanctuary_adjacent",
                "exposed_weakness",
                "ballistic_mastery",
                "target_absorb_reflect",
            ]);
        }
        omitted
    }

    fn resolved_for_policy(&self) -> Self {
        let mut resolved = self.clone();
        if self.policy == MagnitudePolicy::BaselineTest {
            resolved.base_critical_chance_permyriad.get_or_insert(0);
            resolved.base_critical_extra_permyriad.get_or_insert(0);
            resolved
                .equipment_critical_chance_permyriad
                .get_or_insert(0);
            resolved.equipment_critical_extra_permyriad.get_or_insert(0);
            resolved.fatal_chance_permyriad.get_or_insert(ExactRatio {
                numerator: 0,
                denominator: 1,
            });
            resolved.armor_penetration_permyriad.get_or_insert(0);
            resolved.elemental_pierce_permyriad.get_or_insert_with(|| {
                vec![
                    (Element::Physical, 0),
                    (Element::Energy, 0),
                    (Element::Fire, 0),
                    (Element::Death, 0),
                    (Element::Ice, 0),
                    (Element::Earth, 0),
                ]
            });
            resolved.wheel_flat_damage.get_or_insert(0);
            resolved.wheel_flat_healing.get_or_insert(0);
            resolved.source_damage_multiplier_percent.get_or_insert(0);
            resolved.source_healing_multiplier_percent.get_or_insert(0);
            resolved.healing_dealt_percent.get_or_insert(100);
            resolved.monster_armor_disabled.get_or_insert(false);
            resolved.inactive_extra_stages.get_or_insert_with(Vec::new);
        }
        resolved
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TargetOwner {
    Player(Box<PlayerSpellState>),
    Creature(Box<CompanionSnapshot>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    source_id: u64,
    actor: ExactActorRef,
    atom: String,
    owner: TargetOwner,
}
#[derive(Debug)]
pub(crate) struct PreparedMagnitudeOwner {
    attributes: MagnitudeOwnedAttributes,
    resolved_attributes: MagnitudeOwnedAttributes,
    caster: PlayerSpellState,
    now_ms: u64,
    plan: NativeCombatPlan,
    targets: Vec<Target>,
    shared_extensions: Option<(bool, bool)>,
}

fn invalid<T>() -> Result<T, Error> {
    Err(Error::InvalidBatch)
}
fn needed<T: Copy>(value: Option<T>) -> Result<T, Error> {
    value.ok_or(Error::InvalidBatch)
}
fn signed_truncate(value: f64) -> Result<i64, Error> {
    if !value.is_finite() || value < i32::MIN as f64 || value > i32::MAX as f64 {
        return Err(Error::InvalidMagnitude);
    }
    Ok(value.trunc() as i64)
}
fn signed_round(value: f64) -> Result<i64, Error> {
    signed_truncate(value.round())
}
fn percent(value: i64, multiplier: u32) -> Result<i64, Error> {
    signed_truncate(value as f64 * f64::from(multiplier) / 100.0)
}
fn compound(value: i64, bonus: u32) -> Result<i64, Error> {
    // C++ compound += converts the complete expression, not the isolated bonus.
    signed_truncate(value as f64 + value as f64 * f64::from(bonus) / 100.0)
}
fn element_key(element: Element) -> &'static str {
    match element {
        Element::Physical => "physical",
        Element::Energy => "energy",
        Element::Fire => "fire",
        Element::Death => "death",
        Element::Ice => "ice",
        Element::Earth => "earth",
        Element::Healing => "healing",
    }
}
fn condition_attributes(
    state: &PlayerSpellState,
    now_ms: u64,
) -> Result<Option<ConditionValues>, Error> {
    Ok(state
        .conditions
        .attributes_at(now_ms.checked_mul(1000).ok_or(Error::InvalidCondition)?))
}

impl PreparedMagnitudeOwner {
    /// Qualify the entire target set before RNG. Player targets come from the actual
    /// ChannelSpellStates borrow and are compared again by the joined player preflight.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn qualify(
        runtime: &ChannelRuntimeV1,
        caster: &PlayerSpellState,
        actor: ExactActorRef,
        session: GameSessionId,
        attributes: Option<&MagnitudeOwnedAttributes>,
        plan: &NativeCombatPlan,
        bindings: &[LiveActorBinding],
        players: &[(ExactActorRef, &PlayerSpellState)],
        now_ms: u64,
    ) -> Result<Self, Error> {
        let original = attributes.ok_or(Error::InvalidBatch)?;
        if original.policy == MagnitudePolicy::BaselineTest
            && original
                .owner_revisions
                .is_none_or(|(character, equipment, wheel)| {
                    character == 0 || equipment == 0 || wheel == Some(0)
                })
        {
            return invalid();
        }
        let resolved = original.resolved_for_policy();
        let a = &resolved;
        if a.binding.actor != actor
            || a.binding.session != session
            || a.binding.lease_generation == 0
            || a.binding.player_revision != caster.revision()
            || a.binding.content_digest != runtime.content_pin().server_artifact_digest()
            || bindings.len() > MAX_EFFECTS
        {
            return invalid();
        }
        runtime
            .player_control_facts(actor, session)
            .map_err(Error::Owner)?;
        // All configurations must be resolved, even if one particular RNG outcome would
        // make a later stage irrelevant. This keeps rejection independent of RNG.
        let _ = needed(a.base_critical_chance_permyriad)?;
        let _ = needed(a.base_critical_extra_permyriad)?;
        let _ = needed(a.equipment_critical_chance_permyriad)?;
        let _ = needed(a.equipment_critical_extra_permyriad)?;
        let fatal = needed(a.fatal_chance_permyriad)?;
        let penetration = needed(a.armor_penetration_permyriad)?;
        let pierces = a
            .elemental_pierce_permyriad
            .as_ref()
            .ok_or(Error::InvalidBatch)?;
        if fatal.denominator == 0
            || fatal.numerator < 0
            || i128::from(fatal.numerator) > 10000 * i128::from(fatal.denominator)
            || penetration > 10000
            || needed(a.monster_armor_disabled)?
            || a.inactive_extra_stages
                .as_ref()
                .is_none_or(|v| !v.is_empty())
            || needed(a.wheel_flat_damage)? < 0
            || needed(a.wheel_flat_healing)? < 0
        {
            return invalid();
        }
        let _ = needed(a.source_damage_multiplier_percent)?;
        let _ = needed(a.source_healing_multiplier_percent)?;
        let _ = needed(a.healing_dealt_percent)?;
        let mut critical_chance = u32::from(needed(a.equipment_critical_chance_permyriad)?);
        let mut critical_extra = needed(a.equipment_critical_extra_permyriad)?;
        if let Some(ConditionValues::Attributes {
            critical_chance_percent,
            critical_extra_percentage_points,
            ..
        }) = condition_attributes(caster, now_ms)?
        {
            if critical_chance_percent > 0 {
                critical_chance = critical_chance_percent
                    .checked_mul(100)
                    .ok_or(Error::InvalidCondition)?;
            }
            critical_extra = critical_extra
                .checked_add(
                    i32::try_from(
                        critical_extra_percentage_points
                            .checked_mul(100)
                            .ok_or(Error::InvalidCondition)?,
                    )
                    .map_err(|_| Error::InvalidCondition)?,
                )
                .ok_or(Error::InvalidCondition)?;
        }
        if critical_chance
            .checked_add(u32::from(needed(a.base_critical_chance_permyriad)?))
            .is_none_or(|v| v > u32::from(u16::MAX))
            || critical_extra
                .checked_add(needed(a.base_critical_extra_permyriad)?)
                .is_none_or(|v| v < 0)
        {
            return invalid();
        }
        for (i, (element, _)) in pierces.iter().enumerate() {
            if *element == Element::Healing || pierces[..i].iter().any(|v| v.0 == *element) {
                return invalid();
            }
        }
        let (element, block_armor, hits) = match plan {
            NativeCombatPlan::Combat(p) => (p.element, p.block_armor, p.hits.as_slice()),
            NativeCombatPlan::MassSpiritMend { heals, .. } => {
                (Element::Healing, false, heals.as_slice())
            }
            _ => (Element::Healing, false, &[][..]),
        };
        if element != Element::Healing && !pierces.iter().any(|v| v.0 == element) {
            return invalid();
        }
        // Canary chains evaluate each bounce under its due owner, not at cast time.
        // The timer compositor must prepare a fresh single-bounce qualifier.
        if hits.iter().any(|hit| hit.delay_ms != 0) {
            return invalid();
        }
        let mut targets = Vec::new();
        for (i, binding) in bindings.iter().enumerate() {
            if binding.target_atom.is_empty()
                || binding.target_atom.contains('\0')
                || bindings[..i]
                    .iter()
                    .any(|v| v.actor == binding.actor || v.source_id == binding.source_id)
                || !runtime.borrow_exact_actor_lookup().contains(binding.actor)
            {
                return invalid();
            }
            if !hits.iter().any(|hit| hit.target == binding.source_id) {
                continue;
            }
            let owner = if binding.actor == actor {
                TargetOwner::Player(Box::new(caster.clone()))
            } else if let Some((_, state)) = players.iter().find(|v| v.0 == binding.actor) {
                TargetOwner::Player(Box::new((*state).clone()))
            } else {
                TargetOwner::Creature(Box::new(
                    runtime
                        .companion_snapshot(binding.actor)
                        .map_err(Error::Owner)?,
                ))
            };
            if hits.iter().any(|hit| hit.target == binding.source_id) && element != Element::Healing
            {
                let TargetOwner::Creature(creature) = &owner else {
                    return invalid();
                };
                if !creature.state.policy.flags.attackable {
                    return invalid();
                }
                let p = &creature.state.policy;
                let mitigation = p.mitigation.ok_or(Error::InvalidBatch)?;
                if mitigation.denominator == 0
                    || mitigation.numerator < 0
                    || (block_armor && p.armor.is_none())
                    || p.resistances.iter().any(|r| r.percent.denominator == 0)
                {
                    return invalid();
                }
                if creature
                    .state
                    .conditions
                    .attributes_at(now_ms.checked_mul(1000).ok_or(Error::InvalidCondition)?)
                    .is_some()
                {
                    return invalid();
                }
            }
            targets.push(Target {
                source_id: binding.source_id,
                actor: binding.actor,
                atom: binding.target_atom.clone(),
                owner,
            });
        }
        if hits
            .iter()
            .any(|hit| !targets.iter().any(|v| v.source_id == hit.target))
        {
            return invalid();
        }
        Ok(Self {
            attributes: original.clone(),
            resolved_attributes: resolved,
            caster: caster.clone(),
            now_ms,
            plan: plan.clone(),
            targets,
            shared_extensions: None,
        })
    }

    /// Independent actual owner reads are required again before any physical replacement.
    pub(crate) fn validate_current(
        &self,
        runtime: &ChannelRuntimeV1,
        caster: &PlayerSpellState,
        attributes: &MagnitudeOwnedAttributes,
        players: &[(ExactActorRef, &PlayerSpellState)],
    ) -> Result<(), Error> {
        self.validate_current_with_lookup(runtime, caster, attributes, &|actor| {
            players
                .iter()
                .find(|(a, _)| *a == actor)
                .map(|(_, player)| *player)
        })
    }
    /// A borrowed owner lookup avoids allocating a temporary roster after source COMMIT.
    pub(crate) fn validate_current_with_lookup<'a>(
        &self,
        runtime: &ChannelRuntimeV1,
        caster: &PlayerSpellState,
        attributes: &MagnitudeOwnedAttributes,
        player: &dyn Fn(ExactActorRef) -> Option<&'a PlayerSpellState>,
    ) -> Result<(), Error> {
        if caster != &self.caster || attributes != &self.attributes {
            return Err(Error::SnapshotChanged);
        }
        runtime
            .player_control_facts(attributes.binding.actor, attributes.binding.session)
            .map_err(Error::Owner)?;
        for target in &self.targets {
            match &target.owner {
                TargetOwner::Creature(before) => {
                    if runtime
                        .validate_companion_snapshot(before.as_ref())
                        .is_err()
                    {
                        return Err(Error::SnapshotChanged);
                    }
                }
                TargetOwner::Player(before) if target.actor == attributes.binding.actor => {
                    if caster != before.as_ref() {
                        return Err(Error::SnapshotChanged);
                    }
                }
                TargetOwner::Player(before) => {
                    if player(target.actor) != Some(before.as_ref()) {
                        return Err(Error::SnapshotChanged);
                    }
                }
            }
        }
        Ok(())
    }

    fn extensions(
        &mut self,
        shared: bool,
        draw: &mut dyn FnMut(i64, i64) -> i64,
    ) -> Result<(bool, bool), Error> {
        if shared && let Some(value) = self.shared_extensions {
            return Ok(value);
        }
        let a = &self.resolved_attributes;
        let mut skill_chance = u32::from(needed(a.equipment_critical_chance_permyriad)?);
        let mut bonus = needed(a.equipment_critical_extra_permyriad)?;
        if let Some(ConditionValues::Attributes {
            critical_chance_percent,
            critical_extra_percentage_points,
            ..
        }) = condition_attributes(&self.caster, self.now_ms)?
        {
            if critical_chance_percent > 0 {
                skill_chance = critical_chance_percent * 100;
            }
            bonus = bonus
                .checked_add(
                    i32::try_from(critical_extra_percentage_points * 100)
                        .map_err(|_| Error::InvalidMagnitude)?,
                )
                .ok_or(Error::InvalidMagnitude)?;
        }
        let chance = skill_chance
            .checked_add(u32::from(needed(a.base_critical_chance_permyriad)?))
            .ok_or(Error::InvalidMagnitude)?;
        let _ = bonus
            .checked_add(needed(a.base_critical_extra_permyriad)?)
            .ok_or(Error::InvalidMagnitude)?;
        let critical = chance > 0 && checked_draw(draw, 1, 10000)? <= i64::from(chance);
        let fatal_chance = needed(a.fatal_chance_permyriad)?;
        let fatal = fatal_chance.numerator > 0
            && i128::from(checked_draw(draw, 0, 10000)?) * i128::from(fatal_chance.denominator)
                < i128::from(fatal_chance.numerator);
        let value = (critical, fatal);
        if shared {
            self.shared_extensions = Some(value);
        }
        Ok(value)
    }

    /// Retains source healingMap separately from the residual damage; the
    /// actual combat batch owns atomic capped healing followed by damage.
    pub(crate) fn finish_with_damage_healing(
        &mut self,
        plan: &NativeCombatPlan,
        hit: &MagnitudePlan,
        binding: &LiveActorBinding,
        draw: &mut dyn FnMut(i64, i64) -> i64,
    ) -> Result<(i64, i64), Error> {
        let mut healing = 0;
        let magnitude = self.finish_inner(plan, hit, binding, draw, &mut healing)?;
        Ok((magnitude, healing))
    }
    pub(crate) fn finish(
        &mut self,
        plan: &NativeCombatPlan,
        hit: &MagnitudePlan,
        binding: &LiveActorBinding,
        draw: &mut dyn FnMut(i64, i64) -> i64,
    ) -> Result<i64, Error> {
        self.finish_with_damage_healing(plan, hit, binding, draw)
            .map(|v| v.0)
    }
    fn finish_inner(
        &mut self,
        plan: &NativeCombatPlan,
        hit: &MagnitudePlan,
        binding: &LiveActorBinding,
        draw: &mut dyn FnMut(i64, i64) -> i64,
        damage_healing: &mut i64,
    ) -> Result<i64, Error> {
        if plan != &self.plan
            || hit.delay_ms != 0
            || hit.magnitude < 0
            || hit.target != binding.source_id
        {
            return invalid();
        }
        let authored_hits = match plan {
            NativeCombatPlan::Combat(p) => &p.hits,
            NativeCombatPlan::MassSpiritMend { heals, .. } => heals,
            _ => return invalid(),
        };
        if !authored_hits.contains(hit) {
            return invalid();
        }
        let target = self
            .targets
            .iter()
            .find(|t| {
                t.actor == binding.actor
                    && t.source_id == binding.source_id
                    && t.atom == binding.target_atom
            })
            .ok_or(Error::InvalidBatch)?
            .clone();
        let (element, armor, shared) = match plan {
            NativeCombatPlan::Combat(p) => (
                p.element,
                p.block_armor,
                p.area.is_some() || p.resolve_critical_and_fatal_once,
            ),
            NativeCombatPlan::MassSpiritMend { .. } => (Element::Healing, false, true),
            _ => return invalid(),
        };
        // Healing extensions require Blessing of the Grove. Active blessing is an
        // explicitly unsupported extra, so a qualified absent blessing makes no draw.
        let (critical, fatal) = if element == Element::Healing {
            (false, false)
        } else {
            self.extensions(shared, draw)?
        };
        let a = &self.resolved_attributes;
        let mut value = signed_truncate(hit.magnitude as f64)?;
        if critical {
            let mut bonus = needed(a.equipment_critical_extra_permyriad)?;
            if let Some(ConditionValues::Attributes {
                critical_extra_percentage_points,
                ..
            }) = condition_attributes(&self.caster, self.now_ms)?
            {
                bonus = bonus
                    .checked_add(
                        i32::try_from(critical_extra_percentage_points * 100)
                            .map_err(|_| Error::InvalidMagnitude)?,
                    )
                    .ok_or(Error::InvalidMagnitude)?;
            }
            bonus = bonus
                .checked_add(needed(a.base_critical_extra_permyriad)?)
                .ok_or(Error::InvalidMagnitude)?;
            value = signed_truncate(value as f64 * (1.0 + f64::from(bonus) / 10000.0))?;
        }
        if fatal {
            value = value
                .checked_add(signed_round(value as f64 * 0.6)?)
                .ok_or(Error::InvalidMagnitude)?;
        }
        if element == Element::Healing {
            value = percent(value, needed(a.healing_dealt_percent)?)?;
            value = compound(
                value,
                needed(a.source_healing_multiplier_percent)?
                    .checked_add(hit.bonus_percent)
                    .ok_or(Error::InvalidMagnitude)?,
            )?;
            value = value
                .checked_add(i64::from(needed(a.wheel_flat_healing)?))
                .ok_or(Error::InvalidMagnitude)?;
            return Ok(value.max(0));
        }
        if let Some(side) = hit.side_percent {
            value = signed_round(value as f64 * f64::from(side) / 100.0)?;
        }
        let TargetOwner::Creature(source_target) = &target.owner else {
            return invalid();
        };
        *damage_healing = source_target
            .state
            .policy
            .damage_healing(element_key(element), value)
            .map_err(Error::Owner)?;
        if let Some(ConditionValues::Attributes {
            damage_dealt_percent,
            ..
        }) = condition_attributes(&self.caster, self.now_ms)?
        {
            value = percent(value, damage_dealt_percent)?;
        }
        let TargetOwner::Creature(creature) = &target.owner else {
            return invalid();
        };
        let policy = &creature.state.policy;
        let key = element_key(element);
        if policy.damage_immunities.iter().any(|v| v == key) {
            return Ok(0);
        }
        if armor {
            let base = policy.armor.ok_or(Error::InvalidBatch)?;
            let penetration = if element == Element::Physical {
                needed(a.armor_penetration_permyriad)?
            } else {
                0
            };
            let armor_value =
                signed_round(f64::from(base) * (1.0 - f64::from(penetration) / 10000.0))?;
            let blocked = if armor_value > 3 {
                checked_draw(draw, armor_value / 2, armor_value - (armor_value % 2 + 1))?
            } else {
                i64::from(armor_value > 0)
            };
            value = value.saturating_sub(blocked).max(0);
        }
        let mitigation = policy.mitigation.ok_or(Error::InvalidBatch)?;
        let mitigation_percent =
            (mitigation.numerator as f64 / mitigation.denominator as f64).min(30.0);
        value = signed_truncate(value as f64 - value as f64 * mitigation_percent / 100.0)?;
        let resistance = policy
            .resistances
            .iter()
            .find(|v| v.damage_type == key)
            .map(|v| v.percent.numerator as f64 / v.percent.denominator as f64)
            .unwrap_or(0.0);
        let original = 100.0 - resistance;
        let mut sensitivity = original;
        if element != Element::Physical && original > 0.0 {
            let mut pierce = f64::from(
                a.elemental_pierce_permyriad
                    .as_ref()
                    .and_then(|v| v.iter().find(|v| v.0 == element))
                    .ok_or(Error::InvalidBatch)?
                    .1,
            ) / 100.0;
            if sensitivity < 100.0 {
                let full = pierce.min(100.0 - sensitivity);
                sensitivity += full;
                pierce -= full;
            }
            sensitivity = (sensitivity + pierce / 2.0).min(original.max(original * 2.0));
        }
        value = signed_round(value as f64 * sensitivity / 100.0)?.max(0);
        // Source immunity/armor zero short-circuit occurs BEFORE Wheel flat damage.
        if value == 0 {
            return Ok(0);
        }
        // Wheel uses signed negative C++ damage and compound truncation towards zero.
        let negative = value.checked_neg().ok_or(Error::InvalidMagnitude)?;
        let bonus = needed(a.source_damage_multiplier_percent)?
            .checked_add(hit.bonus_percent)
            .ok_or(Error::InvalidMagnitude)?;
        value = compound(negative, bonus)?
            .checked_neg()
            .ok_or(Error::InvalidMagnitude)?;
        let mut flat = i64::from(needed(a.wheel_flat_damage)?);
        if let Some(side) = hit.side_percent {
            flat = signed_round(flat as f64 * f64::from(side) / 100.0)?;
        }
        value.checked_add(flat).ok_or(Error::InvalidMagnitude)
    }
}

fn checked_draw(draw: &mut dyn FnMut(i64, i64) -> i64, min: i64, max: i64) -> Result<i64, Error> {
    let value = draw(min, max);
    if min > max || !(min..=max).contains(&value) {
        return Err(Error::InvalidMagnitude);
    }
    Ok(value)
}

#[cfg(test)]
#[path = "magnitude_owner_tests.rs"]
mod tests;
