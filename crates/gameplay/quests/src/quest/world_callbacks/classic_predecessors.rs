//! Preserve mid-quest pins before adding the Inquisition's missing physical operations.
use super::{Callback, ChargeWindow, QuestStateCatalogue};
use crate::quest::NpcDefinitionPredecessor;

pub(crate) const QUEST: &str = "oteryn:quest.the_inquisition_quest";

pub(crate) fn includes(callback: &Callback) -> bool {
    callback.quest == QUEST
        && (callback.effects.use_summon.is_some()
            || matches!(
                callback.effects.charge_window,
                Some(ChargeWindow::Spend { .. })
            ))
}

pub(crate) fn retain(catalogue: &mut QuestStateCatalogue, hash: [u8; 32]) {
    if catalogue
        .definition_hash(QUEST)
        .is_some_and(|current| current != hash)
    {
        catalogue
            .predecessors
            .entry(QUEST.into())
            .or_default()
            .push(NpcDefinitionPredecessor {
                hash,
                defaults: Default::default(),
            });
    }
}

/// The original NPC, kill and authored stages have all registered at this point.
/// Crystal 201189f8 actions_vampire_hunt.lua:1-32 and actions_holy_water.lua:65-135.
pub(crate) fn attach(catalogue: &mut QuestStateCatalogue) -> Result<(), String> {
    let Some(hash) = catalogue.definition_hash(QUEST) else {
        return Ok(());
    };
    let arrivals: Vec<_> = super::load()?.into_iter().filter(includes).collect();
    if arrivals.is_empty() {
        return Ok(());
    }
    let extended = super::extend_callbacks(catalogue.clone(), &arrivals)?;
    let next = extended
        .definition_hash(QUEST)
        .ok_or("Inquisition definition disappeared")?;
    catalogue.tracks.extend(
        extended
            .tracks
            .into_iter()
            .filter(|(_, track)| track.quest == QUEST),
    );
    catalogue.transitions.extend(
        extended
            .transitions
            .into_iter()
            .filter(|(_, transition)| transition.quest == QUEST),
    );
    catalogue.hashes.insert(QUEST.into(), next);
    retain(catalogue, hash);
    Ok(())
}
