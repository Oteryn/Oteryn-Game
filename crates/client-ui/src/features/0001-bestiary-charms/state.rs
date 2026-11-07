use super::{BestiaryRaceProgress, CharmView};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

/// The client's Bestiary and Charm state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cyclopedia {
    pub(super) bestiary: BTreeMap<NonZeroU32, BestiaryRaceProgress>,
    pub(super) charms: CharmView,
    pub(super) feedback: Option<&'static str>,
}

impl Cyclopedia {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A Bestiary snapshot replaces every race.
    pub fn apply_bestiary_snapshot(&mut self, races: Vec<BestiaryRaceProgress>) {
        self.bestiary.clear();
        self.apply_bestiary_delta(races);
    }

    /// A Bestiary delta upserts the listed races and leaves the others unchanged.
    pub fn apply_bestiary_delta(&mut self, races: Vec<BestiaryRaceProgress>) {
        self.bestiary
            .extend(races.into_iter().map(|race| (race.race, race)));
    }

    /// A Charm snapshot or delta: both carry the whole view.
    pub fn apply_charm_view(&mut self, view: CharmView) {
        self.charms = view;
    }
}
