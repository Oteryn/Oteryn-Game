//! Reuse the quest writer's already locked authoritative progression sample.
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTransition};

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

pub(super) fn requires_premium(
    catalogue: &QuestStateCatalogue,
    transition: &QuestTransition,
    npc_reward: bool,
) -> bool {
    let Some(bindings) = catalogue.authored_bindings() else {
        return false;
    };
    // Explorer Society's Crystal enrollment shares tracks with the Premium
    // field course. Its independently registered NPC exchanges keep their own gates.
    if npc_reward
        && transition.quest == "oteryn:quest.the_explorer_society_quest"
        && !bindings
            .reward_callbacks()
            .any(|callback| callback.key == transition.key)
    {
        return false;
    }
    bindings
        .quests
        .iter()
        .find(|binding| binding.quest == transition.quest)
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
    let mut keys = Vec::with_capacity(admission.prerequisites.len());
    for required in &admission.prerequisites {
        match super::native_prerequisites::completed(tx, character, &required.quest).await? {
            Some(true) => {}
            Some(false) => return Ok(false),
            None => keys.push(required.quest.as_str()),
        }
    }
    if keys.is_empty() {
        return Ok(true);
    }
    let required = keys.len();
    let completed: i64 = sqlx::query_scalar("SELECT count(*)::bigint FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=ANY($2::text[]) AND completed_character_revision IS NOT NULL")
        .bind(character).bind(keys).fetch_one(&mut **tx).await?;
    Ok(usize::try_from(completed).ok() == Some(required))
}
