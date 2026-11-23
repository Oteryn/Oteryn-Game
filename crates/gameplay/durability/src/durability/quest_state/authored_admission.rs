//! Reuse the quest writer's already locked authoritative progression sample.
use super::QuestCause;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTransition, authored_bindings::actor_admission::ActorAdmission,
};

pub(super) fn for_transition<'a>(
    catalogue: &'a QuestStateCatalogue,
    transition: &QuestTransition,
    cause: QuestCause,
) -> Option<&'a ActorAdmission> {
    let key = transition.key.as_str();
    // The new journey gates apply to its authored route. Published puzzles and
    // already-minted chest obligations retain their own admission and proof.
    let already_admitted = match transition.quest.as_str() {
        "oteryn:quest.the_paradox_tower_quest" => {
            key == "oteryn:quest-callback/paradox-skulls"
                || key.starts_with("oteryn:quest-callback/l6/paradox-")
        }
        "oteryn:quest.koshei_the_deathless_quest" => {
            matches!(cause, QuestCause::ClaimObligation(_))
                && matches!(
                    key,
                    "oteryn:quest-transition/quest/u8_1/koshei_the_deathless/koshei_the_deathless_legs/chest"
                        | "oteryn:quest-transition/quest/u8_1/koshei_the_deathless/koshei_the_deathless_platinum/chest"
                )
        }
        _ => false,
    };
    if already_admitted {
        return None;
    }
    catalogue
        .authored_bindings()?
        .quests
        .iter()
        .find(|binding| binding.quest == transition.quest)?
        .admission
        .as_ref()
}

/// The Character root lock serializes prerequisite changes by the same owner.
/// This reads completed state and never creates or modifies the other quest.
pub(super) async fn prerequisites_hold(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: &[u8],
    admission: Option<&ActorAdmission>,
) -> super::StoredResult<bool> {
    let Some(admission) = admission else {
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
