//! W09 Drome Cube: Crystal 201189f8 `data/scripts/actions/items/drome_cube.lua:1-39`.
//! One consumed cube produces one of the twenty concoctions, with equal weight. There is no
//! Cube cooldown. The concoction's separate activation cooldown is not consulted here.

use crate::concoctions::{ALL, Concoction};
use sha2::{Digest, Sha256};

pub const DROME_CUBE_KEY: &str = "oteryn:item.tibia.i36827";
pub const REWARD_COUNT: u8 = 20;

/// This occurrence belongs to the source unit, not the client-chosen command. Refusing a use
/// because the backpack is full cannot let changing the command reroll the same unit.
/// Rejection sampling avoids modulo bias; entropy exhaustion fails closed rather than falling
/// back to a preferred reward. The durable receipt, not this function, answers a replay.
pub fn reward_index(source: [u8; 16], quantity: u32) -> Option<u8> {
    if quantity == 0 {
        return None;
    }
    let digest: [u8; 32] = Sha256::new()
        .chain_update(b"oteryn.drome-cube.reward.v1")
        .chain_update(source)
        .chain_update(u64::from(quantity).to_be_bytes())
        .finalize()
        .into();
    index_from_entropy(&digest)
}

fn index_from_entropy(entropy: &[u8]) -> Option<u8> {
    // 240 is the largest multiple of the twenty-entry pool below 256.
    entropy
        .iter()
        .copied()
        .find(|byte| *byte < 240)
        .map(|byte| byte % REWARD_COUNT)
}

pub fn reward(index: u8) -> Option<Concoction> {
    ALL.get(usize::from(index)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_donor_pool_entry_is_selected_with_equal_weight() {
        let mut counts = [0; 20];
        for byte in 0..=u8::MAX {
            if let Some(index) = index_from_entropy(&[byte]) {
                counts[usize::from(index)] += 1;
            }
        }
        assert_eq!(counts, [12; 20]);
        for index in 0..REWARD_COUNT {
            assert_eq!(
                reward(index).map(Concoction::item_id),
                Some(36723 + u32::from(index))
            );
        }
        assert_eq!(reward(REWARD_COUNT), None);
        assert_eq!(reward(u8::MAX), None);
    }

    #[test]
    fn rejection_uses_the_next_byte_and_never_substitutes_a_reward() {
        assert_eq!(index_from_entropy(&[255, 240, 19]), Some(19));
        assert_eq!(index_from_entropy(&[240; 32]), None);
        assert_eq!(index_from_entropy(&[]), None);
    }

    #[test]
    fn a_source_unit_has_one_stable_draw_and_zero_quantity_is_refused() {
        let first = reward_index([2; 16], 10);
        assert!(first.is_some_and(|index| index < REWARD_COUNT));
        assert_eq!(first, reward_index([2; 16], 10));
        assert_eq!(reward_index([2; 16], 0), None);
        // CommandId, connection generation and wall clock are deliberately not draw inputs.
        let units: std::collections::BTreeSet<_> = (1..=100)
            .filter_map(|quantity| reward_index([2; 16], quantity))
            .collect();
        assert!(units.len() > 1);
    }
}
