//! Crystal 201189f8, actions_strange_bucket.lua:26–75 and items.xml:55483–55505.

pub const QUESTLINE: &str =
    "oteryn:quest-progress/quest/u12_00/the_dream_courts/haunted_house/questline";
pub const FULL_BUCKET_KEY: &str = "oteryn:item.tibia.i29309";

pub const MUTATED_EGG: u32 = 29305;
pub const NORMAL_EGG: u32 = 233;
pub const EMPTY_BUCKET: u32 = 29310;
pub const FULL_BUCKET: u32 = 29309;
pub const SLIME_LOOK_TYPE: u32 = 19;
pub const SLIME_DURATION_MS: u32 = 120_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EggUse {
    pub filled_bucket: Option<u32>,
    pub restore_after_ms: u32,
}

pub fn is_bucket(item: u32) -> bool {
    matches!(item, 29307..=29310)
}

/// The egg changes even when there is too little ectoplasm, or the bucket is full.
pub fn use_on_egg(bucket: u32, questline: i64, roll: u32) -> Option<EggUse> {
    if !is_bucket(bucket) || questline < 1 || roll > 10 {
        return None;
    }
    let filled_bucket = match (bucket, roll >= 5) {
        (EMPTY_BUCKET, true) => Some(29307),
        (29307, true) => Some(29308),
        (29308, true) => Some(FULL_BUCKET),
        _ => None,
    };
    Some(EggUse {
        filled_bucket,
        restore_after_ms: roll * 60_000,
    })
}

pub fn bucket_duration_ms(bucket: u32) -> Option<u32> {
    match bucket {
        29307 => Some(180_000),
        29308 => Some(120_000),
        FULL_BUCKET => Some(60_000),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haunted_cellar_fill_threshold_and_lifetimes() {
        assert_eq!(use_on_egg(EMPTY_BUCKET, 0, 10), None);
        assert_eq!(use_on_egg(EMPTY_BUCKET, 1, 11), None);
        assert_eq!(
            use_on_egg(EMPTY_BUCKET, 1, 4),
            Some(EggUse {
                filled_bucket: None,
                restore_after_ms: 240_000,
            })
        );
        for (before, after, duration) in [
            (EMPTY_BUCKET, 29307, 180_000),
            (29307, 29308, 120_000),
            (29308, FULL_BUCKET, 60_000),
        ] {
            assert_eq!(use_on_egg(before, 1, 5).unwrap().filled_bucket, Some(after));
            assert_eq!(bucket_duration_ms(after), Some(duration));
        }
        assert_eq!(use_on_egg(FULL_BUCKET, 1, 10).unwrap().filled_bucket, None);
    }
}
