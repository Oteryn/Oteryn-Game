//! Exact f32 caster-speed application into the existing condition owner store.
//! Included as a child of `condition` so it extends that store, not a second actor
//! runtime. The caller must own the actor and qualify the native spell profile.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::{
    ApplicationFacts, Applied, ConditionDefinition, ConditionRefusal, ConditionSourceKind,
    ConditionStore, ConditionType, ConditionValues,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpellSpeedError {
    UnsupportedFormula,
    ArithmeticBounds,
    Admission(ConditionRefusal),
}

impl<S: Clone> ConditionStore<S> {
    /// Caster `ConditionSpeed` uses f32 multiplication and addition followed by
    /// truncation. The ordinary store uses decimal thousandths; the frozen content
    /// definition stays unchanged, while this admission fixes the source f32 delta.
    /// No draw, mutation or deadline is accepted before all bounds checks pass.
    pub(crate) fn apply_spell_haste(
        &mut self,
        definition: &ConditionDefinition,
        source: Option<S>,
        immunities: &[ConditionType],
        facts: &ApplicationFacts<'_>,
    ) -> Result<Applied, SpellSpeedError> {
        let ConditionValues::Speed {
            paralysis: false,
            range,
            duration_ms,
        } = definition.values()
        else {
            return Err(SpellSpeedError::UnsupportedFormula);
        };
        if range.a_min != range.a_max || range.b_min != range.b_max || duration_ms == 0 {
            return Err(SpellSpeedError::UnsupportedFormula);
        }
        // Semantic time is supplied by the owner. A past application relative to
        // its already processed pass or still-present instances cannot replace it.
        if facts.now < self.pass_at || self.instances.iter().any(|v| v.started_at > facts.now) {
            return Err(SpellSpeedError::ArithmeticBounds);
        }
        facts
            .now
            .checked_add(u64::from(duration_ms) * 1000)
            .ok_or(SpellSpeedError::ArithmeticBounds)?;
        let a = range.a_min as f32 / 1000.0_f32;
        let b = range.b_min as f32;
        let product = a * (i32::from(facts.base_speed) - 40) as f32;
        let total = f64::from(product + b).trunc();
        if !total.is_finite() || total < f64::from(i32::MIN) || total > f64::from(i32::MAX) {
            return Err(SpellSpeedError::ArithmeticBounds);
        }
        let delta = (total as i32)
            .checked_sub(i32::from(facts.base_speed))
            .ok_or(SpellSpeedError::ArithmeticBounds)?;
        delta
            .checked_neg()
            .ok_or(SpellSpeedError::ArithmeticBounds)?;
        let mut staged = self.clone();
        let applied = staged
            .apply(
                definition,
                source,
                ConditionSourceKind::SelfUse,
                immunities,
                facts,
            )
            .map_err(SpellSpeedError::Admission)?;
        let instance = staged
            .instances
            .iter_mut()
            .find(|v| {
                v.sequence == applied.sequence
                    && v.definition.condition_type() == ConditionType::Haste
            })
            .ok_or(SpellSpeedError::ArithmeticBounds)?;
        instance.speed_delta = i64::from(delta);
        *self = staged;
        Ok(applied)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::super::{SpeedRange, TickFacts};
    use super::*;
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

    fn haste(a: i32, b: i32, duration_ms: u32) -> ConditionDefinition {
        ConditionDefinition::new(
            &format!("spell.haste.a{a}.b{b}.d{duration_ms}"),
            1,
            ConditionValues::Speed {
                paralysis: false,
                range: SpeedRange {
                    a_min: a,
                    b_min: b,
                    a_max: a,
                    b_max: b,
                },
                duration_ms,
            },
        )
        .unwrap()
    }
    #[test]
    fn actual_store_uses_source_f32_replaces_and_expires_at_exact_deadline() {
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let mut facts = ApplicationFacts {
            now: 1000,
            base_speed: 400,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        let mut store = ConditionStore::<u32>::new();
        let first = store
            .apply_spell_haste(&haste(1700, 40, 22000), Some(7), &[], &facts)
            .unwrap();
        assert!(!first.replaced);
        assert_eq!(store.speed_delta(), 252);
        facts.now = 1000000;
        let weak = haste(1300, 40, 30000);
        let refresh = store
            .apply_spell_haste(&weak, Some(7), &[], &facts)
            .unwrap();
        assert!(refresh.replaced);
        assert_eq!(store.speed_delta(), 107);
        assert_eq!(store.instances().len(), 1);
        assert_eq!(store.instances()[0].definition(), &weak);
        assert_eq!(store.instances()[0].provenance().source, Some(7));
        assert!(store.take_due(30999999, TickFacts::default()).is_empty());
        assert_eq!(store.speed_delta(), 107);
        assert!(store.take_due(31000000, TickFacts::default()).is_empty());
        assert_eq!(store.speed_delta(), 0);
        assert!(store.instances().is_empty());
    }
    #[test]
    fn immunity_and_bounds_refuse_without_any_store_mutation() {
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let mut facts = ApplicationFacts {
            now: 0,
            base_speed: 400,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        let definition = haste(1300, 40, 30000);
        let mut store = ConditionStore::<u32>::new();
        let before = store.clone();
        assert_eq!(
            store.apply_spell_haste(&definition, Some(1), &[ConditionType::Haste], &facts),
            Err(SpellSpeedError::Admission(ConditionRefusal::Immune))
        );
        assert_eq!(store, before);
        facts.now = u64::MAX;
        assert_eq!(
            store.apply_spell_haste(&definition, Some(1), &[], &facts),
            Err(SpellSpeedError::ArithmeticBounds)
        );
        assert_eq!(store, before);
        facts.now = 0;
        let range = ConditionDefinition::new(
            "random",
            1,
            ConditionValues::Speed {
                paralysis: false,
                range: SpeedRange {
                    a_min: 1300,
                    a_max: 1700,
                    b_min: 40,
                    b_max: 40,
                },
                duration_ms: 30000,
            },
        )
        .unwrap();
        assert_eq!(
            store.apply_spell_haste(&range, Some(1), &[], &facts),
            Err(SpellSpeedError::UnsupportedFormula)
        );
        assert_eq!(store, before);
    }
    #[test]
    fn replacing_paralysis_uses_existing_speed_conflict_key() {
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 400,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        let paralysis = ConditionDefinition::new(
            "paralysis",
            1,
            ConditionValues::Speed {
                paralysis: true,
                range: SpeedRange {
                    a_min: 0,
                    a_max: 0,
                    b_min: 40,
                    b_max: 40,
                },
                duration_ms: 10000,
            },
        )
        .unwrap();
        let mut store = ConditionStore::<u32>::new();
        store
            .apply(
                &paralysis,
                Some(2),
                ConditionSourceKind::Creature,
                &[],
                &facts,
            )
            .unwrap();
        assert_eq!(store.speed_delta(), -360);
        store
            .apply_spell_haste(&haste(1900, 40, 5000), Some(1), &[], &facts)
            .unwrap();
        assert_eq!(store.speed_delta(), 324);
        assert_eq!(store.instances().len(), 1);
        assert_eq!(
            store.instances()[0].definition().condition_type(),
            ConditionType::Haste
        );
    }
}

impl<S: Clone> ConditionStore<S> {
    pub(crate) fn active_at(
        &self,
        key: super::ConflictKey,
        now: u64,
    ) -> Option<&super::ConditionInstance<S>> {
        self.get(key)
            .filter(|v| v.started_at <= now && v.ends_at.is_none_or(|end| now < end))
    }
    pub(crate) fn speed_delta_at(&self, now: u64) -> i64 {
        self.active_at(super::ConflictKey::Speed, now)
            .map_or(0, |v| v.speed_delta)
    }
    pub(crate) fn light_at(&self, now: u64) -> Option<(u8, Option<u8>)> {
        let instance = self.active_at(super::ConflictKey::Light, now)?;
        let (level, color, duration_ms) = match instance.definition.values {
            ConditionValues::Light { level, duration_ms } => (level, None, duration_ms),
            ConditionValues::SpellLight {
                level,
                color,
                duration_ms,
            } => (level, Some(color), duration_ms),
            _ => return None,
        };
        if level == 0 {
            return Some((0, color));
        }
        // CONDITIONS0 owns linear decay over the condition's duration. Read
        // semantic owner time without consuming or dropping unrelated DOT ticks.
        let interval = match instance.definition.values {
            // Source ConditionLight stores the interval in whole milliseconds.
            ConditionValues::SpellLight { .. } => {
                (u64::from(duration_ms) / u64::from(level)).max(1) * 1000
            }
            _ => (u64::from(duration_ms) * 1000 / u64::from(level)).max(1),
        };
        let decayed = (now - instance.started_at) / interval;
        Some((
            level.saturating_sub(u8::try_from(decayed).unwrap_or(u8::MAX)),
            color,
        ))
    }

    pub(crate) fn attributes_at(&self, now: u64) -> Option<ConditionValues> {
        self.active_at(super::ConflictKey::Attributes, now)
            .map(|v| v.definition.values)
    }
    pub(crate) fn outfit_at(&self, now: u64) -> Option<u32> {
        match self
            .active_at(super::ConflictKey::Outfit, now)?
            .definition
            .values
        {
            ConditionValues::Outfit { look_type, .. } => Some(look_type),
            _ => None,
        }
    }
    pub(crate) fn displayed_temporary_appearance_at(
        &self,
        now: u64,
    ) -> Option<super::TemporaryDisplayedAppearance<'_>> {
        let definition = &self.active_at(super::ConflictKey::Outfit, now)?.definition;
        match definition.values {
            ConditionValues::ItemOutfit { .. } => definition
                .item_appearance
                .as_ref()
                .map(super::TemporaryDisplayedAppearance::Item),
            ConditionValues::Outfit { .. } => definition
                .appearance_selection
                .as_ref()
                .map(super::TemporaryDisplayedAppearance::Outfit),
            _ => None,
        }
    }
    pub(crate) fn appearance_at(
        &self,
        now: u64,
    ) -> Option<&crate::domain::appearance::AppearanceSelection<String>> {
        self.active_at(super::ConflictKey::Outfit, now)?
            .definition
            .appearance_selection
            .as_ref()
    }
    pub(crate) fn remove_type(&mut self, kind: ConditionType) -> bool {
        let before = self.instances.len();
        self.instances
            .retain(|v| v.definition.condition_type() != kind);
        before != self.instances.len()
    }
    pub(crate) fn mana_shield_at(&self, now: u64) -> Option<u32> {
        self.active_at(super::ConflictKey::ManaShield, now)
            .map(|v| v.mana_shield_remaining)
    }
    /// The player facade also proves and pays the correlated mana amount atomically.
    pub(crate) fn consume_mana_shield(
        &mut self,
        now: u64,
        expected: u32,
        amount: u32,
    ) -> Result<(), SpellSpeedError> {
        if amount == 0 || self.mana_shield_at(now) != Some(expected) || amount > expected {
            return Err(SpellSpeedError::ArithmeticBounds);
        }
        let Some(v) = self
            .instances
            .iter_mut()
            .find(|v| v.definition.condition_type() == ConditionType::ManaShield)
        else {
            return Err(SpellSpeedError::ArithmeticBounds);
        };
        v.mana_shield_remaining -= amount;
        if v.mana_shield_remaining == 0 {
            self.remove_type(ConditionType::ManaShield);
        }
        Ok(())
    }
    /// Apply the familiar's already source-evaluated f64 explicit delta. The
    /// physical companion owner binds the recipient and its base speed separately.
    pub(crate) fn apply_explicit_spell_speed(
        &mut self,
        definition: &ConditionDefinition,
        delta: i32,
        source: Option<S>,
        immunities: &[ConditionType],
        facts: &ApplicationFacts<'_>,
    ) -> Result<Applied, SpellSpeedError> {
        if !matches!(definition.values(), ConditionValues::Speed { .. })
            || delta.checked_neg().is_none()
        {
            return Err(SpellSpeedError::UnsupportedFormula);
        }
        let mut staged = self.clone();
        let applied = staged
            .apply(
                definition,
                source,
                ConditionSourceKind::SelfUse,
                immunities,
                facts,
            )
            .map_err(SpellSpeedError::Admission)?;
        let Some(v) = staged.instances.iter_mut().find(|v| {
            v.sequence == applied.sequence
                && v.definition.condition_type() == definition.condition_type()
        }) else {
            return Err(SpellSpeedError::ArithmeticBounds);
        };
        v.speed_delta = i64::from(delta);
        *self = staged;
        Ok(applied)
    }
}

impl<S: Clone> ConditionStore<S> {
    pub(crate) fn accepts_time(&self, now: u64) -> bool {
        now >= self.pass_at && self.instances.iter().all(|v| v.started_at <= now)
    }
}

impl<S: Clone> ConditionStore<S> {
    /// Expire only non-ticking effects when tile legality is not yet qualified.
    /// Pending DOT/regeneration ticks remain owned and are never silently used up.
    /// Unlike the lifecycle owner's `expire_non_ticking`, this covers every spell-owned
    /// effect without a pending tick and refuses a time before the store's last pass.
    pub(crate) fn expire_non_ticking_checked(&mut self, now: u64) -> Result<bool, SpellSpeedError> {
        if !self.accepts_time(now) {
            return Err(SpellSpeedError::ArithmeticBounds);
        }
        let before = (self.instances.len(), self.cleanse_immunities.len());
        self.instances.retain(|v| {
            v.next_tick_at.is_some()
                || v.immediate_due.is_some()
                || v.ends_at.is_none_or(|end| end > now)
        });
        self.cleanse_immunities
            .retain(|immunity| now < immunity.until);
        Ok(before != (self.instances.len(), self.cleanse_immunities.len()))
    }
}

/// Closed current modifier result. Durable progression/base skills are never rewritten.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SkillAdjustments {
    pub(crate) magic_level: i32,
    pub(crate) fist: i32,
    pub(crate) melee: i32,
    pub(crate) distance: i32,
    pub(crate) shielding: i32,
}
impl SkillAdjustments {
    pub(crate) fn adjust_magic_level(self, base: u32) -> Option<u32> {
        adjust(base, self.magic_level)
    }
    pub(crate) fn adjust_shielding(self, base: u32) -> Option<u32> {
        adjust(base, self.shielding)
    }
    pub(crate) fn adjust_skill(self, base: u32, index: usize) -> Option<u32> {
        let delta = match index {
            0 => self.fist,
            1..=3 => self.melee,
            4 => self.distance,
            5 => self.shielding,
            6 => 0,
            _ => return None,
        };
        adjust(base, delta)
    }
}
fn adjust(base: u32, delta: i32) -> Option<u32> {
    u32::try_from((i64::from(base) + i64::from(delta)).max(0)).ok()
}
impl<S: Clone> ConditionStore<S> {
    pub(crate) fn invisible_at(&self, now: u64) -> bool {
        self.active_at(super::ConflictKey::Invisible, now).is_some()
    }
    pub(crate) fn skill_adjustments(&self, now: u64) -> Option<SkillAdjustments> {
        if now < self.pass_at || self.instances.iter().any(|v| v.started_at > now) {
            return None;
        }
        let mut result = SkillAdjustments::default();
        for id in 0..=3 {
            if let Some(instance) = self.active_at(super::ConflictKey::SpellSkills(id), now) {
                if let ConditionValues::SpellSkills {
                    magic_level,
                    fist,
                    melee,
                    distance,
                    shielding,
                    ..
                } = instance.definition.values()
                {
                    result.magic_level = result.magic_level.checked_add(magic_level)?;
                    result.fist = result.fist.checked_add(fist)?;
                    result.melee = result.melee.checked_add(melee)?;
                    result.distance = result.distance.checked_add(distance)?;
                    result.shielding = result.shielding.checked_add(shielding)?;
                } else {
                    return None;
                }
            }
        }
        Some(result)
    }
}

#[cfg(test)]
mod item_appearance_tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::super::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind, TemporaryDisplayedAppearance,
    };
    use super::*;
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    #[test]
    fn temporary_item_selection_never_becomes_an_outfit_unlock_and_expires_exactly() {
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let facts = ApplicationFacts {
            now: 1000,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        let bare = ConditionDefinition::new(
            "spell.chameleon",
            1,
            ConditionValues::ItemOutfit { duration_ms: 10 },
        )
        .unwrap();
        let mut store = ConditionStore::<String>::new();
        assert!(
            store
                .apply(&bare, None, ConditionSourceKind::Player, &[], &facts)
                .is_err()
        );
        assert!(
            bare.clone()
                .with_item_appearance("oteryn:item.tibia.i3264", "definition-r1", [0; 32])
                .is_none()
        );
        let definition = bare
            .with_item_appearance("oteryn:item.tibia.i3264", "definition-r1", [4; 32])
            .unwrap();
        store
            .apply(
                &definition,
                Some("actual-caster".into()),
                ConditionSourceKind::Player,
                &[],
                &facts,
            )
            .unwrap();
        let Some(TemporaryDisplayedAppearance::Item(selection)) =
            store.displayed_temporary_appearance_at(10_999)
        else {
            panic!("qualified source Item appearance missing")
        };
        assert_eq!(selection.definition_key(), "oteryn:item.tibia.i3264");
        assert_eq!(selection.revision_ref(), "definition-r1");
        assert_eq!(selection.artifact_digest(), [4; 32]);
        assert!(store.outfit_at(10_999).is_none());
        assert!(store.appearance_at(10_999).is_none());
        assert!(store.displayed_temporary_appearance_at(11_000).is_none());
    }
}
