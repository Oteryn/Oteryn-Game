//! The ATTACK-0 §4 attack-target owner state of one runtime actor: the target and its lineage,
//! the auto-attack clock and the in-fight (logout block) deadline.
//!
//! Runtime-only: nothing here is durable. The target identity `T` is the D85 identity and the
//! lineage `L` is the CommandRef that set the target; both are supplied by the wiring.

use oteryn_simulation_determinism::SemanticTimeMicros;

/// Facts about the attacker and its current target at one instant, read by the wiring from the
/// Channel runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TargetFacts {
    /// The target still exists in the runtime scope and is visible to the attacker's session.
    pub(crate) present_and_visible: bool,
    pub(crate) is_creature: bool,
    pub(crate) alive: bool,
    pub(crate) same_floor: bool,
    /// Chebyshev tile distance on the floor.
    pub(crate) distance: u32,
    pub(crate) attacker_in_protection_zone: bool,
    pub(crate) target_in_protection_zone: bool,
    /// The attacker is under the 4 s re-entry PvE protection (§3).
    pub(crate) attacker_reentry_protected: bool,
}

/// Why a target is refused when it is set (§3); the target is not stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetRefusal {
    NotVisible,
    NotACreature,
    Dead,
    TargetInProtectionZone,
    AttackerInProtectionZone,
    ReentryProtected,
}

/// Why a valid-to-hold target is not swung at now (§4: "out of range, the swing waits").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwingWait {
    OtherFloor,
    OutOfRange,
    AttackerInProtectionZone,
    TargetInProtectionZone,
    ReentryProtected,
}

/// Why a held target is dropped (§4: "a dead or vanished target clears the target").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetCleared {
    Vanished,
    NotACreature,
    Dead,
}

/// The §4 validity of a held target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetValidity {
    Valid,
    Wait(SwingWait),
    Clear(TargetCleared),
}

/// Melee reach: adjacent (§4).
pub(crate) const MELEE_RANGE: u32 = 1;

impl TargetFacts {
    /// Admission of a new target (§3). Range is not checked: an out-of-range target is held and
    /// its swing waits.
    pub(crate) fn admit(&self) -> Result<(), TargetRefusal> {
        if !self.present_and_visible {
            Err(TargetRefusal::NotVisible)
        } else if !self.is_creature {
            Err(TargetRefusal::NotACreature)
        } else if !self.alive {
            Err(TargetRefusal::Dead)
        } else if self.attacker_reentry_protected {
            Err(TargetRefusal::ReentryProtected)
        } else if self.attacker_in_protection_zone {
            Err(TargetRefusal::AttackerInProtectionZone)
        } else if self.target_in_protection_zone {
            Err(TargetRefusal::TargetInProtectionZone)
        } else {
            Ok(())
        }
    }

    /// The validity of a held target for one swing (§4).
    pub(crate) fn validity(&self) -> TargetValidity {
        if !self.present_and_visible {
            TargetValidity::Clear(TargetCleared::Vanished)
        } else if !self.is_creature {
            TargetValidity::Clear(TargetCleared::NotACreature)
        } else if !self.alive {
            TargetValidity::Clear(TargetCleared::Dead)
        } else if self.attacker_reentry_protected {
            TargetValidity::Wait(SwingWait::ReentryProtected)
        } else if self.attacker_in_protection_zone {
            TargetValidity::Wait(SwingWait::AttackerInProtectionZone)
        } else if self.target_in_protection_zone {
            TargetValidity::Wait(SwingWait::TargetInProtectionZone)
        } else if !self.same_floor {
            TargetValidity::Wait(SwingWait::OtherFloor)
        } else if self.distance > MELEE_RANGE {
            TargetValidity::Wait(SwingWait::OutOfRange)
        } else {
            TargetValidity::Valid
        }
    }
}

/// One due swing: one `AutoAttack` ability invocation through GAME-ABILITY-01 (§4). Its
/// occurrence key is (runtime scope, attacker, `sequence`); the wiring adds the first two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Swing<T, L> {
    pub(crate) target: T,
    pub(crate) lineage: L,
    pub(crate) sequence: u64,
    pub(crate) at: SemanticTimeMicros,
}

/// The outcome of polling the auto-attack timer once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwingPoll<T, L> {
    NoTarget,
    /// The held target was dropped; no swing.
    Cleared(TargetCleared),
    Waiting(SwingWait),
    NotDue {
        due: SemanticTimeMicros,
    },
    Swing(Swing<T, L>),
}

/// The closed RNG purposes of one swing (§4); `HitChance` is reserved for distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwingRngPurpose {
    HitChance,
    DamageDraw,
    DefenceDraw,
    ArmorDraw,
}

/// The attack-target owner state of one runtime actor (player or creature).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttackState<T, L> {
    target: Option<(T, L)>,
    /// Execution time of the last swing; the next is due one interval after it.
    last_swing: Option<SemanticTimeMicros>,
    next_sequence: u64,
    /// End of the in-fight deadline (`ATTACK0-RL-03`).
    in_fight_until: Option<SemanticTimeMicros>,
}

impl<T, L> Default for AttackState<T, L> {
    fn default() -> Self {
        Self {
            target: None,
            last_swing: None,
            next_sequence: 0,
            in_fight_until: None,
        }
    }
}

impl<T: Copy + Eq, L: Copy> AttackState<T, L> {
    pub(crate) fn target(&self) -> Option<T> {
        self.target.map(|(target, _)| target)
    }

    /// Set a new target after [`TargetFacts::admit`]; the attack clock is kept, so changing
    /// targets never shortens the interval.
    pub(crate) fn set_target(
        &mut self,
        target: T,
        lineage: L,
        facts: &TargetFacts,
    ) -> Result<(), TargetRefusal> {
        facts.admit()?;
        self.target = Some((target, lineage));
        Ok(())
    }

    /// Stop attacking (a `none` intent, reconnect or channel transfer). The in-fight deadline is
    /// untouched (§3 Reconnect).
    pub(crate) fn clear_target(&mut self) {
        self.target = None;
    }

    /// Poll the auto-attack timer at `now` with the current facts of the held target.
    ///
    /// `DEADLINE_STATE` catch-up: at most one swing per call, never a backlog, and the next
    /// deadline counts from `now` (the execution time), so a stall never shortens the next
    /// interval.
    pub(crate) fn poll_swing(
        &mut self,
        now: SemanticTimeMicros,
        interval_micros: u64,
        facts: &TargetFacts,
    ) -> SwingPoll<T, L> {
        let Some((target, lineage)) = self.target else {
            return SwingPoll::NoTarget;
        };
        match facts.validity() {
            TargetValidity::Clear(reason) => {
                self.target = None;
                return SwingPoll::Cleared(reason);
            }
            TargetValidity::Wait(reason) => return SwingPoll::Waiting(reason),
            TargetValidity::Valid => {}
        }
        if let Some(due) = self.next_due(interval_micros)
            && now < due
        {
            return SwingPoll::NotDue { due };
        }
        let sequence = self.next_sequence;
        self.next_sequence = sequence.saturating_add(1);
        self.last_swing = Some(now);
        SwingPoll::Swing(Swing {
            target,
            lineage,
            sequence,
            at: now,
        })
    }

    /// The deadline of the next swing; `None` when one may swing at once.
    pub(crate) fn next_due(&self, interval_micros: u64) -> Option<SemanticTimeMicros> {
        self.last_swing
            .map(|last| SemanticTimeMicros::from_micros(last.get().saturating_add(interval_micros)))
    }

    /// Whether the actor swung less than one interval before `now` (the defence mode factor of
    /// `player.cpp:853-872` reads it).
    pub(crate) fn swung_within_interval(
        &self,
        now: SemanticTimeMicros,
        interval_micros: u64,
    ) -> bool {
        self.next_due(interval_micros).is_some_and(|due| now < due)
    }

    /// Record a hit dealt or taken: the in-fight deadline runs `in_fight_micros` from `now` and
    /// never moves earlier.
    pub(crate) fn record_hit(&mut self, now: SemanticTimeMicros, in_fight_micros: u64) {
        let until = SemanticTimeMicros::from_micros(now.get().saturating_add(in_fight_micros));
        self.in_fight_until = Some(match self.in_fight_until {
            Some(current) if current > until => current,
            _ => until,
        });
    }

    /// Whether the in-fight deadline still runs at `now`: a logout is refused and a closed
    /// client leaves the actor in the world (§4).
    pub(crate) fn in_fight(&self, now: SemanticTimeMicros) -> bool {
        self.in_fight_until.is_some_and(|until| now < until)
    }

    pub(crate) fn in_fight_until(&self) -> Option<SemanticTimeMicros> {
        self.in_fight_until
    }
}
