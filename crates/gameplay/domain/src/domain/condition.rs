//! Shared immutable condition vocabulary and content values.

pub mod attributes;
use attributes::AttributeModifiers;

/// The damage-over-time elements, one conflict key each (§3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DotElement {
    Poison,
    Fire,
    Energy,
    Bleeding,
    Drown,
    Freezing,
    Dazzled,
    Cursed,
}

/// §3.5: the gate families `muted`, `pacified` and `powerless`, one conflict key each. The
/// declaration order is the refusal order when several gates refuse the same action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConditionGate {
    Muted,
    Pacified,
    Powerless,
}

impl ConditionGate {
    /// The content condition type that names the family.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Muted => "muted",
            Self::Pacified => "pacified",
            Self::Powerless => "powerless",
        }
    }
}

/// At most one instance exists per conflict key. Source spell subids extend the
/// candidate key vocabulary; the per-actor instance cap is `COND0-RL-01`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConflictKey {
    Status(StatusKind),
    Speed,
    Element(DotElement),
    FoodRegeneration,
    Recovery,
    ManaShield,
    Light,
    Attributes,
    SpellSkills(u8),
    SpellRegeneration(u8),
    Invisible,
    Outfit,
    Gate(ConditionGate),
    /// §3.5 `agony`.
    Agony,
}

/// The closed condition types. A creature's content immunities name these; haste and paralysis
/// share the `speed` key but not an immunity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConditionType {
    Status(StatusKind),
    Haste,
    Paralysis,
    DamageOverTime(DotElement),
    FoodRegeneration,
    Recovery,
    ManaShield,
    Light,
    Attributes,
    SpellSkills(u8),
    SpellRegeneration(u8),
    Invisible,
    Outfit,
    Gate(ConditionGate),
    Agony,
}

impl ConditionType {
    pub const fn conflict_key(self) -> ConflictKey {
        match self {
            Self::Status(kind) => ConflictKey::Status(kind),
            Self::Haste | Self::Paralysis => ConflictKey::Speed,
            Self::DamageOverTime(element) => ConflictKey::Element(element),
            Self::FoodRegeneration => ConflictKey::FoodRegeneration,
            Self::Recovery => ConflictKey::Recovery,
            Self::ManaShield => ConflictKey::ManaShield,
            Self::Light => ConflictKey::Light,
            Self::Attributes => ConflictKey::Attributes,
            Self::SpellSkills(id) => ConflictKey::SpellSkills(id),
            Self::SpellRegeneration(id) => ConflictKey::SpellRegeneration(id),
            Self::Invisible => ConflictKey::Invisible,
            Self::Outfit => ConflictKey::Outfit,
            Self::Gate(gate) => ConflictKey::Gate(gate),
            Self::Agony => ConflictKey::Agony,
        }
    }

    /// §5: the `negative` dispel tag: damage over time and paralysis. §3.5: the gate families
    /// and `agony` are not tagged, so Cleanse and the `negative` selector never select them.
    pub const fn is_negative(self) -> bool {
        matches!(
            self,
            Self::Paralysis
                | Self::DamageOverTime(_)
                | Self::Status(StatusKind::Drunk | StatusKind::Rooted | StatusKind::Feared)
        )
    }
}

/// A speed range in thousandths: the target speed is drawn in
/// `[a_min × (base − 40) + b_min, a_max × (base − 40) + b_max]` (§3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeedRange {
    pub a_min: i32,
    pub b_min: i32,
    pub a_max: i32,
    pub b_max: i32,
}
/// Ordered field ramp groups; source XML is compiled into at most this many
/// distinct consecutive amounts, with repetitions preserving every source tick.
/// Bounded source-spell candidate cap: Curse needs 17 captured sequence groups.
/// The accepted per-pass tick cap remains `COND0-RL-03`.
pub const MAX_DOT_SEQUENCE_STEPS: usize = 32;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DotSequenceStep {
    pub amount: u32,
    pub interval_ms: u32,
    pub repetitions: u32,
}

/// Exact source/content ratio, without quantization to thousandths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactSpeedRatio {
    pub numerator: i64,
    pub denominator: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RationalSpeedRange {
    pub a_min: ExactSpeedRatio,
    pub b_min: i32,
    pub a_max: ExactSpeedRatio,
    pub b_max: i32,
}
impl RationalSpeedRange {
    pub fn valid(self) -> bool {
        self.a_min.denominator != 0 && self.a_max.denominator != 0
    }
    pub fn endpoints(self, base: u16) -> Result<(i64, i64), ConditionRefusal> {
        let end = |a: ExactSpeedRatio, b: i32| {
            if a.denominator == 0 {
                return Err(ConditionRefusal::DrawFailed);
            }
            let d = i128::from(a.denominator);
            let n = i128::from(a.numerator) * (i128::from(base) - 40) + i128::from(b) * d;
            // One truncation of the complete expression, as in the accepted speed rule.
            i64::try_from(n / d).map_err(|_| ConditionRefusal::DrawFailed)
        };
        let (x, y) = (end(self.a_min, self.b_min)?, end(self.a_max, self.b_max)?);
        Ok((x.min(y), x.max(y)))
    }
}

/// The content values of one definition (§3: values come from content, never from code).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::large_enum_variant,
    reason = "transient owner result; boxing would add an allocation to the owner turn"
)]
pub enum ConditionValues {
    DamageSchedule {
        element: DotElement,
    },
    TimedStatus {
        kind: StatusKind,
        duration_ms: u32,
    },
    SourceAttributes {
        modifiers: AttributeModifiers,
        duration_ms: u32,
    },
    RationalSpeed {
        paralysis: bool,
        range: RationalSpeedRange,
        duration_ms: u32,
    },
    Speed {
        paralysis: bool,
        range: SpeedRange,
        duration_ms: u32,
    },
    DamageOverTime {
        element: DotElement,
        total_min: u32,
        total_max: u32,
        per_tick: u32,
        interval_ms: u32,
        /// `false`: the first tick is dealt at once on application.
        delayed: bool,
    },
    DamageSequence {
        element: DotElement,
        steps: [DotSequenceStep; MAX_DOT_SEQUENCE_STEPS],
        len: u8,
        delayed: bool,
    },
    FoodRegeneration {
        added_ms: u32,
        interval_ms: u32,
    },
    Recovery {
        duration_ms: u32,
        interval_ms: u32,
    },
    ManaShield {
        duration_ms: u32,
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
    ItemOutfit {
        duration_ms: u32,
    },
    /// §3.5: a gate family; a new application refreshes the duration.
    Gate {
        gate: ConditionGate,
        duration_ms: u32,
    },
    /// §3.5: damage over time of an undefined element, shaped like `DamageOverTime`.
    Agony {
        total_min: u32,
        total_max: u32,
        per_tick: u32,
        interval_ms: u32,
        /// `false`: the first tick is dealt at once on application.
        delayed: bool,
    },
}

impl ConditionValues {
    /// §3.5: the gate family named by its content condition type; any other name is `None`.
    pub fn gate(key: &str, duration_ms: u32) -> Option<Self> {
        let gate = match key {
            "muted" => ConditionGate::Muted,
            "pacified" => ConditionGate::Pacified,
            "powerless" => ConditionGate::Powerless,
            _ => return None,
        };
        Some(Self::Gate { gate, duration_ms })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionRefusal {
    ZeroDamage,
    /// The target's content lists an immunity to this type (§6.1).
    Immune,
    /// §6.1 PvE re-entry protection.
    ReentryProtected,
    /// The policy kept the current instance (§3.1 not strictly greater; light shorter).
    KeptCurrent,
    /// ITEM-USE-0 §6.1: the food time would reach its cap; nothing is burned.
    Full,
    /// `COND0-RL-01`.
    InstanceLimit,
    DrawFailed,
    ArithmeticBounds,
    /// A prepared Cleanse deadline cannot be represented; no instance is removed.
    TimeOverflow,
    /// No instance identity is reused after exhausting the actor's sequence space.
    SequenceExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatusKind {
    Invisible,
    Drunk,
    Rooted,
    Feared,
}
