//! C08: the actor's persistent conditions across logout and a fresh login.
//!
//! Crystal 201189f8 saves every condition whose `Condition::isPersistent()` holds
//! (`src/io/functions/iologindata_save_player.cpp:264`), loads them
//! (`iologindata_load_player.cpp:287`) and re-adds each at login (`player.cpp:11565`).
//! `isPersistent()` (`condition.cpp:356`) keeps a condition with a finite duration of the default
//! or combat id; an infinite one (`ticks == -1`) is not saved. The restart keeps the remaining
//! time (`ConditionSpeed::serialize` the drawn delta, `ConditionDamage::serialize` the remaining
//! damage and `delayed`, so a non-delayed damage over time deals its next tick again on login).
//!
//! What is saved here, from a live instance with a finite remaining time (or, for a damage over
//! time, a remaining total): a timed status, a gate, haste and paralysis (the drawn delta), a
//! damage over time and agony of a fixed per-tick amount, recovery, light (its current decayed
//! level), spell light, spell regeneration, attributes, invisibility, spell skills and a plain
//! outfit. Restoring re-applies each as a fresh application of its own definition with the
//! remaining duration.
//!
//! Not saved, and so absent after a fresh login: food regeneration (BED-REGEN already restores
//! it from the row's `rest`), an infinite instance (Crystal does not save `ticks == -1` either),
//! a damage sequence or damage schedule (their step cursor), a mana shield (its remaining
//! capacity), source attributes (bound to their source), an item outfit, and any definition
//! that carries an appearance selection, an item appearance or a damage schedule.
use super::cast::PlayerSpellState;
use oteryn_gameplay_domain::domain::condition::{
    ConditionGate, ConditionValues, DotElement, SpeedRange, StatusKind,
};
use oteryn_gameplay_durability::durability::character_last_position::{
    MAX_SAVED_CONDITIONS, SavedConditions,
};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionInstance, ConditionSourceKind,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedCondition {
    key: String,
    revision: u32,
    values: SavedValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum SavedValues {
    TimedStatus {
        kind: SavedStatus,
        duration_ms: u32,
    },
    Gate {
        gate: SavedGate,
        duration_ms: u32,
    },
    /// Haste or paralysis with the delta drawn at its application.
    Speed {
        paralysis: bool,
        delta: i32,
        duration_ms: u32,
    },
    DamageOverTime {
        element: SavedElement,
        total: u32,
        per_tick: u32,
        interval_ms: u32,
        delayed: bool,
    },
    Agony {
        total: u32,
        per_tick: u32,
        interval_ms: u32,
        delayed: bool,
    },
    Recovery {
        duration_ms: u32,
        interval_ms: u32,
    },
    Light {
        level: u8,
        duration_ms: u32,
    },
    SpellLight {
        level: u8,
        color: u8,
        duration_ms: u32,
    },
    SpellRegeneration {
        duration_ms: u32,
        sub_id: u8,
        health_gain: u32,
        health_interval_ms: u32,
        mana_gain: u32,
        mana_interval_ms: u32,
    },
    Attributes {
        duration_ms: u32,
        critical_chance_percent: u32,
        critical_extra_percentage_points: u32,
        damage_dealt_percent: u32,
        incoming_reduction_percent: u32,
    },
    Invisible {
        duration_ms: u32,
    },
    SpellSkills {
        duration_ms: u32,
        sub_id: u8,
        magic_level: i32,
        fist: i32,
        melee: i32,
        distance: i32,
        shielding: i32,
    },
    Outfit {
        duration_ms: u32,
        look_type: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedStatus {
    Invisible,
    Drunk,
    Rooted,
    Feared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedGate {
    Muted,
    Pacified,
    Powerless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedElement {
    Poison,
    Fire,
    Energy,
    Bleeding,
    Drown,
    Freezing,
    Dazzled,
    Cursed,
}

impl SavedStatus {
    const fn of(kind: StatusKind) -> Self {
        match kind {
            StatusKind::Invisible => Self::Invisible,
            StatusKind::Drunk => Self::Drunk,
            StatusKind::Rooted => Self::Rooted,
            StatusKind::Feared => Self::Feared,
        }
    }
    const fn kind(self) -> StatusKind {
        match self {
            Self::Invisible => StatusKind::Invisible,
            Self::Drunk => StatusKind::Drunk,
            Self::Rooted => StatusKind::Rooted,
            Self::Feared => StatusKind::Feared,
        }
    }
}

impl SavedGate {
    const fn of(gate: ConditionGate) -> Self {
        match gate {
            ConditionGate::Muted => Self::Muted,
            ConditionGate::Pacified => Self::Pacified,
            ConditionGate::Powerless => Self::Powerless,
        }
    }
    const fn gate(self) -> ConditionGate {
        match self {
            Self::Muted => ConditionGate::Muted,
            Self::Pacified => ConditionGate::Pacified,
            Self::Powerless => ConditionGate::Powerless,
        }
    }
}

impl SavedElement {
    const fn of(element: DotElement) -> Self {
        match element {
            DotElement::Poison => Self::Poison,
            DotElement::Fire => Self::Fire,
            DotElement::Energy => Self::Energy,
            DotElement::Bleeding => Self::Bleeding,
            DotElement::Drown => Self::Drown,
            DotElement::Freezing => Self::Freezing,
            DotElement::Dazzled => Self::Dazzled,
            DotElement::Cursed => Self::Cursed,
        }
    }
    const fn element(self) -> DotElement {
        match self {
            Self::Poison => DotElement::Poison,
            Self::Fire => DotElement::Fire,
            Self::Energy => DotElement::Energy,
            Self::Bleeding => DotElement::Bleeding,
            Self::Drown => DotElement::Drown,
            Self::Freezing => DotElement::Freezing,
            Self::Dazzled => DotElement::Dazzled,
            Self::Cursed => DotElement::Cursed,
        }
    }
}

/// The saved form of one live instance at `now_us`, or `None` when Crystal would not persist it
/// or this slice does not carry it (module docs).
fn saved_of(instance: &ConditionInstance<String>, now_us: u64) -> Option<SavedCondition> {
    let definition = instance.definition();
    if definition.source_damage_schedule().is_some()
        || definition.source_appearance_selection().is_some()
        || definition.source_item_appearance().is_some()
    {
        return None;
    }
    // A finite remaining time; `-1` is an infinite instance (and every damage over time, which
    // ends with its total instead).
    let remaining = instance.remaining_duration_ms(now_us);
    let duration_ms = remaining
        .and_then(|ms| u32::try_from(ms).ok())
        .filter(|ms| *ms > 0);
    let values = match definition.values() {
        ConditionValues::TimedStatus { kind, .. } => {
            duration_ms.map(|duration_ms| SavedValues::TimedStatus {
                kind: SavedStatus::of(kind),
                duration_ms,
            })
        }
        ConditionValues::Gate { gate, .. } => duration_ms.map(|duration_ms| SavedValues::Gate {
            gate: SavedGate::of(gate),
            duration_ms,
        }),
        ConditionValues::Speed { paralysis, .. }
        | ConditionValues::RationalSpeed { paralysis, .. } => {
            let delta = i32::try_from(instance.speed_delta()).ok()?;
            duration_ms.map(|duration_ms| SavedValues::Speed {
                paralysis,
                delta,
                duration_ms,
            })
        }
        ConditionValues::DamageOverTime {
            element,
            per_tick,
            interval_ms,
            delayed,
            ..
        } => (remaining == Some(-1) && instance.remaining_total() > 0).then(|| {
            SavedValues::DamageOverTime {
                element: SavedElement::of(element),
                total: instance.remaining_total(),
                per_tick,
                interval_ms,
                delayed,
            }
        }),
        ConditionValues::Agony {
            per_tick,
            interval_ms,
            delayed,
            ..
        } => {
            (remaining == Some(-1) && instance.remaining_total() > 0).then(|| SavedValues::Agony {
                total: instance.remaining_total(),
                per_tick,
                interval_ms,
                delayed,
            })
        }
        ConditionValues::Recovery { interval_ms, .. } => {
            duration_ms.map(|duration_ms| SavedValues::Recovery {
                duration_ms,
                interval_ms,
            })
        }
        ConditionValues::Light { .. } => {
            let level = instance.light_level(now_us);
            duration_ms
                .filter(|_| level > 0)
                .map(|duration_ms| SavedValues::Light { level, duration_ms })
        }
        ConditionValues::SpellLight { level, color, .. } => {
            duration_ms.map(|duration_ms| SavedValues::SpellLight {
                level,
                color,
                duration_ms,
            })
        }
        ConditionValues::SpellRegeneration {
            sub_id,
            health_gain,
            health_interval_ms,
            mana_gain,
            mana_interval_ms,
            ..
        } => duration_ms.map(|duration_ms| SavedValues::SpellRegeneration {
            duration_ms,
            sub_id,
            health_gain,
            health_interval_ms,
            mana_gain,
            mana_interval_ms,
        }),
        ConditionValues::Attributes {
            critical_chance_percent,
            critical_extra_percentage_points,
            damage_dealt_percent,
            incoming_reduction_percent,
            ..
        } => duration_ms.map(|duration_ms| SavedValues::Attributes {
            duration_ms,
            critical_chance_percent,
            critical_extra_percentage_points,
            damage_dealt_percent,
            incoming_reduction_percent,
        }),
        ConditionValues::Invisible { .. } => {
            duration_ms.map(|duration_ms| SavedValues::Invisible { duration_ms })
        }
        ConditionValues::SpellSkills {
            sub_id,
            magic_level,
            fist,
            melee,
            distance,
            shielding,
            ..
        } => duration_ms.map(|duration_ms| SavedValues::SpellSkills {
            duration_ms,
            sub_id,
            magic_level,
            fist,
            melee,
            distance,
            shielding,
        }),
        ConditionValues::Outfit { look_type, .. } => {
            duration_ms.map(|duration_ms| SavedValues::Outfit {
                duration_ms,
                look_type,
            })
        }
        ConditionValues::DamageSchedule { .. }
        | ConditionValues::DamageSequence { .. }
        | ConditionValues::FoodRegeneration { .. }
        | ConditionValues::ManaShield { .. }
        | ConditionValues::SourceAttributes { .. }
        | ConditionValues::ItemOutfit { .. } => None,
    }?;
    Some(SavedCondition {
        key: definition.key().to_owned(),
        revision: definition.revision(),
        values,
    })
}

/// The definition one saved entry re-applies, or `None` for one no definition admits.
fn definition_of(saved: &SavedCondition) -> Option<ConditionDefinition> {
    let values = match saved.values {
        SavedValues::TimedStatus { kind, duration_ms } => ConditionValues::TimedStatus {
            kind: kind.kind(),
            duration_ms,
        },
        SavedValues::Gate { gate, duration_ms } => ConditionValues::Gate {
            gate: gate.gate(),
            duration_ms,
        },
        // A fixed target of `base + delta`: `(1000 * (base - 40) + (40 + delta) * 1000) / 1000`
        // in the speed draw, whatever the base at login, as Crystal re-applies its `speedDelta`.
        SavedValues::Speed {
            paralysis,
            delta,
            duration_ms,
        } => {
            let b = delta.checked_add(40)?;
            ConditionValues::Speed {
                paralysis,
                range: SpeedRange {
                    a_min: 1000,
                    b_min: b,
                    a_max: 1000,
                    b_max: b,
                },
                duration_ms,
            }
        }
        SavedValues::DamageOverTime {
            element,
            total,
            per_tick,
            interval_ms,
            delayed,
        } => ConditionValues::DamageOverTime {
            element: element.element(),
            total_min: total,
            total_max: total,
            per_tick,
            interval_ms,
            delayed,
        },
        SavedValues::Agony {
            total,
            per_tick,
            interval_ms,
            delayed,
        } => ConditionValues::Agony {
            total_min: total,
            total_max: total,
            per_tick,
            interval_ms,
            delayed,
        },
        SavedValues::Recovery {
            duration_ms,
            interval_ms,
        } => ConditionValues::Recovery {
            duration_ms,
            interval_ms,
        },
        SavedValues::Light { level, duration_ms } => ConditionValues::Light { level, duration_ms },
        SavedValues::SpellLight {
            level,
            color,
            duration_ms,
        } => ConditionValues::SpellLight {
            level,
            color,
            duration_ms,
        },
        SavedValues::SpellRegeneration {
            duration_ms,
            sub_id,
            health_gain,
            health_interval_ms,
            mana_gain,
            mana_interval_ms,
        } => ConditionValues::SpellRegeneration {
            duration_ms,
            sub_id,
            health_gain,
            health_interval_ms,
            mana_gain,
            mana_interval_ms,
        },
        SavedValues::Attributes {
            duration_ms,
            critical_chance_percent,
            critical_extra_percentage_points,
            damage_dealt_percent,
            incoming_reduction_percent,
        } => ConditionValues::Attributes {
            duration_ms,
            critical_chance_percent,
            critical_extra_percentage_points,
            damage_dealt_percent,
            incoming_reduction_percent,
        },
        SavedValues::Invisible { duration_ms } => ConditionValues::Invisible { duration_ms },
        SavedValues::SpellSkills {
            duration_ms,
            sub_id,
            magic_level,
            fist,
            melee,
            distance,
            shielding,
        } => ConditionValues::SpellSkills {
            duration_ms,
            sub_id,
            magic_level,
            fist,
            melee,
            distance,
            shielding,
        },
        SavedValues::Outfit {
            duration_ms,
            look_type,
        } => ConditionValues::Outfit {
            duration_ms,
            look_type,
        },
    };
    ConditionDefinition::new(&saved.key, saved.revision, values)
}

impl PlayerSpellState {
    /// The persistent conditions to store with the final logout write at `now_us` (semantic
    /// microseconds), or `None` when there are none or they do not fit the 0691 bounds.
    #[must_use]
    pub fn saved_conditions_snapshot(&self, now_us: u64) -> Option<SavedConditions> {
        let saved = self
            .conditions
            .instances()
            .iter()
            .filter_map(|instance| saved_of(instance, now_us))
            .take(MAX_SAVED_CONDITIONS)
            .collect::<Vec<_>>();
        if saved.is_empty() {
            return None;
        }
        SavedConditions::new(serde_json::to_string(&saved).ok()?)
    }

    /// Re-apply `saved` on a freshly created actor at `now_us`, each as a fresh self
    /// application of its definition with its remaining duration. An entry that no definition
    /// admits any more, or that the condition policy refuses, is dropped; the rest still apply.
    /// Returns how many applied, or `None` for undecodable text.
    pub fn restore_saved_conditions(
        &mut self,
        saved: &SavedConditions,
        now_us: u64,
    ) -> Option<usize> {
        let entries = serde_json::from_str::<Vec<SavedCondition>>(saved.as_str()).ok()?;
        // Restoration replays no random decision: a speed range is a single point here and no
        // other restored definition draws. This inert context grants no authority.
        let root = GameplayDecisionRoot::from_bytes([0; 32]);
        let base_speed = u16::try_from(self.base_speed).ok()?;
        let mut applied = 0;
        for definition in entries.iter().filter_map(definition_of) {
            let facts = ApplicationFacts {
                now: now_us,
                base_speed,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
            };
            if self
                .conditions
                .apply(&definition, None, ConditionSourceKind::SelfUse, &[], &facts)
                .is_ok()
            {
                applied += 1;
            }
        }
        Some(applied)
    }
}

#[cfg(test)]
#[path = "saved_conditions_tests.rs"]
mod tests;
