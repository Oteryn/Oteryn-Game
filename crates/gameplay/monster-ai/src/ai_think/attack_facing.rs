//! Crystal `monster.cpp:2467-2523`: attack facing retains the prior axis on diagonal ties.
use super::{CardinalStep, CreatureAiState, MovementLocalPosition};

impl CreatureAiState {
    /// The current creature-facing fact used by directional attack geometry.
    #[must_use]
    pub const fn attack_facing(&self) -> CardinalStep {
        self.attack_facing
    }

    pub(super) fn update_attack_facing(
        &mut self,
        position: MovementLocalPosition,
        target: MovementLocalPosition,
    ) {
        self.attack_facing = look_direction(self.attack_facing, position, target);
    }
}

fn look_direction(
    current: CardinalStep,
    position: MovementLocalPosition,
    target: MovementLocalPosition,
) -> CardinalStep {
    use CardinalStep::{East, North, South, West};
    let dx = i64::from(target.x) - i64::from(position.x);
    let dy = i64::from(target.y) - i64::from(position.y);
    match dx.abs().cmp(&dy.abs()) {
        std::cmp::Ordering::Greater => {
            if dx < 0 {
                West
            } else {
                East
            }
        }
        std::cmp::Ordering::Less => {
            if dy < 0 {
                North
            } else {
                South
            }
        }
        std::cmp::Ordering::Equal => match (dx < 0, dy < 0, dx > 0, dy > 0, current) {
            (true, true, _, _, South) => West,
            (true, true, _, _, East) => North,
            (true, _, _, true, North) => West,
            (true, _, _, true, East) => South,
            (_, true, true, _, South) => East,
            (_, true, true, _, West) => North,
            (false, false, _, _, North) => East,
            (false, false, _, _, West) => South,
            _ => current,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crystal_attack_facing_uses_dominant_axis_and_prior_diagonal_direction() {
        use CardinalStep::{East, North, South, West};
        let origin = MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        for (dx, dy, expected) in [(2, 1, East), (-2, 1, West), (1, -2, North), (1, 2, South)] {
            let target = MovementLocalPosition {
                x: origin.x + dx,
                y: origin.y + dy,
                ..origin
            };
            for current in [North, East, South, West] {
                assert_eq!(look_direction(current, origin, target), expected);
            }
        }
        for (dx, dy, expected) in [
            (-1, -1, [North, North, West, West]),
            (-1, 1, [West, South, South, West]),
            (1, -1, [North, East, East, North]),
            (1, 1, [East, East, South, South]),
        ] {
            let target = MovementLocalPosition {
                x: origin.x + dx,
                y: origin.y + dy,
                ..origin
            };
            for (current, expected) in [North, East, South, West].into_iter().zip(expected) {
                assert_eq!(look_direction(current, origin, target), expected);
            }
        }
    }
}
