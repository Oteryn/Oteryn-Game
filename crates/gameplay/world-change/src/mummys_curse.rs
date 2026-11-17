//! Crystal 201189f8: startup/tables/item.lua:3103-3127 and
//! scripts/actions/worldchanges/the_mummys_curse/horestis_jars.lua:18-121.

pub const INTACT_JAR: u16 = 12511;
pub const BROKEN_JAR: u16 = 12506;
pub const RETRY_SECONDS: u64 = 30 * 60;
pub const JARS: [(u16, u16, u16, i8); 5] = [
    (50006, 32991, 32721, -8),
    (50007, 32940, 32778, -9),
    (50008, 32985, 32772, -10),
    (50009, 32936, 32752, -11),
    (50010, 32979, 32737, -12),
];
pub const HORESTIS_SPAWNS: [(u16, u16, i8, u64); 2] =
    [(32943, 32791, -12, 0), (32942, 32765, -12, 45)];

#[derive(Debug, PartialEq, Eq)]
pub enum JarOutcome {
    Refused,
    Prerequisite,
    CoolingDown,
    Failed { retry_at: u64 },
    Broken { progress: u8 },
    FinalJar,
}

/// Progress counts transformed jars. Five represents Crystal's global value
/// of four together with the transformed appearance of the fifth jar.
pub fn use_jar(action: u16, progress: u8, now: u64, retry_at: u64, roll: u8) -> JarOutcome {
    let Some(index) = action.checked_sub(JARS[0].0).filter(|&index| index < 5) else {
        return JarOutcome::Refused;
    };
    let index = index as u8;
    if index < progress {
        return JarOutcome::Refused;
    }
    if index > progress {
        return JarOutcome::Prerequisite;
    }
    if index == 4 {
        return JarOutcome::FinalJar;
    }
    if now < retry_at {
        return JarOutcome::CoolingDown;
    }
    match roll {
        1 => JarOutcome::Broken {
            progress: index + 1,
        },
        2..=10 => JarOutcome::Failed {
            retry_at: now.saturating_add(RETRY_SECONDS),
        },
        _ => JarOutcome::Refused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crystal_cooldown_applies_only_to_failed_rolls_and_expires_at_equality() {
        use JarOutcome::*;
        assert_eq!(use_jar(50006, 0, 20, 0, 10), Failed { retry_at: 1820 });
        assert_eq!(use_jar(50006, 0, 1819, 1820, 1), CoolingDown);
        assert_eq!(use_jar(50006, 0, 1820, 1820, 1), Broken { progress: 1 });
        assert_eq!(use_jar(50006, 0, 1820, 1820, 0), Refused);
        assert_eq!(use_jar(50006, 0, 1820, 1820, 11), Refused);
    }

    #[test]
    fn crystal_sequence_refuses_repeats_and_finishes_without_a_roll() {
        use JarOutcome::*;
        for (index, &(action, ..)) in JARS[..4].iter().enumerate() {
            let index = index as u8;
            assert_eq!(use_jar(action + 1, index, 0, 0, 1), Prerequisite);
            assert_eq!(
                use_jar(action, index, 0, 0, 1),
                Broken {
                    progress: index + 1
                }
            );
            assert_eq!(use_jar(action, index + 1, 0, 0, 1), Refused);
        }
        assert_eq!(use_jar(50010, 4, 0, u64::MAX, 10), FinalJar);
        assert_eq!(use_jar(50010, 5, 0, 0, 1), Refused);
        assert_eq!(
            HORESTIS_SPAWNS,
            [(32943, 32791, -12, 0), (32942, 32765, -12, 45)]
        );
    }
}
