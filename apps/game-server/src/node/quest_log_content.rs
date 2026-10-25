//! Boot composition of the admitted quest state and its source journal.

use super::serve::BootError;

pub(crate) fn load_quest_catalogue(
    load: impl FnOnce(
        &str,
    ) -> Result<
        oteryn_gameplay_quests::quest::loader::LoweredQuestState,
        oteryn_gameplay_quests::quest::loader::QuestLoadError,
    >,
    content_revision: &str,
) -> Result<
    (
        std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>,
        oteryn_gameplay_dispatch::gameplay_transport::quest_log::QuestLogLines,
        String,
    ),
    BootError,
> {
    let lowered = load(content_revision)
        .map_err(|_| BootError::ContentActivation("quest state catalogue"))?;
    let loaded = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)
        .map_err(|_| BootError::ContentActivation("quest log catalogue"))?;
    for excluded in &loaded.exclusions {
        super::serve::event(&format!(
            "event=quest_log_excluded quest={} mission={} reason={}",
            excluded.quest,
            excluded.mission.as_deref().unwrap_or("*"),
            excluded.reason
        ));
    }
    let lines =
        oteryn_gameplay_dispatch::gameplay_transport::quest_log::QuestLogLines::new(loaded.content)
            .ok_or(BootError::ContentActivation("quest log line bounds"))?;
    // TASKBOARD-1: the code-built weekly Task Board quest joins the lowered quests.
    let catalogue = oteryn_gameplay_quests::quest::task_board::extend(lowered.catalogue())
        .map_err(|_| BootError::ContentActivation("task board quest"))?;
    let counts = lowered.counts();
    let line = format!(
        "event=quest_catalogue state=loaded content_revision={content_revision} quests={} transitions={} not_supported={} not_supported_explicit={} not_supported_inexact={}",
        counts.quests,
        counts.transitions,
        counts.not_supported,
        counts.explicit_computed,
        counts.inexact,
    );
    Ok((std::sync::Arc::new(catalogue), lines, line))
}
