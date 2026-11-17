//! World-scoped lifecycle for the lake north of Ghorza's tent.

pub const START: &str = "oteryn:quest-reward/krailos-water-start";
pub const CAPTURE: &str = "oteryn:quest-reward/krailos-water-capture";
pub const POUR: &str = "oteryn:quest-callback/krailos-water-pour";
pub const PURIFY: &str = "oteryn:quest-callback/krailos-water-purify";

const CARAFES: i16 = 6;
const POWDERS: i16 = 30;
// The quest packet describes a filled lake drying after a few days.
const WET_SAVES: u64 = 3;

#[derive(Clone, Copy)]
pub enum Action {
    Start,
    Pour,
    Purify,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Dry,
    DirtyWater,
    ShallowWater,
}

#[derive(Default)]
pub struct Lake {
    epoch: u64,
    progress: i16,
}

impl Lake {
    pub fn from_stored(epoch: u64, progress: i16) -> Option<Self> {
        (0..=CARAFES + POWDERS)
            .contains(&progress)
            .then_some(Self { epoch, progress })
    }

    pub fn stored(&self) -> (u64, i16) {
        (self.epoch, self.progress)
    }

    pub fn phase(&self, epoch: u64) -> Option<Phase> {
        let age = epoch.checked_sub(self.epoch)?;
        Some(if age >= WET_SAVES || self.progress < CARAFES {
            Phase::Dry
        } else if self.progress < CARAFES + POWDERS {
            Phase::DirtyWater
        } else {
            Phase::ShallowWater
        })
    }

    pub fn advance(&mut self, epoch: u64, action: Action) -> bool {
        let Some(age) = epoch.checked_sub(self.epoch) else {
            return false;
        };
        if (self.progress < CARAFES && age > 0) || age >= WET_SAVES {
            *self = Self { epoch, progress: 0 };
        }
        match action {
            Action::Start => self.progress < CARAFES,
            Action::Pour if self.progress < CARAFES => {
                self.progress += 1;
                true
            }
            Action::Purify if (CARAFES..CARAFES + POWDERS).contains(&self.progress) => {
                self.progress += 1;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Lake, Phase};

    #[test]
    fn krailos_lake_accepts_the_water_mission_only_while_dry() {
        let mut lake = Lake::default();
        assert!(lake.advance(10, Action::Start));
        for _ in 0..6 {
            assert!(lake.advance(10, Action::Pour));
        }
        assert_eq!(lake.phase(10), Some(Phase::DirtyWater));
        assert!(!lake.advance(10, Action::Start));
        for _ in 0..30 {
            assert!(lake.advance(10, Action::Purify));
        }
        assert!(!lake.advance(10, Action::Start));
        assert!(!lake.advance(10, Action::Purify));
        assert!(!lake.advance(10, Action::Pour));
        assert_eq!(lake.phase(12), Some(Phase::ShallowWater));
        assert_eq!(lake.phase(13), Some(Phase::Dry));
        assert!(lake.advance(13, Action::Start));
    }

    #[test]
    fn krailos_lake_server_save_discards_incomplete_filling() {
        let mut lake = Lake::default();
        for _ in 0..5 {
            assert!(lake.advance(10, Action::Pour));
        }
        assert!(lake.advance(11, Action::Pour));
        assert!(!lake.advance(11, Action::Purify));
        for _ in 0..5 {
            assert!(lake.advance(11, Action::Pour));
        }
        for _ in 0..29 {
            assert!(lake.advance(11, Action::Purify));
        }
        assert_eq!(lake.phase(12), Some(Phase::DirtyWater));
        assert!(lake.advance(12, Action::Purify));
        assert_eq!(lake.phase(12), Some(Phase::ShallowWater));
    }
}
