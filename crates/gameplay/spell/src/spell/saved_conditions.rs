//! C08: the actor's persistent conditions across logout and a fresh login.
//!
//! Crystal 201189f8 saves every condition whose `Condition::isPersistent()` holds
//! (`src/io/functions/iologindata_save_player.cpp:264`), loads them
//! (`iologindata_load_player.cpp:287`) and re-adds each at login (`player.cpp:11565`).
//! `isPersistent()` (`condition.cpp:356`) keeps a condition with a finite duration of the default
//! or combat id; an infinite one (`ticks == -1`) is not saved. The restart keeps the remaining
//! time (`ConditionSpeed::serialize` the drawn delta, `ConditionDamage::serialize` the remaining
//! damage list and `delayed`, so a non-delayed damage over time deals its next tick again on
//! login). Crystal keeps every condition of the player in one list, so both Oteryn stores are
//! saved: the spell owner's and the runtime actor's.
//!
//! What is saved here, from a live instance with a finite remaining time (or, for damage, a
//! remaining total):
//! - a timed status, a gate, haste and paralysis (the drawn delta), recovery, light (its current
//!   decayed level), spell light, spell regeneration, attributes, invisibility and spell skills;
//! - every damage family with what it has still to deal: a damage over time and agony of a fixed
//!   per-tick amount (the remaining total), a damage sequence (its remaining steps), an agony
//!   ramp (its remaining start) and a damage schedule (its remaining segments), as
//!   `ConditionDamage::serialize` (`condition.cpp:1680`) writes the remaining `IntervalInfo`
//!   list. Crystal's load skips the owner (`ConditionDamage::unserializeProp`,
//!   `condition.cpp:1652`), so a restored damage has no attacker: it is restored with creature
//!   provenance under [`SAVED_DAMAGE_SOURCE`], which the player periodic owners deal like any
//!   creature damage;
//! - a mana shield with its cast capacity. `ConditionManaShield::serialize`
//!   (`condition.cpp:1496`) writes `manaShield`, the capacity of the cast, and `startCondition`
//!   sets the shield to it again: Crystal does NOT persist the remaining capacity, so a restored
//!   shield starts full;
//! - source attributes (`ConditionAttributes::serialize`, `condition.cpp:645`);
//! - an outfit, an item outfit and an outfit with an appearance selection
//!   (`ConditionOutfit::serialize`, `condition.cpp:2553`, writes the `Outfit_t`). An item
//!   outfit is restored only under the server artifact it was applied under;
//! - spell and group cooldowns: Crystal makes them conditions (`CONDITION_SPELLCOOLDOWN`,
//!   `CONDITION_SPELLGROUPCOOLDOWN`) of `CONDITIONID_DEFAULT` with finite ticks
//!   (`spells.cpp:789`, `800`, `818`), so `isPersistent()` saves them with the rest.
//!
//! Not saved, and so absent after a fresh login: food regeneration (BED-REGEN already restores
//! it from the row's `rest`), an infinite instance (Crystal does not save `ticks == -1` either,
//! such as the swimming outfit), the mana shield's remaining capacity (above), and the item-use
//! exhaust and next-action timers, which Crystal keeps as player fields, not conditions.
//! Restoring re-applies each entry as a fresh application of its own definition with what it had
//! left; one that no longer fits the 0691 bounds is left out, never the whole snapshot.
use super::cast::PlayerSpellState;
use oteryn_gameplay_domain::domain::appearance::AppearanceSelection;
use oteryn_gameplay_domain::domain::condition::attributes::{
    AttributeModifier, AttributeModifiers,
};
use oteryn_gameplay_domain::domain::condition::{
    ConditionGate, ConditionValues, DotElement, DotSequenceStep, MAX_DOT_SEQUENCE_STEPS,
    SpeedRange, StatusKind,
};
use oteryn_gameplay_durability::durability::character_last_position::{
    MAX_SAVED_CONDITIONS, MAX_SAVED_CONDITIONS_BYTES, SavedConditions,
};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionInstance, ConditionSourceKind, DamageSchedule,
    DamageSegment,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
};
use serde::{Deserialize, Serialize};

/// The creature source a restored damage carries: Crystal restores it without its owner.
pub const SAVED_DAMAGE_SOURCE: &str = "creature:saved-condition";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedCondition {
    key: String,
    revision: u32,
    values: SavedValues,
    /// Saved from the runtime actor's own store; its owner re-applies it there.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    runtime: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// The steps a damage sequence has still to deal, the first with its remaining repetitions.
    DamageSequence {
        element: SavedElement,
        steps: Vec<SavedStep>,
        delayed: bool,
    },
    /// An agony ramp from its next amount down to 1.
    AgonyRamp {
        start: u32,
        interval_ms: u32,
        delayed: bool,
    },
    /// The segments a damage schedule has still to deal, the first with its remaining count.
    DamageSchedule {
        element: SavedElement,
        segments: Vec<SavedSegment>,
        delayed: bool,
    },
    /// A mana shield with its cast capacity; it is restored full (module docs).
    ManaShield {
        duration_ms: u32,
        capacity: u32,
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
    SourceAttributes {
        duration_ms: u32,
        melee: Option<SavedModifier>,
        distance: Option<SavedModifier>,
        magic_points: Option<SavedModifier>,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        appearance: Option<SavedAppearance>,
    },
    /// An item appearance, bound to the server artifact it was applied under.
    ItemOutfit {
        duration_ms: u32,
        item_key: String,
        item_revision: String,
        artifact_digest: String,
    },
    /// A spell cooldown (`key` is the spell) with the time it has left.
    SpellCooldown {
        remaining_us: u64,
    },
    /// A group cooldown (`key` is the group) with the time it has left.
    GroupCooldown {
        remaining_us: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedStep {
    amount: u32,
    interval_ms: u32,
    repetitions: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSegment {
    count: u32,
    amount: u32,
    interval_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum SavedModifier {
    PercentOfBase(i32),
    Add(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedAppearance {
    outfit_key: String,
    colours: [u16; 4],
    addons: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mount_key: Option<String>,
}

impl SavedModifier {
    const fn of(modifier: AttributeModifier) -> Self {
        match modifier {
            AttributeModifier::PercentOfBase(n) => Self::PercentOfBase(n),
            AttributeModifier::Add(n) => Self::Add(n),
        }
    }
    const fn modifier(self) -> AttributeModifier {
        match self {
            Self::PercentOfBase(n) => AttributeModifier::PercentOfBase(n),
            Self::Add(n) => AttributeModifier::Add(n),
        }
    }
}

fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 || !text.is_ascii() {
        return None;
    }
    let mut out = [0; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// The length PostgreSQL's `jsonb` text of compact `json` has: it writes one space after every
/// structural `:` and `,`, and the 0691 bound is on that text.
fn stored_len(json: &str) -> usize {
    let (mut extra, mut in_string, mut escaped) = (0, false, false);
    for byte in json.bytes() {
        if in_string {
            match (escaped, byte) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
        } else {
            match byte {
                b'"' => in_string = true,
                b':' | b',' => extra += 1,
                _ => {}
            }
        }
    }
    json.len() + extra
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
/// (module docs).
fn saved_of<S>(
    instance: &ConditionInstance<S>,
    now_us: u64,
    runtime: bool,
) -> Option<SavedCondition> {
    let definition = instance.definition();
    // A finite remaining time; `-1` is an infinite instance (and every damage family, which
    // ends with its total instead).
    let remaining = instance.remaining_duration_ms(now_us);
    let duration_ms = remaining
        .and_then(|ms| u32::try_from(ms).ok())
        .filter(|ms| *ms > 0);
    // A damage family is saved while it has damage left to deal.
    let damage_left = remaining == Some(-1) && instance.remaining_total() > 0;
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
        } => damage_left.then(|| SavedValues::DamageOverTime {
            element: SavedElement::of(element),
            total: instance.remaining_total(),
            per_tick,
            interval_ms,
            delayed,
        }),
        ConditionValues::Agony {
            per_tick,
            interval_ms,
            delayed,
            ..
        } => damage_left.then(|| SavedValues::Agony {
            total: instance.remaining_total(),
            per_tick,
            interval_ms,
            delayed,
        }),
        ConditionValues::DamageSequence {
            element,
            steps,
            len,
            delayed,
        } => {
            let (step, repetition) = instance.sequence_cursor();
            let rest = steps.get(usize::from(step)..usize::from(len))?;
            let steps = rest
                .iter()
                .enumerate()
                .map(|(index, s)| SavedStep {
                    amount: s.amount,
                    interval_ms: s.interval_ms,
                    repetitions: if index == 0 {
                        s.repetitions.saturating_sub(repetition)
                    } else {
                        s.repetitions
                    },
                })
                .filter(|s| s.repetitions > 0)
                .collect::<Vec<_>>();
            (damage_left && !steps.is_empty()).then(|| SavedValues::DamageSequence {
                element: SavedElement::of(element),
                steps,
                delayed,
            })
        }
        ConditionValues::AgonyRamp {
            start,
            interval_ms,
            delayed,
        } => {
            let start = start.saturating_sub(instance.sequence_cursor().1);
            (damage_left && start > 0).then_some(SavedValues::AgonyRamp {
                start,
                interval_ms,
                delayed,
            })
        }
        ConditionValues::DamageSchedule { element } => {
            let delayed = definition.source_damage_schedule()?.delayed();
            let segments = instance
                .remaining_damage_segments()?
                .into_iter()
                .filter(|s| s.count > 0)
                .map(|s| SavedSegment {
                    count: s.count,
                    amount: s.amount,
                    interval_ms: s.interval_ms,
                })
                .collect::<Vec<_>>();
            (damage_left && !segments.is_empty()).then(|| SavedValues::DamageSchedule {
                element: SavedElement::of(element),
                segments,
                delayed,
            })
        }
        ConditionValues::ManaShield { .. } => {
            let (_, capacity) = instance.mana_shield();
            duration_ms.map(|duration_ms| SavedValues::ManaShield {
                duration_ms,
                capacity,
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
        ConditionValues::SourceAttributes { modifiers, .. } => {
            duration_ms.map(|duration_ms| SavedValues::SourceAttributes {
                duration_ms,
                melee: modifiers.melee.map(SavedModifier::of),
                distance: modifiers.distance.map(SavedModifier::of),
                magic_points: modifiers.magic_points.map(SavedModifier::of),
            })
        }
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
            let appearance =
                definition
                    .source_appearance_selection()
                    .map(|selection| SavedAppearance {
                        outfit_key: selection.outfit_key.clone(),
                        colours: selection.colours,
                        addons: selection.addons,
                        mount_key: selection.mount_key.clone(),
                    });
            duration_ms.map(|duration_ms| SavedValues::Outfit {
                duration_ms,
                look_type,
                appearance,
            })
        }
        ConditionValues::ItemOutfit { .. } => {
            let item = definition.source_item_appearance()?;
            duration_ms.map(|duration_ms| SavedValues::ItemOutfit {
                duration_ms,
                item_key: item.definition_key().to_owned(),
                item_revision: item.revision_ref().to_owned(),
                artifact_digest: hex(item.artifact_digest()),
            })
        }
        ConditionValues::FoodRegeneration { .. } => None,
    }?;
    Some(SavedCondition {
        key: definition.key().to_owned(),
        revision: definition.revision(),
        values,
        runtime,
    })
}

/// The definition one saved entry re-applies, or `None` for one no definition admits (and for a
/// cooldown, which is no condition here). An item outfit of another server artifact than
/// `artifact_digest` is not admitted.
fn definition_of(saved: &SavedCondition, artifact_digest: [u8; 32]) -> Option<ConditionDefinition> {
    let values = match &saved.values {
        SavedValues::TimedStatus { kind, duration_ms } => ConditionValues::TimedStatus {
            kind: kind.kind(),
            duration_ms: *duration_ms,
        },
        SavedValues::Gate { gate, duration_ms } => ConditionValues::Gate {
            gate: gate.gate(),
            duration_ms: *duration_ms,
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
                paralysis: *paralysis,
                range: SpeedRange {
                    a_min: 1000,
                    b_min: b,
                    a_max: 1000,
                    b_max: b,
                },
                duration_ms: *duration_ms,
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
            total_min: *total,
            total_max: *total,
            per_tick: *per_tick,
            interval_ms: *interval_ms,
            delayed: *delayed,
        },
        SavedValues::Agony {
            total,
            per_tick,
            interval_ms,
            delayed,
        } => ConditionValues::Agony {
            total_min: *total,
            total_max: *total,
            per_tick: *per_tick,
            interval_ms: *interval_ms,
            delayed: *delayed,
        },
        SavedValues::DamageSequence {
            element,
            steps,
            delayed,
        } => {
            if steps.len() > MAX_DOT_SEQUENCE_STEPS {
                return None;
            }
            let mut array = [DotSequenceStep::default(); MAX_DOT_SEQUENCE_STEPS];
            for (slot, step) in array.iter_mut().zip(steps) {
                *slot = DotSequenceStep {
                    amount: step.amount,
                    interval_ms: step.interval_ms,
                    repetitions: step.repetitions,
                };
            }
            ConditionValues::DamageSequence {
                element: element.element(),
                steps: array,
                len: u8::try_from(steps.len()).ok()?,
                delayed: *delayed,
            }
        }
        SavedValues::AgonyRamp {
            start,
            interval_ms,
            delayed,
        } => ConditionValues::AgonyRamp {
            start: *start,
            interval_ms: *interval_ms,
            delayed: *delayed,
        },
        SavedValues::DamageSchedule {
            element,
            segments,
            delayed,
        } => {
            let segments = segments
                .iter()
                .map(|s| DamageSegment {
                    count: s.count,
                    amount: s.amount,
                    interval_ms: s.interval_ms,
                })
                .collect();
            return ConditionDefinition::new_damage_schedule(
                &saved.key,
                saved.revision,
                element.element(),
                DamageSchedule::Fixed {
                    delayed: *delayed,
                    segments,
                },
            );
        }
        SavedValues::ManaShield { duration_ms, .. } => ConditionValues::ManaShield {
            duration_ms: *duration_ms,
        },
        SavedValues::Recovery {
            duration_ms,
            interval_ms,
        } => ConditionValues::Recovery {
            duration_ms: *duration_ms,
            interval_ms: *interval_ms,
        },
        SavedValues::Light { level, duration_ms } => ConditionValues::Light {
            level: *level,
            duration_ms: *duration_ms,
        },
        SavedValues::SpellLight {
            level,
            color,
            duration_ms,
        } => ConditionValues::SpellLight {
            level: *level,
            color: *color,
            duration_ms: *duration_ms,
        },
        SavedValues::SpellRegeneration {
            duration_ms,
            sub_id,
            health_gain,
            health_interval_ms,
            mana_gain,
            mana_interval_ms,
        } => ConditionValues::SpellRegeneration {
            duration_ms: *duration_ms,
            sub_id: *sub_id,
            health_gain: *health_gain,
            health_interval_ms: *health_interval_ms,
            mana_gain: *mana_gain,
            mana_interval_ms: *mana_interval_ms,
        },
        SavedValues::Attributes {
            duration_ms,
            critical_chance_percent,
            critical_extra_percentage_points,
            damage_dealt_percent,
            incoming_reduction_percent,
        } => ConditionValues::Attributes {
            duration_ms: *duration_ms,
            critical_chance_percent: *critical_chance_percent,
            critical_extra_percentage_points: *critical_extra_percentage_points,
            damage_dealt_percent: *damage_dealt_percent,
            incoming_reduction_percent: *incoming_reduction_percent,
        },
        SavedValues::SourceAttributes {
            duration_ms,
            melee,
            distance,
            magic_points,
        } => ConditionValues::SourceAttributes {
            modifiers: AttributeModifiers {
                melee: melee.map(SavedModifier::modifier),
                distance: distance.map(SavedModifier::modifier),
                magic_points: magic_points.map(SavedModifier::modifier),
            },
            duration_ms: *duration_ms,
        },
        SavedValues::Invisible { duration_ms } => ConditionValues::Invisible {
            duration_ms: *duration_ms,
        },
        SavedValues::SpellSkills {
            duration_ms,
            sub_id,
            magic_level,
            fist,
            melee,
            distance,
            shielding,
        } => ConditionValues::SpellSkills {
            duration_ms: *duration_ms,
            sub_id: *sub_id,
            magic_level: *magic_level,
            fist: *fist,
            melee: *melee,
            distance: *distance,
            shielding: *shielding,
        },
        SavedValues::Outfit {
            duration_ms,
            look_type,
            appearance,
        } => {
            let definition = ConditionDefinition::new(
                &saved.key,
                saved.revision,
                ConditionValues::Outfit {
                    duration_ms: *duration_ms,
                    look_type: *look_type,
                },
            )?;
            return match appearance {
                None => Some(definition),
                Some(appearance) => definition.with_appearance(AppearanceSelection {
                    outfit_key: appearance.outfit_key.clone(),
                    colours: appearance.colours,
                    addons: appearance.addons,
                    mount_key: appearance.mount_key.clone(),
                }),
            };
        }
        SavedValues::ItemOutfit {
            duration_ms,
            item_key,
            item_revision,
            artifact_digest: digest,
        } => {
            let digest = unhex(digest).filter(|digest| *digest == artifact_digest)?;
            return ConditionDefinition::new(
                &saved.key,
                saved.revision,
                ConditionValues::ItemOutfit {
                    duration_ms: *duration_ms,
                },
            )?
            .with_item_appearance(item_key, item_revision, digest);
        }
        SavedValues::SpellCooldown { .. } | SavedValues::GroupCooldown { .. } => return None,
    };
    ConditionDefinition::new(&saved.key, saved.revision, values)
}

/// What a restore re-applied to the spell owner's state, and what it hands the runtime actor's
/// own condition owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoredConditions {
    /// Conditions and cooldowns re-applied to this state.
    pub applied: usize,
    /// Definitions saved from the runtime actor's store, for its owner to re-apply there as a
    /// self application. A damage family is never among them: player damage ticks in this
    /// state's store, so it is restored here.
    pub runtime: Vec<ConditionDefinition>,
    /// The base speed the runtime definitions apply with.
    pub base_speed: u16,
}

impl PlayerSpellState {
    /// The persistent conditions and cooldowns to store with the final logout write at `now_us`
    /// (semantic microseconds): this state's conditions, then `runtime` (the runtime actor's own
    /// condition store), then the cooldowns. `None` when there are none. An entry that would take
    /// the snapshot past the 0691 bounds is left out; the others are kept.
    #[must_use]
    pub fn saved_conditions_snapshot<A>(
        &self,
        runtime: &[ConditionInstance<A>],
        now_us: u64,
    ) -> Option<SavedConditions> {
        let cooldown = |key: &String, deadline: &SemanticTimeMicros, group: bool| {
            let remaining_us = deadline.get().checked_sub(now_us).filter(|us| *us > 0)?;
            Some(SavedCondition {
                key: key.clone(),
                revision: 0,
                values: if group {
                    SavedValues::GroupCooldown { remaining_us }
                } else {
                    SavedValues::SpellCooldown { remaining_us }
                },
                runtime: false,
            })
        };
        let entries = self
            .conditions
            .instances()
            .iter()
            .filter_map(|instance| saved_of(instance, now_us, false))
            .chain(
                runtime
                    .iter()
                    .filter_map(|instance| saved_of(instance, now_us, true)),
            )
            .chain(
                self.cooldowns
                    .spells
                    .iter()
                    .filter_map(|(key, deadline)| cooldown(key, deadline, false)),
            )
            .chain(
                self.cooldowns
                    .groups
                    .iter()
                    .filter_map(|(key, deadline)| cooldown(key, deadline, true)),
            );
        // `[` and `]`, then each entry with its separator.
        let mut text = String::from("[");
        let mut count = 0;
        for entry in entries {
            if count == MAX_SAVED_CONDITIONS {
                break;
            }
            let Ok(json) = serde_json::to_string(&entry) else {
                continue;
            };
            let separator = if count == 0 { "" } else { "," };
            if stored_len(&text) + stored_len(separator) + stored_len(&json) + 1
                > MAX_SAVED_CONDITIONS_BYTES
            {
                continue;
            }
            text.push_str(separator);
            text.push_str(&json);
            count += 1;
        }
        text.push(']');
        if count == 0 {
            return None;
        }
        SavedConditions::new(text)
    }

    /// Re-apply `saved` on a freshly created actor at `now_us`, each as a fresh application of
    /// its definition with what it had left: a damage family with creature provenance
    /// ([`SAVED_DAMAGE_SOURCE`]), anything else as a self application; a cooldown is armed again
    /// for its remaining time. `artifact_digest` is the active server artifact (an item outfit of
    /// another is dropped). An entry that no definition admits any more, or that the condition
    /// policy refuses, is dropped; the rest still apply. `None` for undecodable text.
    pub fn restore_saved_conditions(
        &mut self,
        saved: &SavedConditions,
        now_us: u64,
        artifact_digest: [u8; 32],
    ) -> Option<RestoredConditions> {
        let entries = serde_json::from_str::<Vec<SavedCondition>>(saved.as_str()).ok()?;
        // Restoration replays no random decision: a speed range is a single point here and no
        // other restored definition draws. This inert context grants no authority.
        let root = GameplayDecisionRoot::from_bytes([0; 32]);
        let base_speed = u16::try_from(self.base_speed).ok()?;
        let mut restored = RestoredConditions {
            applied: 0,
            runtime: Vec::new(),
            base_speed,
        };
        for entry in &entries {
            let cooldowns = match entry.values {
                SavedValues::SpellCooldown { remaining_us } => {
                    Some((&mut self.cooldowns.spells, remaining_us))
                }
                SavedValues::GroupCooldown { remaining_us } => {
                    Some((&mut self.cooldowns.groups, remaining_us))
                }
                _ => None,
            };
            if let Some((slots, remaining_us)) = cooldowns {
                if let Some(deadline) = now_us.checked_add(remaining_us) {
                    slots.insert(entry.key.clone(), SemanticTimeMicros::from_micros(deadline));
                    restored.applied += 1;
                }
                continue;
            }
            let Some(definition) = definition_of(entry, artifact_digest) else {
                continue;
            };
            let damage = matches!(
                entry.values,
                SavedValues::DamageOverTime { .. }
                    | SavedValues::Agony { .. }
                    | SavedValues::DamageSequence { .. }
                    | SavedValues::AgonyRamp { .. }
                    | SavedValues::DamageSchedule { .. }
            );
            if entry.runtime && !damage {
                restored.runtime.push(definition);
                continue;
            }
            let capacity = match entry.values {
                SavedValues::ManaShield { capacity, .. } => capacity,
                _ => 0,
            };
            let facts = ApplicationFacts {
                now: now_us,
                base_speed,
                mana_shield_capacity: capacity,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
            };
            let (source, kind) = if damage {
                (
                    Some(SAVED_DAMAGE_SOURCE.to_owned()),
                    ConditionSourceKind::Creature,
                )
            } else {
                (None, ConditionSourceKind::SelfUse)
            };
            if self
                .conditions
                .apply(&definition, source, kind, &[], &facts)
                .is_ok()
            {
                restored.applied += 1;
            }
        }
        Some(restored)
    }
}

#[cfg(test)]
#[path = "saved_conditions_tests.rs"]
mod tests;
