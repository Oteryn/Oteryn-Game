//! Server-save-relative fire restoration from Thornfire spoiler 54126@1020842.

const HOUR_MS: i64 = 3_600_000;

pub const FILLED_BUCKET: &str = "oteryn:item.tibia.i12722";

// Crystal prepared_bucket.lua:9-23. The active Shadowthorn phase and
// placement still need to be checked by the terrain owner.
const FIRE_APPEARANCES: &[u32] = &[
    232, 5061, 5062, 5063, 5064, 5065, 5066, 5067, 6288, 6289, 7803, 19096, 32021,
];

/// Bounds of the 610 captured Shadowthorn stations. The caller still
/// proves the live map placement and appearance before a terrain CAS.
pub fn is_shadowthorn_fire(appearance: u32, x: u16, y: u16, depth: u8) -> bool {
    FIRE_APPEARANCES.contains(&appearance)
        && (33050..=33129).contains(&x)
        && (32135..=32210).contains(&y)
        && (2..=8).contains(&depth)
}

pub fn restore_deadline(save_ms: i64, extinguished_ms: i64) -> Option<i64> {
    if extinguished_ms < save_ms {
        return None;
    }
    [8, 16]
        .into_iter()
        .filter_map(|hours| save_ms.checked_add(hours * HOUR_MS))
        .find(|&boundary| extinguished_ms < boundary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thornfire_restore_only_selects_shadowthorn_fire_placements() {
        assert!(is_shadowthorn_fire(5063, 33074, 32166, 2));
        assert!(!is_shadowthorn_fire(5063, 33049, 32166, 2));
        assert!(!is_shadowthorn_fire(5063, 33074, 32166, 9));
        assert!(!is_shadowthorn_fire(408, 33074, 32166, 2));
    }

    #[test]
    fn thornfire_uses_the_first_future_save_relative_restore_boundary() {
        let save = 1_800_000_000_000;
        let eight = save + 8 * HOUR_MS;
        let sixteen = save + 16 * HOUR_MS;
        assert_eq!(restore_deadline(save, save - 1), None);
        assert_eq!(restore_deadline(save, save), Some(eight));
        assert_eq!(restore_deadline(save, eight - 1), Some(eight));
        assert_eq!(restore_deadline(save, eight), Some(sixteen));
        assert_eq!(restore_deadline(save, sixteen - 1), Some(sixteen));
        assert_eq!(restore_deadline(save, sixteen), None);
        assert_eq!(restore_deadline(i64::MAX, i64::MAX), None);
    }
}
