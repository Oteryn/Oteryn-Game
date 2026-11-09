use super::{Panel, RaceFilter};
use oteryn_client_runtime::charm_progression::catalogue::Card;
impl Panel {
    pub fn catalogue_rows(&self) -> Vec<&Card> {
        let mut rows: Vec<_> = self.catalogue.page.cards.iter().collect();
        if self.race_filter != RaceFilter::All {
            rows.extend(&self.selector.races);
            rows.sort_by_key(|c| c.race);
            rows.dedup_by_key(|c| c.race);
        }
        let category = self
            .catalogue
            .category
            .checked_sub(1)
            .and_then(|i| self.catalogue.page.categories.get(usize::from(i)));
        rows.into_iter()
            .filter(|card| {
                if category.is_some_and(|c| *c != card.category) {
                    return false;
                }
                let progress = self
                    .reply
                    .iter()
                    .flat_map(|r| &r.bestiary)
                    .find(|r| r.race.get() == u32::from(card.race));
                match self.race_filter {
                    RaceFilter::All => true,
                    RaceFilter::InProgress => progress.is_some_and(|r| !r.is_complete()),
                    RaceFilter::Complete => progress.is_some_and(|r| r.is_complete()),
                }
            })
            .collect()
    }
    pub fn catalogue_label(&self, card: &Card) -> String {
        let progress = self
            .reply
            .iter()
            .flat_map(|r| &r.bestiary)
            .find(|r| r.race.get() == u32::from(card.race));
        progress.map_or_else(
            || {
                format!(
                    "{} — {}",
                    card.name,
                    if self.reply.is_some() {
                        "undiscovered"
                    } else {
                        "progress not loaded"
                    }
                )
            },
            |r| {
                format!(
                    "{} — {} kills, stage {}/3",
                    card.name,
                    r.kill_count,
                    r.unlocked_stage()
                )
            },
        )
    }
}
