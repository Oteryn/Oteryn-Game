//! Reuse the quest writer's already locked authoritative progression sample.
use oteryn_gameplay_quests::quest::QuestStateCatalogue;

pub(super) fn permits_level(catalogue: &QuestStateCatalogue, quest: &str, level: i64) -> bool {
    catalogue
        .authored_bindings()
        .and_then(|bindings| {
            bindings
                .quests
                .iter()
                .find(|binding| binding.quest == quest)
        })
        .and_then(|binding| binding.admission.as_ref())
        .is_none_or(|admission| level >= i64::from(admission.minimum_level))
}

pub(super) fn requires_premium(catalogue: &QuestStateCatalogue, quest: &str) -> bool {
    catalogue
        .authored_bindings()
        .and_then(|bindings| {
            bindings
                .quests
                .iter()
                .find(|binding| binding.quest == quest)
        })
        .and_then(|binding| binding.admission.as_ref())
        .is_some_and(|admission| admission.premium)
}

/// The Character root lock serializes prerequisite changes by the same owner.
/// This reads completed state and never creates or modifies the other quest.
pub(super) async fn prerequisites_hold(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: &[u8],
    catalogue: &QuestStateCatalogue,
    quest: &str,
) -> super::StoredResult<bool> {
    let Some(admission) = catalogue
        .authored_bindings()
        .and_then(|bindings| {
            bindings
                .quests
                .iter()
                .find(|binding| binding.quest == quest)
        })
        .and_then(|binding| binding.admission.as_ref())
    else {
        return Ok(true);
    };
    if admission.prerequisites.is_empty() {
        return Ok(true);
    }
    let keys: Vec<_> = admission
        .prerequisites
        .iter()
        .map(|required| required.quest.as_str())
        .collect();
    let completed: i64 = sqlx::query_scalar("SELECT count(*)::bigint FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=ANY($2::text[]) AND completed_character_revision IS NOT NULL")
        .bind(character).bind(keys).fetch_one(&mut **tx).await?;
    Ok(usize::try_from(completed).ok() == Some(admission.prerequisites.len()))
}
