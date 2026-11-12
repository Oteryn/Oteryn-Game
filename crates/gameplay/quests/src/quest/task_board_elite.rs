//! Owner-approved, provisional T201 values. Unknown Global numbers live only in content.
use super::{BountyBoard, TaskBoardRefusal};
use serde::Deserialize;
use std::sync::OnceLock;

pub const MAX_RARITY_CREATURES: usize = 4096;
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    schema: String,
    parity: String,
    decision: String,
    pub dust_by_difficulty: [u32; 4],
    pub base_basis_points: [u32; 3],
    pub increment_basis_points: [u32; 3],
    pub cap_basis_points: [u32; 3],
}
impl Policy {
    pub fn checked_in() -> Result<&'static Self, TaskBoardRefusal> {
        static TABLE: OnceLock<Option<Policy>> = OnceLock::new();
        TABLE
            .get_or_init(|| {
                let p: Policy = serde_json::from_str(include_str!(
                    "../../../../../content/combat/bounty_elite_v1.json"
                ))
                .ok()?;
                (p.schema == "OTERYN_BOUNTY_ELITE/v1"
                    && p.parity == "PARITY_PENDING"
                    && !p.decision.is_empty()
                    && p.cap_basis_points
                        .iter()
                        .try_fold(0_u32, |sum, v| sum.checked_add(*v))
                        .is_some_and(|sum| sum <= 10_000)
                    && (0..3).all(|i| {
                        p.base_basis_points[i] <= p.cap_basis_points[i]
                            && p.increment_basis_points[i] > 0
                    })
                    && p.dust_by_difficulty
                        .iter()
                        .all(|v| *v > 0 && v.checked_mul(8).is_some()))
                .then_some(p)
            })
            .as_ref()
            .ok_or(TaskBoardRefusal::InvalidInput)
    }
    pub fn weights(&self, misses: u16) -> [u32; 3] {
        std::array::from_fn(|i| {
            self.base_basis_points[i]
                .saturating_add(u32::from(misses).saturating_mul(self.increment_basis_points[i]))
                .min(self.cap_basis_points[i])
        })
    }
    pub fn grade(&self, misses: u16, roll: u32) -> u8 {
        let [silver, gold, platinum] = self.weights(misses);
        if roll <= platinum {
            3
        } else if roll <= platinum + gold {
            2
        } else if roll <= platinum + gold + silver {
            1
        } else {
            0
        }
    }
    fn maximum_misses(&self) -> u16 {
        (0..3)
            .map(|i| {
                (self.cap_basis_points[i] - self.base_basis_points[i])
                    .div_ceil(self.increment_basis_points[i])
            })
            .max()
            .unwrap_or(0)
            .min(u32::from(u16::MAX)) as u16
    }
}
/// An unchosen offer advances once; selecting an improved offer resets only its race.
pub(super) fn remember_offers(
    board: &mut BountyBoard,
    selected: Option<usize>,
) -> Result<(), TaskBoardRefusal> {
    let policy = Policy::checked_in()?;
    for (index, offer) in board.offers.iter().enumerate() {
        if selected == Some(index) {
            if offer.grade > 0 {
                board.rarity.remove(&offer.race);
            }
        } else {
            if board.rarity.len() >= MAX_RARITY_CREATURES && !board.rarity.contains_key(&offer.race)
            {
                return Err(TaskBoardRefusal::LimitReached);
            }
            let misses = board.rarity.entry(offer.race.clone()).or_default();
            *misses = misses.saturating_add(1).min(policy.maximum_misses());
        }
    }
    Ok(())
}
