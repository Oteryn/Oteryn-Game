//! ARENA1 A4: the nine accepted Drome modifiers, independent of monster artwork.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DromeModifier {
    Somersault,
    GoingDownWithMe,
    ExplodingCorpses,
    ThatEscalatedQuickly,
    TheFloorIsLava,
    BeamMeUp,
    TankedUp,
    SownSorrow,
    BadRoots,
}
mod death;
mod draw;
mod familiar;
mod roles;
mod runtime;
use draw::bounded_draw;
pub use draw::drome_rotation_modifiers;
pub use roles::DromeCreatureRole;
pub use runtime::DromeActorModifiers;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DromeParticipant {
    pub actor: ExactActorRef,
    pub position: MovementLocalPosition,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DromeModifierEffect {
    CurrentHealthDamage {
        target: ExactActorRef,
        percent: u32,
    },
    Teleport {
        actor: ExactActorRef,
        destination: MovementLocalPosition,
    },
    Status {
        target: ExactActorRef,
        kind: oteryn_gameplay_domain::domain::condition::StatusKind,
        duration_ms: u32,
        incoming_percent: u32,
    },
    Mark {
        kind: DromeModifier,
        positions: Vec<MovementLocalPosition>,
        resolves_at: u64,
    },
    Seed {
        kind: DromeModifier,
        position: MovementLocalPosition,
        resolves_at: u64,
    },
}
#[derive(Debug, Clone)]
pub struct DromeModifierState {
    selected: [DromeModifier; 2],
    run: [u8; 16],
    group_size: usize,
    room: Vec<MovementLocalPosition>,
    started: u64,
    wave: u32,
    next_marks: u64,
    next_seed: u64,
    ordinal: u64,
    pending: Vec<Pulse>,
}
impl DromeModifierState {
    pub fn new(
        selected: [DromeModifier; 2],
        run: [u8; 16],
        group_size: usize,
        mut room: Vec<MovementLocalPosition>,
        started: u64,
    ) -> Result<Self, CarrierError> {
        room.sort_by_key(|at| (at.floor, at.y, at.x));
        room.dedup();
        if selected[0] == selected[1] || !(1..=5).contains(&group_size) || room.len() < 100 {
            return Err(CarrierError::InvalidDamage);
        }
        Ok(Self {
            selected,
            run,
            group_size,
            room,
            started,
            wave: 0,
            next_marks: started.saturating_add(15_000_000),
            next_seed: started.saturating_add(seed_period(group_size)),
            ordinal: 0,
            pending: Vec::new(),
        })
    }
    pub fn begin_wave(
        &mut self,
        wave: u32,
        now: u64,
        admission_size: usize,
    ) -> Result<(), CarrierError> {
        if !(1..=5).contains(&admission_size) {
            return Err(CarrierError::InvalidDamage);
        }
        self.wave = wave;
        self.started = now;
        self.group_size = admission_size;
        self.next_marks = now.saturating_add(15_000_000);
        self.next_seed = now.saturating_add(seed_period(admission_size));
        self.pending.clear();
        self.ordinal = 0;
        Ok(())
    }
    pub fn melee_draw(&self, ordinal: u64) -> Option<u8> {
        self.draw(ordinal, 100, 0)
            .and_then(|draw| u8::try_from(draw).ok())
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn select_test_modifiers(
        &mut self,
        selected: [DromeModifier; 2],
    ) -> Result<(), CarrierError> {
        if selected[0] == selected[1] {
            return Err(CarrierError::InvalidDamage);
        }
        self.selected = selected;
        self.begin_wave(self.wave, self.started, self.group_size)
    }
    pub fn selected(&self) -> [DromeModifier; 2] {
        self.selected
    }
    pub fn wave_bonus(&self, health: i64, maximum: i64) -> u32 {
        u32::from(
            self.selected.contains(&DromeModifier::ThatEscalatedQuickly)
                && health > 0
                && i128::from(health) * 4 < i128::from(maximum),
        ) * 5
    }
    pub fn melee(
        &self,
        source: ExactActorRef,
        at: MovementLocalPosition,
        draw_percent: u8,
        participants: &[DromeParticipant],
        free: impl Fn(MovementLocalPosition) -> bool,
    ) -> Vec<DromeModifierEffect> {
        if !self.selected.contains(&DromeModifier::Somersault) || draw_percent >= 15 {
            return Vec::new();
        }
        let Some(target) = participants
            .iter()
            .filter(|player| player.position.floor == at.floor)
            .max_by_key(|player| {
                (
                    at.x.abs_diff(player.position.x)
                        .max(at.y.abs_diff(player.position.y)),
                    std::cmp::Reverse(player.actor.placement_identity()),
                )
            })
        else {
            return Vec::new();
        };
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let Some(x) = target.position.x.checked_add(dx) else {
                    continue;
                };
                let Some(y) = target.position.y.checked_add(dy) else {
                    continue;
                };
                let destination = MovementLocalPosition {
                    x,
                    y,
                    floor: target.position.floor,
                };
                if self.room.contains(&destination) && free(destination) {
                    return vec![DromeModifierEffect::Teleport {
                        actor: source,
                        destination,
                    }];
                }
            }
        }
        Vec::new()
    }
    pub fn tick(
        &mut self,
        wave: u32,
        now: u64,
        participants: &[DromeParticipant],
        free: impl Fn(MovementLocalPosition) -> bool,
    ) -> Vec<DromeModifierEffect> {
        if now < self.started {
            return Vec::new();
        }
        if wave != self.wave {
            self.wave = wave;
            self.started = now;
            self.next_marks = now.saturating_add(15_000_000);
            self.next_seed = now.saturating_add(seed_period(self.group_size));
            self.pending.clear();
            self.ordinal = 0;
        }
        let mut effects = Vec::new();
        let mut pending = Vec::new();
        for pulse in std::mem::take(&mut self.pending) {
            let seed = matches!(
                pulse.kind,
                DromeModifier::SownSorrow | DromeModifier::BadRoots
            );
            if seed
                && now < pulse.deadline
                && participants
                    .iter()
                    .any(|player| pulse.positions.contains(&player.position))
            {
                continue;
            }
            if now < pulse.deadline {
                pending.push(pulse);
                continue;
            }
            let mut reserved_destinations = Vec::new();
            for player in participants {
                let affected = seed || pulse.positions.contains(&player.position);
                if !affected {
                    continue;
                }
                let effect = match pulse.kind {
                    DromeModifier::TheFloorIsLava => DromeModifierEffect::CurrentHealthDamage {
                        target: player.actor,
                        percent: 60,
                    },
                    DromeModifier::BeamMeUp => {
                        let choices: Vec<_> =
                            self.room.iter().copied().filter(|at| free(*at)).collect();
                        let Some(index) =
                            self.draw(pulse.ordinal, choices.len(), 100 + effects.len() as u64)
                        else {
                            continue;
                        };
                        reserved_destinations.push(choices[index]);
                        DromeModifierEffect::Teleport {
                            actor: player.actor,
                            destination: choices[index],
                        }
                    }
                    DromeModifier::TankedUp => DromeModifierEffect::Status {
                        target: player.actor,
                        kind: StatusKind::Drunk,
                        duration_ms: 10_000,
                        incoming_percent: 110,
                    },
                    DromeModifier::SownSorrow => DromeModifierEffect::Status {
                        target: player.actor,
                        kind: StatusKind::Feared,
                        duration_ms: 2_000,
                        incoming_percent: 115,
                    },
                    DromeModifier::BadRoots => DromeModifierEffect::Status {
                        target: player.actor,
                        kind: StatusKind::Rooted,
                        duration_ms: 3_000,
                        incoming_percent: 125,
                    },
                    _ => continue,
                };
                effects.push(effect);
            }
        }
        self.pending = pending;
        let marks_due = now >= self.next_marks;
        let seed_due = now >= self.next_seed;
        if marks_due {
            self.next_marks = now.saturating_add(15_000_000);
        }
        if seed_due {
            self.next_seed = now.saturating_add(seed_period(self.group_size));
        }
        for kind in self.selected {
            let seed = matches!(kind, DromeModifier::SownSorrow | DromeModifier::BadRoots);
            if !(if seed {
                seed_due
            } else {
                marks_due
                    && matches!(
                        kind,
                        DromeModifier::TheFloorIsLava
                            | DromeModifier::BeamMeUp
                            | DromeModifier::TankedUp
                    )
            }) {
                continue;
            }
            let ordinal = self.ordinal;
            self.ordinal = self.ordinal.saturating_add(1);
            let mut choices = self.room.clone();
            let count = if seed { 1 } else { 100 };
            let mut positions = Vec::with_capacity(count);
            for draw in 0..count {
                let Some(index) = self.draw(ordinal, choices.len(), draw as u64) else {
                    break;
                };
                positions.push(choices.swap_remove(index));
            }
            let deadline = now.saturating_add(if seed { 6_000_000 } else { 3_000_000 });
            if seed {
                effects.push(DromeModifierEffect::Seed {
                    kind,
                    position: positions[0],
                    resolves_at: deadline,
                });
            } else {
                effects.push(DromeModifierEffect::Mark {
                    kind,
                    positions: positions.clone(),
                    resolves_at: deadline,
                });
            }
            self.pending.push(Pulse {
                kind,
                positions,
                deadline,
                ordinal,
            });
        }
        effects
    }
    fn draw(&self, pulse: u64, count: usize, index: u64) -> Option<usize> {
        if count == 0 {
            return None;
        }
        let mut seed = [0; 32];
        seed[..16].copy_from_slice(&self.run);
        seed[16..20].copy_from_slice(&self.wave.to_be_bytes());
        seed[20..28].copy_from_slice(&pulse.to_be_bytes());
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(seed);
        let occurrence = oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(self.run);
        bounded_draw(&root, occurrence, "drome_wave", index, count)
    }
}
#[derive(Debug, Clone)]
struct Pulse {
    kind: DromeModifier,
    positions: Vec<MovementLocalPosition>,
    deadline: u64,
    ordinal: u64,
}
fn seed_period(group_size: usize) -> u64 {
    (23 - 3 * group_size as u64) * 1_000_000
}
fn adjacent(a: MovementLocalPosition, b: MovementLocalPosition) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1
}

#[cfg(test)]
#[path = "tibiadrome_modifiers_tests.rs"]
mod tests;
