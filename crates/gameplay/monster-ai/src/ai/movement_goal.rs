//! CREATURE-MOVE-1 §1.5. Thinks select goals; only the step deadline evaluates a move.
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use crate::movement::{
    CardinalStep,
    path_search::{chebyshev, delta},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementGoal {
    Stand,
    Chase {
        actor: ExactActorRef,
        at: MovementLocalPosition,
    },
    KeepDistance {
        actor: ExactActorRef,
        at: MovementLocalPosition,
        distance: u16,
    },
    Flee {
        actor: ExactActorRef,
        at: MovementLocalPosition,
    },
    Wander {
        anchor: MovementLocalPosition,
        radius: u16,
        interval_ms: u64,
    },
    WalkBack {
        spawn: MovementLocalPosition,
    },
}

pub(crate) const CARDINALS: [CardinalStep; 4] = [
    CardinalStep::North,
    CardinalStep::East,
    CardinalStep::South,
    CardinalStep::West,
];

pub(crate) fn destination(
    at: MovementLocalPosition,
    step: CardinalStep,
) -> Option<MovementLocalPosition> {
    let (dx, dy) = delta(step);
    Some(MovementLocalPosition {
        x: at.x.checked_add(dx)?,
        y: at.y.checked_add(dy)?,
        floor: at.floor,
    })
}

impl MovementGoal {
    pub(crate) fn target(self) -> Option<(ExactActorRef, MovementLocalPosition)> {
        match self {
            Self::Chase { actor, at }
            | Self::KeepDistance { actor, at, .. }
            | Self::Flee { actor, at } => Some((actor, at)),
            _ => None,
        }
    }

    pub(crate) fn local_step(
        self,
        position: MovementLocalPosition,
        static_chance: u32,
        draw: u64,
        admits: impl Fn(MovementLocalPosition) -> bool,
    ) -> Option<CardinalStep> {
        let candidates = || {
            CARDINALS.into_iter().filter_map(|step| {
                let next = destination(position, step)?;
                admits(next).then_some((step, next))
            })
        };
        match self {
            Self::Chase { at, .. } if chebyshev(position, at) == 1 => {
                if draw % 1_000_000 < u64::from(static_chance) {
                    return None;
                }
                let steps: Vec<_> = candidates()
                    .filter(|(_, next)| chebyshev(*next, at) == 1)
                    .collect();
                let index = usize::try_from(draw / 1_000_000 % 4).ok()?;
                (0..4)
                    .map(|offset| CARDINALS[(index + offset) % 4])
                    .find(|step| steps.iter().any(|(candidate, _)| candidate == step))
            }
            Self::KeepDistance { at, distance, .. } => {
                let current = chebyshev(position, at);
                let wanted = i64::from(distance);
                if current == wanted {
                    return None;
                }
                candidates()
                    .filter(|(_, next)| {
                        (chebyshev(*next, at) - wanted).abs() < (current - wanted).abs()
                    })
                    .min_by_key(|(_, next)| (chebyshev(*next, at) - wanted).abs())
                    .map(|(step, _)| step)
            }
            Self::Flee { at, .. } => {
                let current = chebyshev(position, at);
                if current >= 11 {
                    return None;
                }
                candidates()
                    .filter(|(_, next)| chebyshev(*next, at) >= current)
                    .max_by_key(|(step, next)| {
                        (
                            chebyshev(*next, at),
                            std::cmp::Reverse(CARDINALS.iter().position(|s| s == step)),
                        )
                    })
                    .map(|(step, _)| step)
            }
            Self::Wander { anchor, radius, .. } => {
                let step = CARDINALS[usize::try_from(draw % 4).ok()?];
                let next = destination(position, step)?;
                (next.floor == anchor.floor
                    && chebyshev(next, anchor) <= i64::from(radius)
                    && admits(next))
                .then_some(step)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct NearCounter(u8);
impl NearCounter {
    pub(crate) fn evaluate(&mut self, position: MovementLocalPosition, goal: MovementGoal) -> bool {
        match goal {
            MovementGoal::Chase { at, .. } | MovementGoal::KeepDistance { at, .. } => {
                if position.floor == at.floor && chebyshev(position, at) <= 1 {
                    self.0 = self.0.saturating_add(1).min(2);
                } else {
                    self.0 = self.0.saturating_sub(1);
                }
                self.0 > 0
            }
            _ => false,
        }
    }
}

impl super::CreatureAiState {
    pub(super) fn choose_goal(
        &self,
        position: MovementLocalPosition,
        target: Option<(ExactActorRef, MovementLocalPosition)>,
        fleeing: bool,
        attack_distance: u64,
        perceived: bool,
    ) -> MovementGoal {
        if !self.profile.behavior().movement.can_walk {
            MovementGoal::Stand
        } else {
            match target {
                Some((actor, at)) if fleeing => MovementGoal::Flee { actor, at },
                Some((actor, at)) if attack_distance > 1 => MovementGoal::KeepDistance {
                    actor,
                    at,
                    distance: u16::try_from(attack_distance).unwrap_or(u16::MAX),
                },
                Some((actor, at)) => MovementGoal::Chase { actor, at },
                None if !perceived => self
                    .spawn
                    .filter(|spawn| *spawn != position)
                    .map_or(MovementGoal::Stand, |spawn| MovementGoal::WalkBack {
                        spawn,
                    }),
                None => match self.profile.wander() {
                    Some(wander) if wander.interval_ms > 0 => MovementGoal::Wander {
                        anchor: self.anchor,
                        radius: wander.radius_tiles,
                        interval_ms: wander.interval_ms,
                    },
                    _ => MovementGoal::Stand,
                },
            }
        }
    }
}

#[cfg(test)]
mod movement_goal_tests;
