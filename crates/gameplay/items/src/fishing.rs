//! Global fishing rules shared by dispatch and the durable item owner.

pub const FISHING_ROD: &str = "oteryn:item.tibia.i3483";
pub const WORM: &str = "oteryn:item.tibia.i3492";
pub const FISH: &str = "oteryn:item.tibia.i3578";

pub const WATER: [u16; 26] = [
    629, 630, 631, 632, 633, 634, 4597, 4598, 4599, 4600, 4601, 4602, 4609, 4610, 4611, 4612, 4613,
    4614, 7236, 9582, 12560, 12562, 12561, 12563, 13988, 13989,
];

/// Global's chance, represented in thousandths of one percent to avoid floats.
pub fn catch_threshold(skill: u16) -> u32 {
    let scaled = 10_000_i64 + (i64::from(skill) - 10) * 597;
    scaled.clamp(10_000, 50_000) as u32
}

pub fn catches(skill: u16, roll: u32) -> bool {
    (1..=100_000).contains(&roll) && roll <= catch_threshold(skill)
}

pub fn is_water(production_key: &str) -> bool {
    production_key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse::<u16>().ok())
        .is_some_and(|id| WATER.contains(&id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_curve_is_clamped_and_uses_the_reference_slope() {
        assert_eq!(catch_threshold(10), 10_000);
        assert_eq!(catch_threshold(20), 15_970);
        assert_eq!(catch_threshold(77), 49_999);
        assert_eq!(catch_threshold(78), 50_000);
        assert!(catches(10, 10_000));
        assert!(!catches(10, 10_001));
    }

    #[test]
    fn only_the_global_water_catalogue_is_accepted() {
        assert!(is_water("oteryn:item.tibia.i4597"));
        assert!(is_water("oteryn:item.tibia.i13989"));
        assert!(!is_water("oteryn:item.tibia.i4596"));
    }
}
