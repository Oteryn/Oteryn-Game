//! Read bounded personal predicates while the existing MINT owner holds the root lock.
use super::*;

pub(super) async fn satisfied(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    claim: &str,
) -> std::result::Result<bool, DurabilityError> {
    let gates = oteryn_gameplay_quests::quest::reward_claim_gates::load()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let Some(gate) = gates.iter().find(|gate| gate.claim == claim) else {
        return Ok(true);
    };
    for condition in &gate.conditions {
        let stored: Option<(String, i64)> = sqlx::query_as(
            "SELECT quest_key,value FROM game_character_quest_tracks \
             WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
        )
        .bind(character.as_bytes().as_slice())
        .bind(&condition.track)
        .fetch_optional(&mut **tx)
        .await?;
        if !stored.is_some_and(|(quest, value)| {
            quest == gate.quest && (condition.minimum..=condition.maximum).contains(&value)
        }) {
            return Ok(false);
        }
    }
    Ok(true)
}
