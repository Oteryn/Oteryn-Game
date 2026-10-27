//! Native encounter dispatch for the two sourced Rotten Blood room mechanics.
//!
//! Target acquisition remains with the creature/condition owners. This owner validates the
//! resulting chain geometry and produces room-scoped damage consequences atomically.

const GLOOM_INITIAL_RANGE_TILES: u16 = 3;
const GLOOM_JUMP_RANGE_TILES: u16 = 6;
const GLOOM_BASE_DAMAGE: u32 = 2_250;
const GLOOM_JUMP_NUMERATOR: u32 = 3;
const GLOOM_JUMP_DENOMINATOR: u32 = 2;
const ROTTEN_CHARGE_ROOM_DAMAGE: u32 = 1_250;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoomPosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

impl RoomPosition {
    fn in_range(self, other: Self, range: u16) -> bool {
        self.floor == other.floor
            && self.x.abs_diff(other.x) <= u32::from(range)
            && self.y.abs_diff(other.y) <= u32::from(range)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoomTarget {
    pub actor: u64,
    pub position: RoomPosition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RottenBloodEvent<'a> {
    GloomChain {
        origin: RoomPosition,
        targets: &'a [RoomTarget],
    },
    RottenChargeDropped {
        room_players: &'a [RoomTarget],
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgonyDamage {
    pub actor: u64,
    pub amount: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RottenBloodRefusal {
    EmptyChain,
    DuplicateTarget,
    OutsideChainRange,
    DamageOverflow,
}

/// Existing encounter owner for Rotten Blood room events.
#[derive(Default)]
pub struct RottenBloodEncounterOwner;

impl RottenBloodEncounterOwner {
    /// Dispatches a condition/creature-owned event into its encounter-room consequences.
    pub fn dispatch(
        &mut self,
        event: RottenBloodEvent<'_>,
    ) -> Result<Vec<AgonyDamage>, RottenBloodRefusal> {
        match event {
            RottenBloodEvent::GloomChain { origin, targets } => {
                let Some((first, rest)) = targets.split_first() else {
                    return Err(RottenBloodRefusal::EmptyChain);
                };
                if !origin.in_range(first.position, GLOOM_INITIAL_RANGE_TILES) {
                    return Err(RottenBloodRefusal::OutsideChainRange);
                }

                let mut result = Vec::with_capacity(targets.len());
                let mut previous = *first;
                let mut damage = GLOOM_BASE_DAMAGE;
                result.push(AgonyDamage {
                    actor: first.actor,
                    amount: damage,
                });
                for (jump_index, target) in rest.iter().enumerate() {
                    if result.iter().any(|hit| hit.actor == target.actor) {
                        return Err(RottenBloodRefusal::DuplicateTarget);
                    }
                    if !previous
                        .position
                        .in_range(target.position, GLOOM_JUMP_RANGE_TILES)
                    {
                        return Err(RottenBloodRefusal::OutsideChainRange);
                    }
                    let multiplier = u32::try_from(jump_index)
                        .ok()
                        .and_then(|index| index.checked_add(GLOOM_JUMP_NUMERATOR))
                        .ok_or(RottenBloodRefusal::DamageOverflow)?;
                    damage = GLOOM_BASE_DAMAGE
                        .checked_mul(multiplier)
                        .and_then(|value| value.checked_div(GLOOM_JUMP_DENOMINATOR))
                        .ok_or(RottenBloodRefusal::DamageOverflow)?;
                    result.push(AgonyDamage {
                        actor: target.actor,
                        amount: damage,
                    });
                    previous = *target;
                }
                Ok(result)
            }
            RottenBloodEvent::RottenChargeDropped { room_players } => {
                let mut result = Vec::with_capacity(room_players.len());
                for player in room_players {
                    if result
                        .iter()
                        .any(|hit: &AgonyDamage| hit.actor == player.actor)
                    {
                        return Err(RottenBloodRefusal::DuplicateTarget);
                    }
                    result.push(AgonyDamage {
                        actor: player.actor,
                        amount: ROTTEN_CHARGE_ROOM_DAMAGE,
                    });
                }
                Ok(result)
            }
        }
    }
}
