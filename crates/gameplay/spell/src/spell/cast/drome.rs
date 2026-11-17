//! W17 player effects: docs/reference/tibia-manual/combat.md:271-272,274,276-278.
//! The arena owner qualifies the participant and commits this runtime-only successor.
use super::PlayerSpellState;
use oteryn_gameplay_domain::domain::condition::{
    ConditionRefusal, ConditionType, ConditionValues, ConflictKey, StatusKind,
};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DromeStatus {
    Fear,
    Root,
    Drunk,
}

impl DromeStatus {
    const ALL: [Self; 3] = [Self::Fear, Self::Root, Self::Drunk];

    const fn facts(self) -> (StatusKind, &'static str, u32, u32) {
        // Manual combat.md:276-278: Tanked Up 10s/+10%, Sown Sorrow 2s/+15%, Bad Roots 3s/+25%.
        match self {
            Self::Fear => (StatusKind::Feared, "drome.feared", 2_000, 15),
            Self::Root => (StatusKind::Rooted, "drome.rooted", 3_000, 25),
            Self::Drunk => (StatusKind::Drunk, "drome.drunk", 10_000, 10),
        }
    }
}

impl PlayerSpellState {
    /// Manual combat.md:271-272,274: remove a fraction of CURRENT HP directly.
    /// Percentage arena effects bypass incoming bonuses, reductions and mana shields.
    pub fn after_drome_percentage_hit(&self, percent: u8) -> Option<Self> {
        if !self.drome_practice || self.health == 0 || !(1..100).contains(&percent) {
            return None;
        }
        let loss = u32::try_from(u64::from(self.health) * u64::from(percent) / 100).ok()?;
        let mut next = self.clone();
        next.health = next.health.checked_sub(loss)?;
        next.revision = next.revision.checked_add(1)?;
        Some(next)
    }

    /// Tests use a convenience wrapper; production distinguishes canonical refusals.
    #[cfg(test)]
    fn after_drome_status(&self, kind: DromeStatus, now_us: u64) -> Option<Self> {
        self.try_after_drome_status(kind, now_us).ok()
    }

    /// Canonical timed status: existing cast/movement gates, suppression and dispel apply.
    /// The arena resolves immunity/retained stronger status as a no-op, while clock
    /// and arithmetic failures retain the pending occurrence for a later owner turn.
    pub fn try_after_drome_status(
        &self,
        kind: DromeStatus,
        now_us: u64,
    ) -> Result<Self, ConditionRefusal> {
        if !self.drome_practice || self.health == 0 {
            return Err(ConditionRefusal::ArithmeticBounds);
        }
        let (status, key, duration_ms, _) = kind.facts();
        let definition = ConditionDefinition::new(
            key,
            1,
            ConditionValues::TimedStatus {
                kind: status,
                duration_ms,
            },
        )
        .ok_or(ConditionRefusal::ArithmeticBounds)?;
        // TimedStatus makes no draw. These inert bytes grant no authority; the arena
        // owner already proves exact actor/session membership before committing.
        let root = GameplayDecisionRoot::from_bytes([0; 32]);
        let facts = ApplicationFacts {
            now: now_us,
            base_speed: u16::try_from(self.base_speed)
                .map_err(|_| ConditionRefusal::ArithmeticBounds)?,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
        };
        let mut next = self.clone();
        next.conditions.apply(
            &definition,
            None,
            ConditionSourceKind::Creature,
            &[],
            &facts,
        )?;
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(ConditionRefusal::ArithmeticBounds)?;
        Ok(next)
    }

    pub(super) fn clear_drome_statuses(&mut self) {
        for kind in DromeStatus::ALL {
            let (status, key, _, _) = kind.facts();
            if self
                .conditions
                .get(ConflictKey::Status(status))
                .is_some_and(|instance| instance.definition().key() == key)
            {
                self.conditions.remove_type(ConditionType::Status(status));
            }
        }
    }

    pub(crate) fn after_drome_incoming_bonus(&self, magnitude: u32, now_us: u64) -> Option<u32> {
        if !self.drome_practice {
            return Some(magnitude);
        }
        let mut percent = 100_u32;
        for kind in DromeStatus::ALL {
            let (status, key, _, bonus) = kind.facts();
            if self
                .conditions
                .active_at(ConflictKey::Status(status), now_us)
                .is_some_and(|instance| instance.definition().key() == key)
            {
                percent += bonus;
            }
        }
        u32::try_from(u64::from(magnitude) * u64::from(percent) / 100).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::spell::actor_conditions::stage_preprocessed_player_hit;
    use oteryn_gameplay_domain::domain::condition::{ConditionType, StatusKind};
    use oteryn_simulation_determinism::SemanticTimeMicros;

    fn player() -> Result<PlayerSpellState, String> {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Druid,
                level: 100,
                magic_level: 50,
                max_health: 1_000,
                max_mana: 500,
                max_soul: 100,
            },
            0,
            0,
        )
        .ok_or("state")?;
        state.drome_practice = true;
        Ok(state)
    }

    #[test]
    fn drome_status_live_damage_bonus_expires_at_exact_microsecond() -> Result<(), String> {
        for (kind, status, duration, damage) in [
            (DromeStatus::Fear, StatusKind::Feared, 2_000_000, 115),
            (DromeStatus::Root, StatusKind::Rooted, 3_000_000, 125),
            (DromeStatus::Drunk, StatusKind::Drunk, 10_000_000, 110),
        ] {
            let before = player()?;
            let state = before.after_drome_status(kind, 123).ok_or("status")?;
            assert_eq!(state.revision(), before.revision() + 1);
            assert!(state.conditions.has_status(status, 123));
            assert!(state.conditions.has_status(status, 122 + duration));
            assert!(!state.conditions.has_status(status, 123 + duration));
            let (_, hit) = stage_preprocessed_player_hit(&state, 100, 122 + duration)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(hit.applied, damage);
            let mut shielded = state.clone();
            shielded.equipped_mana_shield = true;
            let (shielded, hit) =
                stage_preprocessed_player_hit(&shielded, 100, 123).map_err(|e| format!("{e:?}"))?;
            assert_eq!(hit.applied, 0);
            assert_eq!(shielded.mana, state.mana - 2 * damage);
            assert_eq!(shielded.health, state.health);
            let (_, hit) = stage_preprocessed_player_hit(&state, 100, 123 + duration)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(hit.applied, 100);
            let mut dispelled = state.clone();
            dispelled
                .conditions
                .remove_type(ConditionType::Status(status));
            let (_, hit) = stage_preprocessed_player_hit(&dispelled, 100, 123)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(hit.applied, 100);
            assert_eq!(before.conditions.instances().len(), 0);
        }
        Ok(())
    }

    #[test]
    fn drome_fear_closes_real_cast_gate_until_exact_expiry() -> Result<(), String> {
        let state = player()?
            .after_drome_status(DromeStatus::Fear, 123)
            .ok_or("fear")?;
        let book = v1_spell_book()?;
        let spell = book
            .indexed(std::num::NonZeroU32::new(1).ok_or("spell index")?)
            .ok_or("spell")?;
        assert_eq!(
            admit_condition_gates(&state, spell, SemanticTimeMicros::from_micros(2_000_122)),
            Err(oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Rejected)
        );
        assert_eq!(
            admit_condition_gates(&state, spell, SemanticTimeMicros::from_micros(2_000_123)),
            Ok(())
        );
        Ok(())
    }

    #[test]
    fn drome_percentage_hit_uses_current_hp_bypasses_shield_and_bonus() -> Result<(), String> {
        let mut state = player()?
            .after_drome_status(DromeStatus::Root, 0)
            .ok_or("root")?;
        state.health = 777;
        state.equipped_mana_shield = true;
        for (percent, expected) in [(25, 583), (40, 467), (60, 311)] {
            let next = state.after_drome_percentage_hit(percent).ok_or("hit")?;
            assert_eq!(next.health, expected);
            assert_eq!(next.mana, state.mana);
            assert_eq!(next.conditions, state.conditions);
            assert_eq!(next.revision(), state.revision() + 1);
        }
        state.health = 1;
        assert_eq!(
            state.after_drome_percentage_hit(60).ok_or("floor")?.health,
            1
        );
        assert!(state.after_drome_percentage_hit(0).is_none());
        assert!(state.after_drome_percentage_hit(100).is_none());
        state.drome_practice = false;
        assert!(state.after_drome_percentage_hit(25).is_none());
        assert!(state.after_drome_status(DromeStatus::Fear, 0).is_none());
        state.drome_practice = true;
        state.health = 0;
        assert!(state.after_drome_percentage_hit(25).is_none());
        assert!(state.after_drome_status(DromeStatus::Fear, 0).is_none());
        Ok(())
    }

    #[test]
    fn drome_exit_clears_modifier_status_and_overflows_do_not_mutate() -> Result<(), String> {
        let before = player()?;
        assert!(
            before
                .after_drome_status(DromeStatus::Root, u64::MAX)
                .is_none()
        );
        let mut state = before
            .after_drome_status(DromeStatus::Root, 0)
            .ok_or("root")?;
        let exited = state.after_drome_exit(800, 400).ok_or("exit")?;
        assert_eq!(
            (exited.health, exited.mana, exited.drome_practice),
            (800, 400, false)
        );
        assert!(!exited.conditions.has_status(StatusKind::Rooted, 1));
        state.revision = u64::MAX;
        assert!(state.after_drome_percentage_hit(25).is_none());
        assert!(state.after_drome_status(DromeStatus::Drunk, 0).is_none());
        Ok(())
    }

    #[test]
    fn drome_drunk_respects_equipment_suppression_and_foreign_status_is_not_bonus()
    -> Result<(), String> {
        use oteryn_gameplay_domain::domain::condition::ConditionValues;
        use oteryn_gameplay_foundation::foundation::condition::{
            ApplicationFacts, ConditionDefinition, ConditionSourceKind,
        };
        use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
        let mut state = player()?;
        state.conditions.set_suppressed(
            [ConditionType::Status(StatusKind::Drunk)]
                .into_iter()
                .collect(),
        );
        assert!(state.after_drome_status(DromeStatus::Drunk, 0).is_none());
        assert_eq!(
            state.try_after_drome_status(DromeStatus::Drunk, 0),
            Err(super::ConditionRefusal::Immune)
        );
        state.conditions.set_suppressed(Default::default());
        let root = GameplayDecisionRoot::from_bytes([0; 32]);
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 1,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
        };
        let definition = ConditionDefinition::new(
            "other.root",
            1,
            ConditionValues::TimedStatus {
                kind: StatusKind::Rooted,
                duration_ms: 20_000,
            },
        )
        .ok_or("definition")?;
        state
            .conditions
            .apply(
                &definition,
                None,
                ConditionSourceKind::Creature,
                &[],
                &facts,
            )
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            state.try_after_drome_status(DromeStatus::Root, 1),
            Err(super::ConditionRefusal::KeptCurrent)
        );
        let (_, hit) =
            stage_preprocessed_player_hit(&state, 100, 1).map_err(|e| format!("{e:?}"))?;
        assert_eq!(hit.applied, 100);
        let exited = state.after_drome_exit(800, 400).ok_or("exit")?;
        assert!(exited.conditions.has_status(StatusKind::Rooted, 1));
        Ok(())
    }
}
