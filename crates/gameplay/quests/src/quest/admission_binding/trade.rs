//! Reuse the trade owner's canonical operations without rebuilding their effects here.
use super::super::QuestStateCatalogue;

pub(super) fn catalogue() -> Result<QuestStateCatalogue, String> {
    let empty = QuestStateCatalogue::new("registered-trade-owners", Vec::new(), Vec::new())
        .map_err(|error| format!("{error:?}"))?;
    super::super::trade_callbacks::extend(empty)
}
