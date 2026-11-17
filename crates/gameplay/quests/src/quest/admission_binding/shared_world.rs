//! Reuse the collector's exact operations; callback storage is only its physical selector.
use super::super::QuestStateCatalogue;

pub(super) fn catalogue() -> Result<QuestStateCatalogue, String> {
    let empty = QuestStateCatalogue::new("shared-world-owners", Vec::new(), Vec::new())
        .map_err(|error| format!("{error:?}"))?;
    super::super::shared_world_progress::extend(empty)
}
