//! Source conditions lower into the actual PlayerSpellState owner. Creature slots are separate.
use super::cast::PlayerSpellState;
use crate::ability::condition as native;
use crate::foundation as source;

fn element(value: source::DotElement) -> native::DotElement {
    match value {
        source::DotElement::Poison => native::DotElement::Poison,
        source::DotElement::Fire => native::DotElement::Fire,
        source::DotElement::Energy => native::DotElement::Energy,
        source::DotElement::Bleeding => native::DotElement::Bleeding,
        source::DotElement::Drown => native::DotElement::Drown,
        source::DotElement::Freezing => native::DotElement::Freezing,
        source::DotElement::Dazzled => native::DotElement::Dazzled,
        source::DotElement::Cursed => native::DotElement::Cursed,
    }
}
fn modifier(value: Option<source::AttributeModifier>) -> Option<native::AttributeModifier> {
    value.map(|value| match value {
        source::AttributeModifier::Add(v) => native::AttributeModifier::Add(v),
        source::AttributeModifier::PercentOfBase(v) => native::AttributeModifier::PercentOfBase(v),
    })
}
fn schedule(value: &source::DamageSchedule) -> native::DamageSchedule {
    match value {
        source::DamageSchedule::Fixed { delayed, segments } => native::DamageSchedule::Fixed {
            delayed: *delayed,
            segments: segments
                .iter()
                .map(|s| native::DamageSegment {
                    count: s.count,
                    amount: s.amount,
                    interval_ms: s.interval_ms,
                })
                .collect(),
        },
        source::DamageSchedule::Decreasing {
            delayed,
            minimum,
            maximum,
            initial,
            interval_ms,
        } => native::DamageSchedule::Decreasing {
            delayed: *delayed,
            minimum: *minimum,
            maximum: *maximum,
            initial: *initial,
            interval_ms: *interval_ms,
        },
        source::DamageSchedule::Geometric {
            delayed,
            minimum,
            maximum,
            numerator,
            denominator,
            counts,
            interval_ms,
        } => native::DamageSchedule::Geometric {
            delayed: *delayed,
            minimum: *minimum,
            maximum: *maximum,
            numerator: *numerator,
            denominator: *denominator,
            counts: counts.clone(),
            interval_ms: *interval_ms,
        },
    }
}
pub(crate) fn definition(
    value: &source::ConditionDefinition,
) -> Option<native::ConditionDefinition> {
    use native::ConditionValues as N;
    use source::ConditionValues as S;
    let values = match value.values() {
        S::Outfit {
            duration_ms,
            look_type,
        } => {
            return native::ConditionDefinition::new(
                value.key(),
                value.revision(),
                N::Outfit {
                    duration_ms,
                    look_type,
                },
            )?
            .with_appearance(value.source_appearance_selection()?.clone());
        }
        S::ItemOutfit { duration_ms } => {
            let member = value.source_item_appearance()?;
            return native::ConditionDefinition::new(
                value.key(),
                value.revision(),
                N::ItemOutfit { duration_ms },
            )?
            .with_item_appearance(
                member.definition_key(),
                member.revision_ref(),
                member.artifact_digest(),
            );
        }
        S::DamageSchedule { element: e } => {
            return native::ConditionDefinition::new_damage_schedule(
                value.key(),
                value.revision(),
                element(e),
                schedule(value.source_damage_schedule()?),
            );
        }
        S::TimedStatus {
            kind: source::StatusKind::Invisible,
            duration_ms,
        } => N::Invisible { duration_ms },
        S::TimedStatus { kind, duration_ms } => N::TimedStatus {
            kind: match kind {
                source::StatusKind::Drunk => native::StatusKind::Drunk,
                source::StatusKind::Rooted => native::StatusKind::Rooted,
                source::StatusKind::Feared => native::StatusKind::Feared,
                source::StatusKind::Invisible => native::StatusKind::Invisible,
            },
            duration_ms,
        },
        S::SourceAttributes {
            modifiers,
            duration_ms,
        } => N::SourceAttributes {
            modifiers: native::AttributeModifiers {
                melee: modifier(modifiers.melee),
                distance: modifier(modifiers.distance),
                magic_points: modifier(modifiers.magic_points),
            },
            duration_ms,
        },
        S::RationalSpeed {
            paralysis,
            range,
            duration_ms,
        } => N::RationalSpeed {
            paralysis,
            range: native::RationalSpeedRange {
                a_min: native::ExactSpeedRatio {
                    numerator: range.a_min.numerator,
                    denominator: range.a_min.denominator,
                },
                b_min: range.b_min,
                a_max: native::ExactSpeedRatio {
                    numerator: range.a_max.numerator,
                    denominator: range.a_max.denominator,
                },
                b_max: range.b_max,
            },
            duration_ms,
        },
        S::Speed {
            paralysis,
            range,
            duration_ms,
        } => N::Speed {
            paralysis,
            range: native::SpeedRange {
                a_min: range.a_min,
                b_min: range.b_min,
                a_max: range.a_max,
                b_max: range.b_max,
            },
            duration_ms,
        },
        S::DamageOverTime {
            element: e,
            total_min,
            total_max,
            per_tick,
            interval_ms,
            delayed,
        } => N::DamageOverTime {
            element: element(e),
            total_min,
            total_max,
            per_tick,
            interval_ms,
            delayed,
        },
        S::Recovery {
            duration_ms,
            interval_ms,
        } => N::Recovery {
            duration_ms,
            interval_ms,
        },
        S::ManaShield { duration_ms } => N::ManaShield { duration_ms },
        S::Invisible { duration_ms } => N::Invisible { duration_ms },
        _ => return None,
    };
    native::ConditionDefinition::new(value.key(), value.revision(), values)
}
fn immunity(kind: source::ConditionType) -> Option<native::ConditionType> {
    use native::ConditionType as N;
    use source::ConditionType as S;
    Some(match kind {
        S::Status(source::StatusKind::Invisible) => N::Invisible,
        S::Status(source::StatusKind::Drunk) => N::Status(native::StatusKind::Drunk),
        S::Status(source::StatusKind::Rooted) => N::Status(native::StatusKind::Rooted),
        S::Status(source::StatusKind::Feared) => N::Status(native::StatusKind::Feared),
        S::Outfit => N::Outfit,
        S::Attributes => N::Attributes,
        S::Haste => N::Haste,
        S::Paralysis => N::Paralysis,
        S::DamageOverTime(e) => N::DamageOverTime(element(e)),
        S::ManaShield => N::ManaShield,
        S::Recovery => N::Recovery,
        S::Invisible => N::Invisible,
        _ => return None,
    })
}
impl PlayerSpellState {
    pub(crate) fn stage_source_player_conditions(
        &self,
        source: String,
        definitions: &[source::ConditionDefinition],
        immunities: &[source::ConditionType],
        facts: &source::ApplicationFacts<'_>,
    ) -> Option<Self> {
        if self.health == 0
            || !facts.target_is_player
            || definitions.is_empty()
            || definitions.len() > 16
            || !self.conditions.accepts_time(facts.now)
        {
            return None;
        }
        let definitions: Vec<_> = definitions.iter().map(definition).collect::<Option<_>>()?;
        let immunities: Vec<_> = immunities
            .iter()
            .copied()
            .map(immunity)
            .collect::<Option<_>>()?;
        let facts = native::ApplicationFacts {
            now: facts.now,
            base_speed: facts.base_speed,
            mana_shield_capacity: facts.mana_shield_capacity,
            target_reentry_protected: facts.target_reentry_protected,
            source_reentry_protected: facts.source_reentry_protected,
            target_is_player: facts.target_is_player,
            decision_root: facts.decision_root,
            occurrence: facts.occurrence,
        };
        let mut next = self.clone();
        for definition in definitions {
            match next.conditions.apply(
                &definition,
                Some(source.clone()),
                native::ConditionSourceKind::Creature,
                &immunities,
                &facts,
            ) {
                Ok(_) => {}
                Err(
                    native::ConditionRefusal::Immune
                    | native::ConditionRefusal::ReentryProtected
                    | native::ConditionRefusal::KeptCurrent,
                ) => {}
                Err(_) => return None,
            }
        }
        if next.conditions != self.conditions {
            next.revision = self.revision.checked_add(1)?
        }
        Some(next)
    }
    pub(crate) fn stage_source_remove_invisibility(&self, now: u64) -> Option<Self> {
        if self.health == 0 || !self.conditions.accepts_time(now) {
            return None;
        }
        let mut next = self.clone();
        let changed = next
            .conditions
            .remove_type(native::ConditionType::Invisible)
            | next
                .conditions
                .remove_type(native::ConditionType::Status(native::StatusKind::Invisible));
        if changed {
            next.revision = self.revision.checked_add(1)?
        }
        Some(next)
    }
}
impl PlayerSpellState {
    pub(crate) fn source_take_due(
        &mut self,
        now: u64,
        facts: native::TickFacts,
    ) -> Vec<native::ConditionTick<String>> {
        self.conditions.take_due(now, facts)
    }
    pub(crate) fn source_set_batch_revision(&mut self, revision: u64) {
        self.revision = revision;
    }
}

#[cfg(test)]
impl PlayerSpellState {
    // Pure typed fixture stage only. No runtime authority and no captured creature numbers.
    pub(crate) fn stage_owned_test_conditions(
        &self,
        defs: &[native::ConditionDefinition],
        facts: &native::ApplicationFacts<'_>,
    ) -> Option<Self> {
        let mut next = self.clone();
        for def in defs {
            next.conditions
                .apply(
                    def,
                    Some("creature:test.fixture".to_owned()),
                    native::ConditionSourceKind::Creature,
                    &[],
                    facts,
                )
                .ok()?;
        }
        next.revision = self.revision.checked_add(1)?;
        Some(next)
    }
}

impl PlayerSpellState {
    /// Sole canonical Player store: consume only this admitted field's application
    /// tick. Existing overdue combat/regeneration ticks retain their counters.
    pub(crate) fn stage_creature_field_contact(
        &self,
        source: String,
        definition: &native::ConditionDefinition,
        facts: &native::ApplicationFacts<'_>,
        element: native::DotElement,
    ) -> Option<(Self, u32)> {
        if self.health == 0
            || !facts.target_is_player
            || !source.starts_with("creature:field:")
            || !self.conditions.accepts_time(facts.now)
        {
            return None;
        }
        let mut next = self.clone();
        match next.conditions.apply(
            definition,
            Some(source.clone()),
            native::ConditionSourceKind::Creature,
            &[],
            facts,
        ) {
            Ok(_) => {}
            Err(
                native::ConditionRefusal::Immune
                | native::ConditionRefusal::ReentryProtected
                | native::ConditionRefusal::KeptCurrent,
            ) => return Some((self.clone(), 0)),
            Err(_) => return None,
        }
        let ticks = next.conditions.take_due_source_application(
            facts.now,
            native::TickFacts {
                in_protection_zone: false,
                standing_on_field: Some(element),
            },
            &source,
        );
        let mut damage = 0u32;
        for tick in ticks {
            let native::TickKind::Damage {
                amount,
                refused: false,
                ..
            } = tick.kind
            else {
                return None;
            };
            if tick.provenance.source_kind != native::ConditionSourceKind::Creature
                || tick.provenance.source.as_deref() != Some(source.as_str())
            {
                return None;
            }
            let (successor, hit) =
                super::actor_conditions::stage_creature_hit(&next, amount, facts.now).ok()?;
            damage = damage.checked_add(hit.applied)?;
            next = successor;
            if next.health == 0 {
                break;
            }
        }
        next.revision = if next != *self {
            self.revision.checked_add(1)?
        } else {
            self.revision
        };
        Some((next, damage))
    }
}

#[cfg(test)]
mod creature_field_initial_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    #[test]
    fn field_contact_does_not_consume_another_overdue_owned_dot() {
        let state = PlayerSpellState::new(
            super::super::cast::CharacterCastFacts {
                vocation: super::super::Vocation::Knight,
                level: 100,
                magic_level: 10,
                max_health: 1000,
                max_mana: 500,
                max_soul: 100,
            },
            0,
            0,
        )
        .expect("qualified fixture");
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([1; 32]);
        let facts = native::ApplicationFacts {
            now: 0,
            base_speed: 100,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([1; 16]),
        };
        let old = native::ConditionDefinition::new(
            "native.field.old-poison",
            1,
            native::ConditionValues::DamageOverTime {
                element: native::DotElement::Poison,
                total_min: 100,
                total_max: 100,
                per_tick: 10,
                interval_ms: 1000,
                delayed: false,
            },
        )
        .expect("qualified fixture");
        let mut state = state;
        state
            .conditions
            .apply(
                &old,
                Some("creature:old".to_owned()),
                native::ConditionSourceKind::Creature,
                &[],
                &facts,
            )
            .expect("qualified fixture");
        let original = state
            .conditions
            .get(native::ConflictKey::Element(native::DotElement::Poison))
            .expect("qualified fixture")
            .clone();
        let fresh = native::ConditionDefinition::new(
            "native.field.fresh-fire",
            1,
            native::ConditionValues::DamageOverTime {
                element: native::DotElement::Fire,
                total_min: 40,
                total_max: 40,
                per_tick: 20,
                interval_ms: 1000,
                delayed: false,
            },
        )
        .expect("qualified fixture");
        let (next, damage) = state
            .stage_creature_field_contact(
                "creature:field:fixture".into(),
                &fresh,
                &facts,
                native::DotElement::Fire,
            )
            .expect("qualified fixture");
        assert_eq!(damage, 20);
        assert_eq!(
            next.conditions
                .get(native::ConflictKey::Element(native::DotElement::Poison)),
            Some(&original)
        );
        assert_eq!(next.health, 980);
        assert_eq!(next.revision, state.revision + 1);
    }
}
