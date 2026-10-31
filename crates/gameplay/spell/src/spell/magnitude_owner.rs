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
use crate::foundation::{
    ChannelRuntimeV1, CharacterId, CompanionSnapshot, ExactActorRef, GameSessionId,
};
use oteryn_gameplay_domain::domain::condition::ConditionValues;

pub use oteryn_gameplay_domain::domain::magnitude_policy::MagnitudePolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactRatio {
    pub numerator: i64,
    pub denominator: u64,
}

/// These active source systems require their own stage adapters. Empty is meaningful only
/// when every actual provider has qualified their absence; None is unknown, not empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtraStage {
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
pub struct MagnitudeOwnerBinding {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub character: CharacterId,
    pub lease_generation: u64,
    pub player_revision: u64,
    pub content_digest: [u8; 32],
}

/// Data projected by actual equipment/build/config owners under their current binding.
/// Private prepared constructors below independently bind it to the actual actor and
/// PlayerSpellState. A data snapshot alone is never a current ownership grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagnitudeOwnedAttributes {
    pub policy: MagnitudePolicy,
    pub item_augments: Vec<oteryn_gameplay_content::content::item_spell_augments::ItemSpellAugment>,
    /// Actual durable observations, retained across numerical revalidation.
    pub owner_revisions: Option<(u64, u64, Option<u64>)>,
    pub binding: MagnitudeOwnerBinding,
    pub base_critical_chance_permyriad: Option<u16>,
    pub base_critical_extra_permyriad: Option<i32>,
    pub equipment_critical_chance_permyriad: Option<u16>,
    pub equipment_critical_extra_permyriad: Option<i32>,
    pub fatal_chance_permyriad: Option<ExactRatio>,
    pub armor_penetration_permyriad: Option<u32>,
    pub elemental_pierce_permyriad: Option<Vec<(Element, u32)>>,
    /// The same fenced allocation/Atelier snapshot as owner_revisions, never an admission cache.
    pub wheel_effects: Option<oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects>,
    pub wheel_flat_damage: Option<i32>,
    pub wheel_flat_healing: Option<i32>,
    pub source_damage_multiplier_percent: Option<u32>,
    pub source_healing_multiplier_percent: Option<u32>,
    pub healing_dealt_percent: Option<u32>,
    pub monster_armor_disabled: Option<bool>,
    pub inactive_extra_stages: Option<Vec<ExtraStage>>,
}

impl MagnitudeOwnedAttributes {
    pub fn delayed_stance_snapshot(
        &self,
        resolved: super::native_delayed::Element,
        revision: u64,
        spell_id: u32,
    ) -> Result<super::native_delayed::StanceSnapshot, Error> {
        let a = self.resolved_for_policy();
        let critical_chance_permyriad = u32::from(needed(a.base_critical_chance_permyriad)?)
            .checked_add(u32::from(needed(a.equipment_critical_chance_permyriad)?))
            .ok_or(Error::InvalidMagnitude)?;
        let critical_damage_permyriad = u32::try_from(
            needed(a.base_critical_extra_permyriad)?
                .checked_add(needed(a.equipment_critical_extra_permyriad)?)
                .ok_or(Error::InvalidMagnitude)?,
        )
        .map_err(|_| Error::InvalidMagnitude)?;
        Ok(super::native_delayed::StanceSnapshot {
            intrinsic: super::native_delayed::Element::Death,
            resolved,
            state_revision: revision,
            spell_id,
            damage_multiplier_permyriad: i32::try_from(
                needed(a.source_damage_multiplier_percent)?
                    .checked_add(100)
                    .and_then(|v| v.checked_mul(100))
                    .ok_or(Error::InvalidMagnitude)?,
            )
            .map_err(|_| Error::InvalidMagnitude)?,
            critical_chance_permyriad,
            critical_damage_permyriad,
            converted: resolved != super::native_delayed::Element::Death,
        })
    }
    /// Unknown modifiers remain unknown in the owner facts. These names describe
    /// each omitted stage in the explicitly selected test baseline.
    pub fn omitted_modifiers(&self) -> Vec<&'static str> {
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
                    (Element::Holy, 0),
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
pub struct PreparedMagnitudeOwner {
    pub attributes: MagnitudeOwnedAttributes,
    resolved_attributes: MagnitudeOwnedAttributes,
    selected_item_augments:
        Vec<oteryn_gameplay_content::content::item_spell_augments::ItemSpellAugment>,
    caster: PlayerSpellState,
    selected_spell: Option<String>,
    selected_focus_group: bool,
    now_ms: u64,
    plan: NativeCombatPlan,
    targets: Vec<Target>,
    shared_extensions: Option<(bool, bool)>,
    pvp_targets: Vec<(ExactActorRef, bool)>,
    delayed_damage_percent: Option<u32>,
    /// Position snapshots are captured only for an active adjacency consumer and revalidated.
    wheel_positions: Vec<(ExactActorRef, crate::foundation::MovementPositionSnapshot)>,
    wheel_link_echo: i64,
    wheel_census: Option<
        Vec<(
            ExactActorRef,
            crate::foundation::MovementPositionSnapshot,
            Option<GameSessionId>,
        )>,
    >,
    wheel_masterless: Vec<(ExactActorRef, bool)>,
    // Private numerical observations, never caller-authored bypass flags. Each result
    // is minted only after finish_inner applied the real target's source block stage.
    player_block_shielding: Vec<(ExactActorRef, u32)>,
    finished_player_damage: Vec<(ExactActorRef, i64)>,
}

/// Values whose timing D.1.3 fixes at planting. Original owner observations stay
/// separate, so delayed numerical validation still compares fresh real owners.
#[derive(Debug, Clone)]
pub struct StrikeCastModifiers {
    source_bonus: u32,
    damage_percent: u32,
    base_damage_augments:
        Vec<oteryn_gameplay_content::content::item_spell_augments::ItemSpellAugment>,
}
impl StrikeCastModifiers {
    pub fn capture(
        attributes: &MagnitudeOwnedAttributes,
        caster: &PlayerSpellState,
        spell_name: &str,
        now_ms: u64,
    ) -> Result<Self, Error> {
        let a = attributes.resolved_for_policy();
        let damage_percent = match condition_attributes(caster, now_ms)? {
            Some(ConditionValues::Attributes {
                damage_dealt_percent,
                ..
            }) => damage_dealt_percent,
            _ => 100,
        };
        Ok(Self {
            // Keep the established raw/stance snapshot. Crystal's delayed Combat
            // reads gem and Focus bonuses at the actual strike, not at planting.
            source_bonus: needed(a.source_damage_multiplier_percent)?,
            damage_percent,
            base_damage_augments: a.item_augments.iter().filter(|augment| {
                augment.spell.eq_ignore_ascii_case(spell_name)
                    && augment.kind == oteryn_gameplay_content::content::item_spell_augments::ItemSpellAugmentKind::BaseDamage
            }).cloned().collect(),
        })
    }
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
        Element::Holy => "holy",
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
    /// F's planted raw/late-modifier split. This changes only numerical inputs;
    /// `attributes` and `caster` remain intact for commit-time owner validation.
    pub fn select_planted_strike(&mut self, snapshot: &StrikeCastModifiers) -> Result<(), Error> {
        let wheel_bonus = self.selected_wheel_damage_bonus()?;
        self.selected_item_augments.retain(|augment| augment.kind != oteryn_gameplay_content::content::item_spell_augments::ItemSpellAugmentKind::BaseDamage);
        self.selected_item_augments
            .extend(snapshot.base_damage_augments.iter().cloned());
        // The raw formula remains planted. Wheel flat belongs to the current
        // Combat after target block, alongside its current gem/Focus modifiers.
        self.resolved_attributes.source_damage_multiplier_percent = Some(
            snapshot
                .source_bonus
                .checked_add(wheel_bonus)
                .ok_or(Error::InvalidMagnitude)?,
        );
        self.delayed_damage_percent = Some(snapshot.damage_percent);
        Ok(())
    }
    pub fn select_echo_stance(
        &mut self,
        snapshot: &super::native_delayed::StanceSnapshot,
    ) -> Result<(), Error> {
        self.resolved_attributes.base_critical_chance_permyriad = Some(
            u16::try_from(snapshot.critical_chance_permyriad)
                .map_err(|_| Error::InvalidMagnitude)?,
        );
        self.resolved_attributes.equipment_critical_chance_permyriad = Some(0);
        self.resolved_attributes.base_critical_extra_permyriad = Some(
            i32::try_from(snapshot.critical_damage_permyriad)
                .map_err(|_| Error::InvalidMagnitude)?,
        );
        self.resolved_attributes.equipment_critical_extra_permyriad = Some(0);
        self.resolved_attributes.source_damage_multiplier_percent = Some(
            u32::try_from(snapshot.damage_multiplier_permyriad / 100 - 100)
                .map_err(|_| Error::InvalidMagnitude)?
                .checked_add(self.selected_wheel_damage_bonus()?)
                .ok_or(Error::InvalidMagnitude)?,
        );
        Ok(())
    }
    /// Select only the named instant spell from the original fenced Item observations.
    /// Selection leaves those observations intact for commit-time revalidation.
    pub fn select_item_spell(&mut self, spell: &super::SpellDefinition) -> Result<(), Error> {
        use oteryn_gameplay_content::content::item_spell_augments::ItemSpellAugmentKind as K;
        self.selected_item_augments.clear();
        self.finished_player_damage.clear();
        self.selected_spell = Some(spell.name.clone());
        self.selected_focus_group = spell
            .groups
            .get(1)
            .is_some_and(|group| group.key == "focus");
        self.shared_extensions = None;
        self.resolved_attributes = self.attributes.resolved_for_policy();
        // Crystal wheel:3535-3539 stores gem spell bonuses separately from global
        // critical skill. Projection already performed each gem's source integer cast.
        if let Some(effects) = &self.attributes.wheel_effects {
            let healing =
                u32::try_from(effects.gem_value_milli("base_healing_bonus", &spell.name) / 1_000)
                    .map_err(|_| Error::InvalidMagnitude)?;
            self.resolved_attributes.source_healing_multiplier_percent = Some(
                needed(self.resolved_attributes.source_healing_multiplier_percent)?
                    .checked_add(healing)
                    .ok_or(Error::InvalidMagnitude)?,
            );
        }
        self.resolved_attributes.source_damage_multiplier_percent = Some(
            needed(self.resolved_attributes.source_damage_multiplier_percent)?
                .checked_add(self.selected_wheel_damage_bonus()?)
                .ok_or(Error::InvalidMagnitude)?,
        );
        if !matches!(spell.carrier, super::Carrier::Instant { .. }) {
            return Ok(());
        }
        for augment in &self.attributes.item_augments {
            if !augment.spell.eq_ignore_ascii_case(&spell.name) {
                continue;
            }
            match augment.kind {
                K::CriticalHitChance => {
                    let chance = u32::from(needed(
                        self.resolved_attributes.base_critical_chance_permyriad,
                    )?)
                    .checked_add(augment.value)
                    .ok_or(Error::InvalidMagnitude)?;
                    self.resolved_attributes.base_critical_chance_permyriad =
                        Some(chance.min(10000) as u16);
                }
                K::CriticalExtraDamage => {
                    self.resolved_attributes.base_critical_extra_permyriad = Some(
                        needed(self.resolved_attributes.base_critical_extra_permyriad)?
                            .checked_add(
                                i32::try_from(augment.value)
                                    .map_err(|_| Error::InvalidMagnitude)?,
                            )
                            .ok_or(Error::InvalidMagnitude)?,
                    );
                }
                _ => {}
            }
            self.selected_item_augments.push(augment.clone());
        }
        Ok(())
    }

    /// Qualify the entire target set before RNG. Player targets come from the actual
    /// ChannelSpellStates borrow and are compared again by the joined player preflight.
    #[allow(clippy::too_many_arguments)]
    pub fn qualify(
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
        Self::qualify_pvp(
            runtime,
            caster,
            actor,
            session,
            attributes,
            plan,
            bindings,
            players,
            now_ms,
            &[],
        )
    }

    /// Same magnitude owner, with the live dispatch's current legal unarmoured player set.
    #[allow(clippy::too_many_arguments)]
    pub fn qualify_pvp(
        runtime: &ChannelRuntimeV1,
        caster: &PlayerSpellState,
        actor: ExactActorRef,
        session: GameSessionId,
        attributes: Option<&MagnitudeOwnedAttributes>,
        plan: &NativeCombatPlan,
        bindings: &[LiveActorBinding],
        players: &[(ExactActorRef, &PlayerSpellState)],
        now_ms: u64,
        pvp_targets: &[(ExactActorRef, bool)],
    ) -> Result<Self, Error> {
        let original = attributes.ok_or(Error::InvalidBatch)?;
        if original.policy == MagnitudePolicy::BaselineTest
            && original
                .owner_revisions
                .is_none_or(|(character, equipment, wheel)| {
                    character == 0
                        || equipment == 0
                        || wheel == Some(0)
                        || (wheel.is_none()
                            && (original.wheel_flat_damage.is_some()
                                || original.wheel_flat_healing.is_some()))
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
                if let TargetOwner::Player(_) = &owner {
                    if !pvp_targets.iter().any(|(a, _)| *a == binding.actor) {
                        return invalid();
                    }
                    targets.push(Target {
                        source_id: binding.source_id,
                        actor: binding.actor,
                        atom: binding.target_atom.clone(),
                        owner,
                    });
                    continue;
                }
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
        let mut player_block_shielding = Vec::new();
        if element != Element::Healing {
            for target in &targets {
                if let TargetOwner::Player(state) = &target.owner {
                    let bonus = state
                        .wheel_battle_instinct(
                            runtime,
                            target.actor,
                            now_ms.checked_mul(1_000).ok_or(Error::InvalidMagnitude)?,
                        )
                        .ok_or(Error::SnapshotChanged)?;
                    player_block_shielding.push((target.actor, bonus.shielding));
                }
            }
        }
        let mut wheel_positions = Vec::new();
        if a.wheel_effects
            .as_ref()
            .is_some_and(|effects| effects.active_unique_perks.contains("sanctuary"))
        {
            for current in std::iter::once(actor).chain(targets.iter().map(|target| target.actor)) {
                if !wheel_positions.iter().any(|(old, _)| *old == current) {
                    wheel_positions.push((
                        current,
                        runtime.read_actor_position(current).map_err(Error::Owner)?,
                    ));
                }
            }
        }
        let wheel_census = if a.wheel_effects.as_ref().is_some_and(|effects| {
            effects.active_unique_perks.contains("battle_instinct")
                || effects.active_unique_perks.contains("positional_tactics")
        }) {
            Some(runtime.positioned_actor_census().map_err(Error::Owner)?)
        } else {
            None
        };
        let mut wheel_masterless = Vec::new();
        if let Some(census) = &wheel_census {
            for (actor, _, session) in census {
                if session.is_none() {
                    wheel_masterless.push((
                        *actor,
                        runtime
                            .companion_snapshot(*actor)
                            .map_err(Error::Owner)?
                            .state
                            .master
                            .is_none(),
                    ));
                }
            }
        }
        Ok(Self {
            attributes: original.clone(),
            selected_item_augments: Vec::new(),
            resolved_attributes: resolved,
            caster: caster.clone(),
            selected_spell: None,
            selected_focus_group: false,
            now_ms,
            plan: plan.clone(),
            targets,
            shared_extensions: None,
            pvp_targets: pvp_targets.to_vec(),
            delayed_damage_percent: None,
            wheel_positions,
            wheel_link_echo: 0,
            wheel_census,
            wheel_masterless,
            player_block_shielding,
            finished_player_damage: Vec::new(),
        })
    }

    /// Independent actual owner reads are required again before any physical replacement.
    pub fn validate_current(
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
    pub fn validate_current_with_lookup<'a>(
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
        if let Some(census) = &self.wheel_census {
            runtime
                .validate_positioned_actor_census(census)
                .map_err(|_| Error::SnapshotChanged)?;
            for (actor, masterless) in &self.wheel_masterless {
                if runtime
                    .companion_snapshot(*actor)
                    .map_err(|_| Error::SnapshotChanged)?
                    .state
                    .master
                    .is_none()
                    != *masterless
                {
                    return Err(Error::SnapshotChanged);
                }
            }
        }
        for (actor, shielding) in &self.player_block_shielding {
            let state = if *actor == attributes.binding.actor {
                Some(caster)
            } else {
                player(*actor)
            }
            .ok_or(Error::SnapshotChanged)?;
            let now = self
                .now_ms
                .checked_mul(1_000)
                .ok_or(Error::InvalidMagnitude)?;
            if state
                .wheel_battle_instinct(runtime, *actor, now)
                .map(|bonus| bonus.shielding)
                != Some(*shielding)
            {
                return Err(Error::SnapshotChanged);
            }
        }
        for (actor, expected) in &self.wheel_positions {
            if runtime.read_actor_position(*actor).map_err(Error::Owner)? != *expected {
                return Err(Error::SnapshotChanged);
            }
        }
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

    /// Prove that every player Damage in this batch is one exact result of this
    /// numerical owner, under the same current caster/target/physical observations.
    /// Generic batches cannot request that the final HP owner skip blockHit.
    pub fn qualify_preprocessed_player_batch<'a>(
        &self,
        runtime: &ChannelRuntimeV1,
        batch: &super::combat_batch::OwnerCombatBatch,
        player: &dyn Fn(ExactActorRef) -> Option<&'a PlayerSpellState>,
    ) -> Result<(), Error> {
        if batch.caster != self.attributes.binding.actor
            || batch.command.game_session_id() != self.attributes.binding.session
            || batch.attacker != self.attributes.binding.character
            || batch.current_lease_generation != self.attributes.binding.lease_generation
            || batch.now_ms != self.now_ms
        {
            return invalid();
        }
        self.validate_current_with_lookup(
            runtime,
            player(batch.caster).ok_or(Error::SnapshotChanged)?,
            &self.attributes,
            player,
        )?;
        let mut used = vec![false; self.finished_player_damage.len()];
        for effect in &batch.effects {
            if runtime.contains_live_creature(effect.target) {
                continue;
            }
            if let super::combat_batch::OwnerCombatChange::Damage { magnitude, .. } = &effect.change
            {
                let index = self
                    .finished_player_damage
                    .iter()
                    .enumerate()
                    .find(|(index, result)| {
                        !used[*index] && **result == (effect.target, *magnitude)
                    })
                    .map(|(index, _)| index)
                    .ok_or(Error::InvalidBatch)?;
                used[index] = true;
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
    pub fn finish_with_damage_healing(
        &mut self,
        plan: &NativeCombatPlan,
        hit: &MagnitudePlan,
        binding: &LiveActorBinding,
        draw: &mut dyn FnMut(i64, i64) -> i64,
    ) -> Result<(i64, i64), Error> {
        let mut healing = 0;
        let magnitude = self.finish_inner(plan, hit, binding, draw, &mut healing)?;
        if magnitude > 0
            && matches!(plan, NativeCombatPlan::Combat(combat) if combat.element != Element::Healing)
            && self.targets.iter().any(|target| {
                target.actor == binding.actor && matches!(target.owner, TargetOwner::Player(_))
            })
        {
            if self.finished_player_damage.len() >= MAX_EFFECTS {
                return invalid();
            }
            self.finished_player_damage
                .try_reserve(1)
                .map_err(|_| Error::InvalidBatch)?;
            self.finished_player_damage.push((binding.actor, magnitude));
        }
        Ok((magnitude, healing))
    }
    pub fn finish(
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
        // Ordinary healing does not roll extensions. Grove's critical heal is rolled
        // after its healing bonuses below (Crystal 201189f game.cpp:7629-7645).
        let (critical, fatal) = if element == Element::Healing {
            (false, false)
        } else {
            self.extensions(shared, draw)?
        };
        let a = &self.resolved_attributes;
        let mut value = signed_truncate(hit.magnitude as f64)?;
        // Item base bonuses precede critical, target mitigation and Wheel flat stages.
        for augment in &self.selected_item_augments {
            use oteryn_gameplay_content::content::item_spell_augments::ItemSpellAugmentKind as K;
            if matches!((augment.kind, element), (K::BaseHealing, Element::Healing))
                || (augment.kind == K::BaseDamage && element != Element::Healing)
            {
                value = value
                    .checked_add(signed_truncate(
                        value as f64 * f64::from(augment.value) / 10000.0,
                    )?)
                    .ok_or(Error::InvalidMagnitude)?;
            }
        }
        if critical {
            let bonus = self.critical_extra()?;
            value = signed_truncate(value as f64 * (1.0 + f64::from(bonus) / 10000.0))?;
        }
        if fatal {
            value = value
                .checked_add(signed_round(value as f64 * 0.6)?)
                .ok_or(Error::InvalidMagnitude)?;
        }
        let party = self
            .caster
            .virtue_party_bonuses(self.now_ms.saturating_mul(1_000));
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
            value = self.finish_wheel_healing(value.max(0), &target, draw)?;
            // Crystal game.cpp:7837-7841 applies Wheel before named-spell Sustain/aura.
            if self.selected_spell.is_some() {
                value = percent(value, self.caster.sustain_healing_percent())?;
                value = percent(value, 100 + party.spell_healing)?;
            }
            return Ok(value);
        }
        value = percent(value, 100 + party.spell_damage)?;
        if let Some(side) = hit.side_percent {
            value = signed_round(value as f64 * f64::from(side) / 100.0)?;
        }
        if let TargetOwner::Player(target_state) = &target.owner {
            let black = self
                .pvp_targets
                .iter()
                .find(|(a, _)| *a == target.actor)
                .ok_or(Error::InvalidBatch)?
                .1;
            if let Some(ConditionValues::Attributes {
                damage_dealt_percent,
                ..
            }) = condition_attributes(&self.caster, self.now_ms)?
            {
                value = percent(
                    value,
                    self.delayed_damage_percent.unwrap_or(damage_dealt_percent),
                )?;
            } else if let Some(percent_at_cast) = self.delayed_damage_percent {
                value = percent(value, percent_at_cast)?;
            }
            // Crystal combat.cpp:796-801 halves PvP before target blockHit and
            // game.cpp:7945-7946 applies Wheel multipliers/flat afterwards.
            let pvp = oteryn_gameplay_combat_rules::pvp::pvp_damage(
                u32::try_from(value.max(0)).map_err(|_| Error::InvalidMagnitude)?,
                black,
            );
            let shielding = self
                .player_block_shielding
                .iter()
                .find(|(actor, _)| *actor == target.actor)
                .ok_or(Error::InvalidBatch)?
                .1;
            value = i64::from(
                target_state
                    .after_player_block(
                        pvp,
                        crate::player_lethal::PlayerHitKind::from_element(Some(element_key(
                            element,
                        ))),
                        shielding,
                        self.now_ms
                            .checked_mul(1_000)
                            .ok_or(Error::InvalidMagnitude)?,
                    )
                    .ok_or(Error::InvalidMagnitude)?,
            );
            if value == 0 {
                return Ok(0);
            }
            let bonus = needed(a.source_damage_multiplier_percent)?
                .checked_add(hit.bonus_percent)
                .ok_or(Error::InvalidMagnitude)?;
            value = compound(value, bonus)?
                .checked_add(i64::from(needed(a.wheel_flat_damage)?))
                .ok_or(Error::InvalidMagnitude)?;
            return self.finish_combat_mastery(value, &target);
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
            value = percent(
                value,
                self.delayed_damage_percent.unwrap_or(damage_dealt_percent),
            )?;
        } else if let Some(percent_at_cast) = self.delayed_damage_percent {
            value = percent(value, percent_at_cast)?;
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
        // Crystal monster.cpp:949-964: bow Ballistic subtracts two resistance
        // points, including Physical; it is not the generic elemental-pierce cap.
        let ballistic = matches!(element, Element::Physical | Element::Holy)
            && a.wheel_effects
                .as_ref()
                .is_some_and(|effects| effects.active_unique_perks.contains("ballistic_mastery"))
            && self
                .caster
                .wheel_equipment()
                .and_then(|hands| hands.ammunition())
                == Some(oteryn_gameplay_content::content::ReferenceAmmoType::Arrow);
        let mut sensitivity = original + if ballistic { 2.0 } else { 0.0 };
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
        self.finish_combat_mastery(
            value.checked_add(flat).ok_or(Error::InvalidMagnitude)?,
            &target,
        )
    }

    fn critical_extra(&self) -> Result<i32, Error> {
        let a = &self.resolved_attributes;
        let mut bonus = needed(a.equipment_critical_extra_permyriad)?
            .checked_add(needed(a.base_critical_extra_permyriad)?)
            .ok_or(Error::InvalidMagnitude)?;
        if let Some(ConditionValues::Attributes {
            critical_extra_percentage_points,
            ..
        }) = condition_attributes(&self.caster, self.now_ms)?
        {
            bonus = bonus
                .checked_add(
                    i32::try_from(
                        critical_extra_percentage_points
                            .checked_mul(100)
                            .ok_or(Error::InvalidMagnitude)?,
                    )
                    .map_err(|_| Error::InvalidMagnitude)?,
                )
                .ok_or(Error::InvalidMagnitude)?;
        }
        if let Some(effects) = &a.wheel_effects {
            let global = effects.gem_value_milli("critical_extra_damage", "Critical Extra Damage");
            let spell = self.selected_spell.as_deref().map_or(0, |name| {
                effects.gem_value_milli("critical_extra_damage", name)
            });
            // Thousandths percent -> hundredths percent. Global fractions survive;
            // the projection has already truncated each spell-specific int bonus.
            let extra = global.checked_add(spell).ok_or(Error::InvalidMagnitude)? / 10;
            bonus = bonus
                .checked_add(i32::try_from(extra).map_err(|_| Error::InvalidMagnitude)?)
                .ok_or(Error::InvalidMagnitude)?;
        }
        // Crystal player_wheel.cpp:3048-3055 contributes 1000 permyriad with bolts.
        if a.wheel_effects
            .as_ref()
            .is_some_and(|effects| effects.active_unique_perks.contains("ballistic_mastery"))
            && self
                .caster
                .wheel_equipment()
                .and_then(|hands| hands.ammunition())
                == Some(oteryn_gameplay_content::content::ReferenceAmmoType::Bolt)
        {
            bonus = bonus.checked_add(1000).ok_or(Error::InvalidMagnitude)?;
        }
        Ok(bonus)
    }

    fn target_health(target: &Target) -> Result<(u64, u64), Error> {
        match &target.owner {
            TargetOwner::Player(state) => Ok((
                u64::from(state.health),
                u64::from(state.character_facts().max_health),
            )),
            TargetOwner::Creature(state) => Ok((
                u64::try_from(state.health).map_err(|_| Error::InvalidMagnitude)?,
                u64::try_from(state.maximum_health).map_err(|_| Error::InvalidMagnitude)?,
            )),
        }
    }

    fn finish_combat_mastery(&self, value: i64, target: &Target) -> Result<i64, Error> {
        let Some(effects) = &self.resolved_attributes.wheel_effects else {
            return Ok(value);
        };
        let (health, maximum) = Self::target_health(target)?;
        let two_handed = self
            .caster
            .wheel_equipment()
            .is_some_and(|hands| hands.two_handed());
        let value = compound(
            value,
            effects.combat_mastery_percent(health, maximum, two_handed),
        )?;
        self.finish_sanctuary(value, target.actor)
    }

    fn finish_sanctuary(&self, mut value: i64, target: ExactActorRef) -> Result<i64, Error> {
        if !self
            .resolved_attributes
            .wheel_effects
            .as_ref()
            .is_some_and(|effects| effects.active_unique_perks.contains("sanctuary"))
        {
            return Ok(value);
        }
        let factor = self.caster.wheel_sanctuary_percent(
            self.selected_spell.as_deref(),
            self.now_ms.saturating_mul(1000),
        );
        value = signed_round(value as f64 * f64::from(factor) / 100.0)?;
        let caster = self.attributes.binding.actor;
        if caster != target {
            let at = self
                .wheel_positions
                .iter()
                .find(|(actor, _)| *actor == caster)
                .ok_or(Error::InvalidBatch)?
                .1
                .position();
            let to = self
                .wheel_positions
                .iter()
                .find(|(actor, _)| *actor == target)
                .ok_or(Error::InvalidBatch)?
                .1
                .position();
            if at.floor == to.floor && at.x.abs_diff(to.x) <= 1 && at.y.abs_diff(to.y) <= 1 {
                value = signed_round(value as f64 * 1.10)?;
            }
        }
        Ok(value)
    }

    fn finish_wheel_healing(
        &mut self,
        mut value: i64,
        target: &Target,
        draw: &mut dyn FnMut(i64, i64) -> i64,
    ) -> Result<i64, Error> {
        let a = &self.resolved_attributes;
        let Some(effects) = &a.wheel_effects else {
            return Ok(value);
        };
        // A named real spell/rune is required. Potion, Gift and link echoes never select it.
        if self.selected_spell.is_some() && effects.active_unique_perks.contains("battle_healing") {
            // Crystal game.cpp:7608-7617: shield triples the 10% bonus; historical
            // official reference data's doubled bonus is deliberately not executable here.
            let multiplier = if self
                .caster
                .wheel_equipment()
                .is_some_and(|hands| hands.real_shield())
            {
                130
            } else {
                110
            };
            value = signed_round(value as f64 * f64::from(multiplier) / 100.0)?;
        }
        if effects.active_unique_perks.contains("healing_link")
            && matches!(
                self.selected_spell.as_deref(),
                Some("Nature's Embrace" | "Heal Friend")
            )
        {
            // Crystal wheel:3507-3513,3881-3888/game:7622-7626. Echo is based on
            // the uncapped value and reenters as an unnamed heal: no Battle/Grove critical.
            let echo = value.checked_mul(25).ok_or(Error::InvalidMagnitude)? / 100;
            if echo > 0 {
                let echo = echo
                    .checked_add(i64::from(needed(a.wheel_flat_healing)?))
                    .ok_or(Error::InvalidMagnitude)?;
                let echo = self.finish_sanctuary(echo, a.binding.actor)?;
                self.wheel_link_echo = self
                    .wheel_link_echo
                    .checked_add(echo)
                    .ok_or(Error::InvalidMagnitude)?;
            }
        }
        if effects
            .revelation_stages
            .get("blessing_of_the_grove")
            .copied()
            .unwrap_or(0)
            > 0
        {
            let (health, maximum) = Self::target_health(target)?;
            let bonus =
                effects.grove_healing_bonus_milli(health, maximum, target.actor == a.binding.actor);
            value = signed_truncate(value as f64 + value as f64 * bonus as f64 / 100_000.0)?;
            if self.selected_spell.is_some() {
                let mut chance = u32::from(needed(a.equipment_critical_chance_permyriad)?);
                if let Some(ConditionValues::Attributes {
                    critical_chance_percent,
                    ..
                }) = condition_attributes(&self.caster, self.now_ms)?
                    && critical_chance_percent > 0
                {
                    chance = critical_chance_percent
                        .checked_mul(100)
                        .ok_or(Error::InvalidMagnitude)?;
                }
                chance = chance
                    .checked_add(u32::from(needed(a.base_critical_chance_permyriad)?))
                    .ok_or(Error::InvalidMagnitude)?;
                // Crystal uses a 1..100 draw multiplied by 100, per healed target, no fatal roll.
                if chance > 0 && checked_draw(draw, 1, 100)? * 100 <= i64::from(chance) {
                    value = signed_truncate(
                        value as f64 * (1.0 + f64::from(self.critical_extra()?) / 10000.0),
                    )?;
                }
            }
        }
        self.finish_sanctuary(value.max(0), target.actor)
    }

    /// Crystal combat.cpp:197-208 and wheel:3510 read these for each real Combat.
    /// Retaining the source's raw/stance snapshot must not erase fresh Wheel bonuses.
    fn selected_wheel_damage_bonus(&self) -> Result<u32, Error> {
        let Some(spell) = self.selected_spell.as_deref() else {
            return Ok(0);
        };
        let gem = self.attributes.wheel_effects.as_ref().map_or(0, |effects| {
            effects.gem_value_milli("base_damage_bonus", spell) / 1_000
        });
        u32::try_from(gem)
            .map_err(|_| Error::InvalidMagnitude)?
            .checked_add(
                self.caster.wheel_focus_damage_percent(
                    self.now_ms
                        .checked_mul(1_000)
                        .ok_or(Error::InvalidMagnitude)?,
                ),
            )
            .ok_or(Error::InvalidMagnitude)
    }

    /// Commit-time caster successor for an actual Combat, including a delayed strike.
    /// A marker-only scheduled cast has no Combat and does not call this method.
    pub fn apply_wheel_cast_state(&self, next: &mut PlayerSpellState) -> Result<(), Error> {
        if self.selected_spell.is_some() {
            next.complete_wheel_focus_cast(
                self.selected_focus_group,
                self.now_ms.checked_mul(1_000).ok_or(Error::InvalidBatch)?,
            )
            .ok_or(Error::InvalidBatch)?;
        }
        Ok(())
    }

    /// Lower the link echo through the same atomic batch and replay receipt as its parent heal.
    pub fn append_wheel_healing_echo(
        &mut self,
        lowered: &mut super::combat_batch::LoweredNativeCombat,
    ) -> Result<(), Error> {
        use super::combat_batch::{OwnerCombatChange, OwnerCombatEffect};
        if self.wheel_link_echo == 0 {
            return Ok(());
        }
        let batch = &mut lowered.batch;
        if batch.caster != self.attributes.binding.actor
            || batch.command.game_session_id() != self.attributes.binding.session
        {
            return Err(Error::InvalidBatch);
        }
        if usize::from(lowered.next_sub_ordinal) >= MAX_EFFECTS {
            return Err(Error::TooManyEffects);
        }
        batch.effects.push(OwnerCombatEffect {
            target: batch.caster,
            sub_ordinal: lowered.next_sub_ordinal,
            change: OwnerCombatChange::Heal {
                target_atom: super::combat_execution::actor_atom(batch.caster),
                magnitude: self.wheel_link_echo,
            },
        });
        lowered.next_sub_ordinal = lowered
            .next_sub_ordinal
            .checked_add(1)
            .ok_or(Error::TooManyEffects)?;
        self.wheel_link_echo = 0;
        Ok(())
    }
}

fn checked_draw(draw: &mut dyn FnMut(i64, i64) -> i64, min: i64, max: i64) -> Result<i64, Error> {
    let value = draw(min, max);
    if min > max || !(min..=max).contains(&value) {
        return Err(Error::InvalidMagnitude);
    }
    Ok(value)
}
