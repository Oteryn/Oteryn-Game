//! Source conditions lower into the actual PlayerSpellState owner. Creature slots are separate.
use super::cast::PlayerSpellState;
use crate::foundation as source;
use oteryn_gameplay_domain::domain::condition as values;
use oteryn_gameplay_domain::domain::condition::attributes;
use oteryn_gameplay_foundation::foundation::condition as native;

/// Vibrancy uses the incoming condition's real decision stream and the current equipment read.
/// A repeated PvP paralysis is deflected and the existing paralysis is removed.
pub fn deflect_equipped_paralysis<S: Clone>(
    store: &mut native::ConditionStore<S>,
    definition: &native::ConditionDefinition,
    percent: u8,
    player_source: bool,
    facts: &native::ApplicationFacts<'_>,
) -> Result<bool, values::ConditionRefusal> {
    if percent == 0
        || !facts.target_is_player
        || facts.target_reentry_protected
        || definition.condition_type() != values::ConditionType::Paralysis
    {
        return Ok(false);
    }
    if !store.accepts_time(facts.now) || percent > 100 {
        return Err(values::ConditionRefusal::ArithmeticBounds);
    }
    let repeated = player_source
        && store.instances().iter().any(|instance| {
            instance.definition().condition_type() == values::ConditionType::Paralysis
                && instance
                    .remaining_duration_ms(facts.now)
                    .is_some_and(|remaining| remaining != 0)
        });
    let roll = oteryn_simulation_determinism::deterministic_decision_u64(
        facts.decision_root,
        facts.occurrence,
        "oteryn.imbuement.vibrancy.v1",
        0,
    )
    .map_err(|_| values::ConditionRefusal::DrawFailed)?
        % 100
        + 1;
    if repeated || roll <= u64::from(percent) {
        store.remove_type(values::ConditionType::Paralysis);
        return Ok(true);
    }
    Ok(false)
}

fn element(value: source::DotElement) -> values::DotElement {
    match value {
        source::DotElement::Poison => values::DotElement::Poison,
        source::DotElement::Fire => values::DotElement::Fire,
        source::DotElement::Energy => values::DotElement::Energy,
        source::DotElement::Bleeding => values::DotElement::Bleeding,
        source::DotElement::Drown => values::DotElement::Drown,
        source::DotElement::Freezing => values::DotElement::Freezing,
        source::DotElement::Dazzled => values::DotElement::Dazzled,
        source::DotElement::Cursed => values::DotElement::Cursed,
    }
}
fn modifier(value: Option<source::AttributeModifier>) -> Option<attributes::AttributeModifier> {
    value.map(|value| match value {
        source::AttributeModifier::Add(v) => attributes::AttributeModifier::Add(v),
        source::AttributeModifier::PercentOfBase(v) => {
            attributes::AttributeModifier::PercentOfBase(v)
        }
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
pub fn definition(value: &source::ConditionDefinition) -> Option<native::ConditionDefinition> {
    use source::ConditionValues as S;
    use values::ConditionValues as N;
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
                source::StatusKind::Drunk => values::StatusKind::Drunk,
                source::StatusKind::Rooted => values::StatusKind::Rooted,
                source::StatusKind::Feared => values::StatusKind::Feared,
                source::StatusKind::Invisible => values::StatusKind::Invisible,
            },
            duration_ms,
        },
        S::SourceAttributes {
            modifiers,
            duration_ms,
        } => N::SourceAttributes {
            modifiers: attributes::AttributeModifiers {
                melee: modifier(modifiers.melee),
                distance: modifier(modifiers.distance),
                magic_points: modifier(modifiers.magic_points),
                shielding: modifier(modifiers.shielding),
                fishing: modifier(modifiers.fishing),
            },
            duration_ms,
        },
        S::RationalSpeed {
            paralysis,
            range,
            duration_ms,
        } => N::RationalSpeed {
            paralysis,
            range: values::RationalSpeedRange {
                a_min: values::ExactSpeedRatio {
                    numerator: range.a_min.numerator,
                    denominator: range.a_min.denominator,
                },
                b_min: range.b_min,
                a_max: values::ExactSpeedRatio {
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
            range: values::SpeedRange {
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
        // CONDITIONS-0 §3.5: the gate families by their content name, and `agony`.
        S::Gate { gate, duration_ms } => N::gate(gate.key(), duration_ms)?,
        S::Agony {
            total_min,
            total_max,
            per_tick,
            interval_ms,
            delayed,
        } => N::Agony {
            total_min,
            total_max,
            per_tick,
            interval_ms,
            delayed,
        },
        _ => return None,
    };
    native::ConditionDefinition::new(value.key(), value.revision(), values)
}
fn immunity(kind: source::ConditionType) -> Option<values::ConditionType> {
    use source::ConditionType as S;
    use values::ConditionType as N;
    Some(match kind {
        S::Status(source::StatusKind::Invisible) => N::Invisible,
        S::Status(source::StatusKind::Drunk) => N::Status(values::StatusKind::Drunk),
        S::Status(source::StatusKind::Rooted) => N::Status(values::StatusKind::Rooted),
        S::Status(source::StatusKind::Feared) => N::Status(values::StatusKind::Feared),
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
    pub fn stage_source_player_conditions(
        &self,
        source: String,
        definitions: &[source::ConditionDefinition],
        immunities: &[source::ConditionType],
        facts: &source::ApplicationFacts<'_>,
    ) -> Option<Self> {
        self.stage_source_player_conditions_with_history(
            source,
            definitions,
            immunities,
            facts,
            None,
        )
    }
    /// The same canonical owner application, with frozen non-authoritative death
    /// display provenance. The condition instance itself bounds its lifetime.
    pub fn stage_source_player_conditions_with_history(
        &self,
        source: String,
        definitions: &[source::ConditionDefinition],
        immunities: &[source::ConditionType],
        facts: &source::ApplicationFacts<'_>,
        historical_source: Option<&native::ConditionHistoricalSource>,
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
            if deflect_equipped_paralysis(
                &mut next.conditions,
                &definition,
                self.equipped_offence.paralysis_deflection_percent,
                false,
                &facts,
            )
            .ok()?
            {
                continue;
            }
            let applied = match historical_source {
                Some(historical) => next.conditions.apply_with_historical_source(
                    &definition,
                    Some(source.clone()),
                    native::ConditionSourceKind::Creature,
                    &immunities,
                    &facts,
                    historical,
                ),
                None => next.conditions.apply(
                    &definition,
                    Some(source.clone()),
                    native::ConditionSourceKind::Creature,
                    &immunities,
                    &facts,
                ),
            };
            match applied {
                Ok(_) => {}
                Err(
                    values::ConditionRefusal::Immune
                    | values::ConditionRefusal::ReentryProtected
                    | values::ConditionRefusal::KeptCurrent,
                ) => {}
                Err(_) => return None,
            }
        }
        if next.conditions != self.conditions {
            next.revision = self.revision.checked_add(1)?
        }
        Some(next)
    }
    pub fn stage_source_remove_invisibility(&self, now: u64) -> Option<Self> {
        if self.health == 0 || !self.conditions.accepts_time(now) {
            return None;
        }
        let mut next = self.clone();
        let changed = next
            .conditions
            .remove_type(values::ConditionType::Invisible)
            | next
                .conditions
                .remove_type(values::ConditionType::Status(values::StatusKind::Invisible));
        if changed {
            next.revision = self.revision.checked_add(1)?
        }
        Some(next)
    }
}
impl PlayerSpellState {
    pub fn source_take_due(
        &mut self,
        now: u64,
        facts: native::TickFacts,
    ) -> Vec<native::ConditionTick<String>> {
        super::food_regeneration::step_due(self, now, facts.in_protection_zone);
        self.conditions.take_due(now, facts)
    }
    pub fn source_set_batch_revision(&mut self, revision: u64) {
        self.revision = revision;
    }
}

#[cfg(any(test, feature = "test-support"))]
impl PlayerSpellState {
    // Pure typed fixture stage only. No runtime authority and no captured creature numbers.
    pub fn stage_owned_test_conditions(
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
    /// Stage only this field's application in the canonical store. The owner
    /// qualifies and receipts every returned hit before publishing the successor.
    pub fn stage_player_field_entry(
        &self,
        source: String,
        definition: &native::ConditionDefinition,
        lineage: &native::ConditionLineage,
        facts: &native::ApplicationFacts<'_>,
        element: impl Into<Option<values::DotElement>>,
    ) -> Option<(Self, Vec<native::ConditionTick<String>>)> {
        if self.health == 0
            || !facts.target_is_player
            || !source.starts_with("player:field:")
            || !self.conditions.accepts_time(facts.now)
        {
            return None;
        }
        let mut next = self.clone();
        match next.conditions.apply_with_lineage(
            definition,
            Some(source.clone()),
            native::ConditionSourceKind::Field,
            &[],
            facts,
            lineage,
        ) {
            Ok(_) => {}
            Err(
                values::ConditionRefusal::Immune
                | values::ConditionRefusal::ReentryProtected
                | values::ConditionRefusal::KeptCurrent,
            ) => return Some((self.clone(), Vec::new())),
            Err(_) => return None,
        }
        let ticks = next.conditions.take_due_source_application(
            facts.now,
            native::TickFacts {
                in_protection_zone: false,
                standing_on_field: element.into(),
            },
            &source,
        );
        Some((next, ticks))
    }

    /// Environmental fields have no creature or player caster. The movement
    /// owner separately qualifies the field and the current receiver/destination.
    pub fn stage_environment_field_contact(
        &self,
        source: String,
        definition: &native::ConditionDefinition,
        facts: &native::ApplicationFacts<'_>,
        element: impl Into<Option<values::DotElement>>,
        stage_hit: impl FnMut(
            &Self,
            u32,
            values::DotElement,
            u64,
        ) -> Option<(Self, crate::ability::creature_bite::FlooredDamage)>,
    ) -> Option<(Self, u32)> {
        self.stage_field_contact(
            source,
            native::ConditionSourceKind::Field,
            definition,
            facts,
            element,
            stage_hit,
        )
    }

    /// Consume only this admitted Creature field's application tick. Overdue
    /// combat/regeneration ticks retain their counters until their own owner pass.
    pub fn stage_creature_field_contact(
        &self,
        source: String,
        definition: &native::ConditionDefinition,
        facts: &native::ApplicationFacts<'_>,
        element: impl Into<Option<values::DotElement>>,
        stage_hit: impl FnMut(
            &Self,
            u32,
            values::DotElement,
            u64,
        ) -> Option<(Self, crate::ability::creature_bite::FlooredDamage)>,
    ) -> Option<(Self, u32)> {
        self.stage_field_contact(
            source,
            native::ConditionSourceKind::Creature,
            definition,
            facts,
            element,
            stage_hit,
        )
    }

    fn stage_field_contact(
        &self,
        source: String,
        source_kind: native::ConditionSourceKind,
        definition: &native::ConditionDefinition,
        facts: &native::ApplicationFacts<'_>,
        element: impl Into<Option<values::DotElement>>,
        mut stage_hit: impl FnMut(
            &Self,
            u32,
            values::DotElement,
            u64,
        )
            -> Option<(Self, crate::ability::creature_bite::FlooredDamage)>,
    ) -> Option<(Self, u32)> {
        let valid_source = match source_kind {
            native::ConditionSourceKind::Creature => source.starts_with("creature:field:"),
            native::ConditionSourceKind::Field => source.starts_with("environment:"),
            _ => false,
        };
        if self.health == 0
            || !facts.target_is_player
            || !valid_source
            || !self.conditions.accepts_time(facts.now)
        {
            return None;
        }
        let mut next = self.clone();
        match next
            .conditions
            .apply(definition, Some(source.clone()), source_kind, &[], facts)
        {
            Ok(_) => {}
            Err(
                values::ConditionRefusal::Immune
                | values::ConditionRefusal::ReentryProtected
                | values::ConditionRefusal::KeptCurrent,
            ) => return Some((self.clone(), 0)),
            Err(_) => return None,
        }
        let ticks = next.conditions.take_due_source_application(
            facts.now,
            native::TickFacts {
                in_protection_zone: false,
                standing_on_field: element.into(),
            },
            &source,
        );
        let mut damage = 0u32;
        for tick in ticks {
            if tick.provenance.source_kind != source_kind
                || tick.provenance.source.as_deref() != Some(source.as_str())
            {
                return None;
            }
            let (successor, hit) = match tick.kind {
                native::TickKind::Damage {
                    element,
                    amount,
                    refused: false,
                } => stage_hit(&next, amount, element, facts.now)?,
                native::TickKind::Agony {
                    amount,
                    refused: false,
                } => super::actor_conditions::stage_agony_hit(&next, amount, facts.now).ok()?,
                _ => return None,
            };
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
    fn qn1_environment_energy_field_2126_deals_its_source_entry_hit() {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tools/content-schema/spell-authoring/samples/native-field-profiles.json"
        ))
        .unwrap();
        let record = document["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["source"]["server"] == "crystal" && r["source"]["server_item_id"] == 2126)
            .unwrap();
        let recipe: oteryn_gameplay_content::content::field_condition::QualifiedFieldCondition =
            serde_json::from_value(record.clone()).unwrap();
        let definition = native::ConditionDefinition::new(
            "native.field.i2126",
            1,
            recipe.condition_values().unwrap().unwrap(),
        )
        .unwrap();
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
        .unwrap();
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([9; 32]);
        let facts = native::ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([9; 16]),
        };
        let source = "environment:field:i2126".to_owned();
        assert!(
            state
                .stage_creature_field_contact(
                    source.clone(),
                    &definition,
                    &facts,
                    recipe.element().unwrap(),
                    |_, _, _, _| None
                )
                .is_none()
        );
        let (next, damage) = state
            .stage_environment_field_contact(
                source,
                &definition,
                &facts,
                recipe.element().unwrap(),
                |before, amount, element, now| {
                    assert_eq!(element, values::DotElement::Energy);
                    super::super::actor_conditions::stage_creature_hit(before, amount, now).ok()
                },
            )
            .expect("source energy field must accept an environmental cause");
        assert_eq!(damage, 100);
        assert_eq!(next.vitals().health, 900);
        assert_eq!(state.vitals().health, 1000, "staging preserves the input");
        assert!(
            next.owned_conditions()
                .clone()
                .take_due(5_000_000, native::TickFacts::default())
                .is_empty(),
            "2126 has one entry, not an invented residual DOT"
        );
        assert!(
            state
                .stage_environment_field_contact(
                    "creature:field:i2126".into(),
                    &definition,
                    &facts,
                    recipe.element().unwrap(),
                    |_, _, _, _| None
                )
                .is_none()
        );
    }

    #[test]
    fn qn1_environment_drown_keeps_field_provenance_and_delayed_ticks() {
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
        .unwrap();
        let definition = native::ConditionDefinition::new(
            "environment.drowning",
            1,
            values::ConditionValues::DamageOverTime {
                element: values::DotElement::Drown,
                total_min: 100,
                total_max: 100,
                per_tick: 20,
                interval_ms: 4000,
                delayed: true,
            },
        )
        .unwrap();
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([9; 32]);
        let facts = native::ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([9; 16]),
        };
        let (next, damage) = state
            .stage_environment_field_contact(
                "environment:drowning".into(),
                &definition,
                &facts,
                Some(values::DotElement::Drown),
                |_, _, _, _| None,
            )
            .unwrap();
        assert_eq!(damage, 0);
        assert_eq!(next.vitals().health, 1000);
        let mut conditions = next.owned_conditions().clone();
        assert!(
            conditions
                .take_due(3_999_999, native::TickFacts::default())
                .is_empty()
        );
        let ticks = conditions.take_due(4_000_000, native::TickFacts::default());
        assert_eq!(ticks.len(), 1);
        assert_eq!(
            ticks[0].provenance.source_kind,
            native::ConditionSourceKind::Field
        );
        assert_eq!(
            ticks[0].provenance.source.as_deref(),
            Some("environment:drowning")
        );
        assert!(ticks[0].provenance.lineage.is_none());
        assert!(matches!(
            ticks[0].kind,
            native::TickKind::Damage {
                element: values::DotElement::Drown,
                amount: 20,
                refused: false
            }
        ));
    }

    #[test]
    fn c08_agony_field_43297_applies_entry_and_residual_damage_bypassing_mana_shield() {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tools/content-schema/spell-authoring/samples/native-field-profiles.json"
        ))
        .unwrap();
        let record = document["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["source"]["server"] == "crystal" && r["source"]["server_item_id"] == 43297)
            .unwrap();
        let recipe: oteryn_gameplay_content::content::field_condition::QualifiedFieldCondition =
            serde_json::from_value(record.clone()).unwrap();
        let definition = native::ConditionDefinition::new(
            "native.field.i43297",
            1,
            recipe.condition_values().unwrap().unwrap(),
        )
        .unwrap();
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([9; 32]);
        let facts = native::ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 500,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([9; 16]),
        };
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
        .unwrap();
        let shield = native::ConditionDefinition::new(
            "test.c08.shield",
            1,
            values::ConditionValues::ManaShield {
                duration_ms: 60_000,
            },
        )
        .unwrap();
        let state = state
            .stage_owned_test_conditions(&[shield], &facts)
            .unwrap();
        let (next, damage) = state
            .stage_creature_field_contact(
                "creature:field:c08".into(),
                &definition,
                &facts,
                recipe.element().unwrap(),
                |_, _, _, _| None,
            )
            .expect("agony field contact");
        assert_eq!(damage, 260);
        assert_eq!(next.vitals().health, 740);
        assert_eq!(next.vitals().mana, 500);
        assert_eq!(
            next.owned_conditions()
                .get(values::ConflictKey::ManaShield)
                .unwrap()
                .mana_shield(),
            (500, 500)
        );
        let mut conditions = next.owned_conditions().clone();
        let ticks = conditions.take_due(5_000_000, native::TickFacts::default());
        assert_eq!(ticks.len(), 1);
        assert_eq!(
            ticks[0].kind,
            native::TickKind::Agony {
                amount: 259,
                refused: false
            }
        );
    }

    #[test]
    fn global_field_step_in_entry_then_every_residual_tick() {
        use oteryn_gameplay_content::content::field_condition::QualifiedFieldCondition;
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tools/content-schema/spell-authoring/samples/native-field-profiles.json"
        ))
        .unwrap();
        for (item, entry, interval_ms, runs) in [
            (2118, 20, 9_000, vec![(10, 7)]),
            (2119, 20, 9_000, vec![(10, 7)]),
            (2131, 20, 9_000, vec![(10, 7)]),
            (2132, 20, 9_000, vec![(10, 7)]),
            (2122, 30, 11_000, vec![(25, 1)]),
            (2135, 30, 11_000, vec![(25, 1)]),
            (105, 4, 4_000, vec![(4, 4), (3, 5), (2, 7), (1, 10)]),
            (2121, 4, 4_000, vec![(4, 4), (3, 5), (2, 7), (1, 10)]),
            (2134, 4, 4_000, vec![(4, 4), (3, 5), (2, 7), (1, 10)]),
            // Owner Global PKG1R4: 2123 separate entry 19 + 7x9 at 9 s.
            (2123, 19, 9_000, vec![(9, 7)]),
            // Owner Global PKG1R4: 2124 6x9 at 9 s, first on step-in;
            // no separate 19 entry hit, so only five ticks remain after entry.
            (2124, 9, 9_000, vec![(9, 5)]),
            // PKG1R6 owner Global 2026-10-30, 0% resistance: 1998 big
            // flames, entry 19 at 17:43:03 + seven ticks of 9 through 17:44:06.
            (1998, 19, 9_000, vec![(9, 7)]),
            // 1999 medium, 17:46:45..17:47:30: six ticks of 9, first on
            // step-in, so five remain; no separate entry hit.
            (1999, 9, 9_000, vec![(9, 5)]),
            // 2000 embers at 0% resistance: 17:36:18..17:37:03, six
            // ticks of 9, first on step-in, so five remain; look 17:36:17.
            (2000, 9, 9_000, vec![(9, 5)]),
            // PKG1R7: both donors give Zaoan fire the same record as 2119.
            (10069, 20, 9_000, vec![(10, 7)]),
            (10070, 20, 9_000, vec![(10, 7)]),
            (10071, 20, 9_000, vec![(10, 7)]),
        ] {
            let record = document["profiles"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| {
                    r["source"]["server"] == "canary" && r["source"]["server_item_id"] == item
                })
                .unwrap();
            let recipe: QualifiedFieldCondition = serde_json::from_value(record.clone()).unwrap();
            let element = recipe.element().unwrap().unwrap();
            let definition = native::ConditionDefinition::new(
                &format!("native.field.{item}"),
                1,
                recipe.condition_values().unwrap().unwrap(),
            )
            .unwrap();
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
            .unwrap();
            let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([1; 32]);
            let facts = native::ApplicationFacts {
                now: 0,
                base_speed: 100,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                    [1; 16],
                ),
            };
            // The actual player STEP contact rule consumes the entry immediately,
            // even while standing on the field. No equipment or PvP reduction here.
            let (mut state, damage) = state
                .stage_creature_field_contact(
                    format!("creature:field:{item}"),
                    &definition,
                    &facts,
                    element,
                    |state, amount, hit_element, now| {
                        assert_eq!(hit_element, element);
                        super::super::actor_conditions::stage_creature_hit(state, amount, now).ok()
                    },
                )
                .unwrap();
            assert_eq!(damage, entry, "entry {item}");
            assert_eq!(state.health, 1000 - entry, "entry HP {item}");
            // Leave the tile: drive the canonical condition clock directly so unrelated
            // natural regeneration cannot offset the HP assertions.
            let mut now_ms = 0;
            let mut total = entry;
            for (amount, count) in runs {
                for _ in 0..count {
                    now_ms += interval_ms;
                    assert!(
                        state
                            .conditions
                            .take_due((now_ms - 1) * 1_000, native::TickFacts::default())
                            .is_empty(),
                        "early tick {item}"
                    );
                    let ticks = state
                        .conditions
                        .take_due(now_ms * 1_000, native::TickFacts::default());
                    assert_eq!(ticks.len(), 1, "tick count {item} at {now_ms}");
                    assert!(
                        matches!(ticks[0].kind, native::TickKind::Damage {
                        amount: got, refused: false, ..
                    } if got == amount),
                        "tick {item} at {now_ms}: {ticks:?}"
                    );
                    let (next, hit) = super::super::actor_conditions::stage_creature_hit(
                        &state,
                        amount,
                        now_ms * 1_000,
                    )
                    .unwrap();
                    assert_eq!(hit.applied, amount);
                    state = next;
                    total += amount;
                    assert_eq!(state.health, 1000 - total, "tick HP {item}");
                }
            }
            assert!(
                state
                    .conditions
                    .take_due((now_ms + interval_ms) * 1_000, native::TickFacts::default())
                    .is_empty(),
                "extra tick {item}"
            );
            assert!(
                state
                    .conditions
                    .get(values::ConflictKey::Element(element))
                    .is_none()
            );
        }
    }
    #[test]
    fn player_field_entry_freezes_lineage_and_leaves_damage_to_the_live_owner() {
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
        let lineage = native::ConditionLineage {
            source_character: [1; 16],
            source_session: [2; 16],
            source_lease_generation: 2,
            source_actor_placement: [3; 16],
            source_scope_generation: 1,
            source_content_digest: [4; 32],
            source_catalog_digest: [5; 32],
            source_creation_command: 1,
        };
        let definition = native::ConditionDefinition::new(
            "native.field.2118",
            1,
            values::ConditionValues::DamageOverTime {
                element: values::DotElement::Fire,
                total_min: 140,
                total_max: 140,
                per_tick: 20,
                interval_ms: 10000,
                delayed: false,
            },
        )
        .expect("recipe");
        let (next, ticks) = state
            .stage_player_field_entry(
                "player:field:fixture".into(),
                &definition,
                &lineage,
                &facts,
                values::DotElement::Fire,
            )
            .expect("entry");
        assert_eq!(
            next.health, state.health,
            "no HP before the actual PvP owner qualifies and receipts the hit"
        );
        assert_eq!(ticks.len(), 1);
        assert_eq!(
            ticks[0].provenance.source_kind,
            native::ConditionSourceKind::Field
        );
        assert_eq!(ticks[0].provenance.lineage, Some(lineage.clone()));
        assert!(matches!(
            ticks[0].kind,
            native::TickKind::Damage {
                amount: 20,
                refused: false,
                ..
            }
        ));
        let mut invalid = lineage;
        invalid.source_lease_generation = 0;
        assert!(
            state
                .stage_player_field_entry(
                    "player:field:fixture".into(),
                    &definition,
                    &invalid,
                    &facts,
                    values::DotElement::Fire
                )
                .is_none()
        );
        assert!(
            state
                .stage_player_field_entry(
                    "creature:field:fixture".into(),
                    &definition,
                    &invalid,
                    &facts,
                    values::DotElement::Fire
                )
                .is_none()
        );
    }
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
            values::ConditionValues::DamageOverTime {
                element: values::DotElement::Poison,
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
            .get(values::ConflictKey::Element(values::DotElement::Poison))
            .expect("qualified fixture")
            .clone();
        let fresh = native::ConditionDefinition::new(
            "native.field.fresh-fire",
            1,
            values::ConditionValues::DamageOverTime {
                element: values::DotElement::Fire,
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
                values::DotElement::Fire,
                |state, amount, element, now| {
                    assert_eq!(element, values::DotElement::Fire);
                    // This pure fixture has no Battle Instinct or world actors.
                    // Keep the issued element in the ordinary block/final owners.
                    let amount = state.after_player_block(
                        amount,
                        crate::player_lethal::PlayerHitKind::from_dot(element),
                        0,
                        now,
                    )?;
                    super::super::actor_conditions::stage_preprocessed_player_hit(
                        state, amount, now,
                    )
                    .ok()
                },
            )
            .expect("qualified fixture");
        assert_eq!(damage, 20);
        assert_eq!(
            next.conditions
                .get(values::ConflictKey::Element(values::DotElement::Poison)),
            Some(&original)
        );
        assert_eq!(next.health, 980);
        assert_eq!(next.revision, state.revision + 1);
    }
}
