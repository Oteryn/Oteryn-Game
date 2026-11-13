//! M18 selected encounter prerequisites, joined to the existing Boss Activity transaction.
//! Chosen First Dragon recipe, quests-00200-00299.json record23 s4/s5;
//! Crystal201189f the_first_dragon/actions_lever.lua:6 supplies the independent cooldown.
use super::*;
use crate::domain::CharacterId;

pub const FIRST_DRAGON: &str = "oteryn:boss.the_first_dragon";
const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const TRIALS: &str = "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s4";
const DEFEAT: &str = "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s5";

fn trials_permit(trials: Option<i64>, defeat: Option<i64>) -> bool {
    trials == Some(2) && matches!(defeat, None | Some(0))
}

/// Call after the participant's current Character root has been locked and fenced.
pub(super) async fn permits(
    tx: &mut Transaction<'_, Postgres>,
    boss: &str,
    character: CharacterId,
) -> Result<bool> {
    if boss != FIRST_DRAGON { return Ok(true); }
    let rows = sqlx::query("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND track_key=ANY($3) ORDER BY track_key FOR SHARE")
        .bind(character.as_bytes().as_slice()).bind(QUEST).bind([TRIALS, DEFEAT].as_slice())
        .fetch_all(&mut **tx).await?;
    let mut trials = None;
    let mut defeat = None;
    for row in rows {
        let key: String = row.try_get("track_key")?;
        if key == TRIALS { trials = Some(row.try_get("value")?); }
        if key == DEFEAT { defeat = Some(row.try_get("value")?); }
    }
    Ok(trials_permit(trials, defeat))
}

/// Premium authority is consumed inside the already fenced participant transaction.
pub(super) async fn permits_entry<G: super::super::quest_state::grizzly::GrizzlyPremiumGate>(
    tx: &mut Transaction<'_, Postgres>, boss: &str, character: CharacterId, premium: &G,
) -> Result<bool> {
    if !permits(tx, boss, character).await? { return Ok(false); }
    if boss != FIRST_DRAGON { return Ok(true); }
    let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    premium.task_permitted(tx, super::super::character_progression::uuid_text(&account)?).await
}

#[cfg(any(test, feature = "test-support"))]
pub async fn fixture_first_dragon_entry_permit<G: super::super::quest_state::grizzly::GrizzlyPremiumGate>(
    tx: &mut Transaction<'_, Postgres>, character: CharacterId, premium: &G,
) -> Result<bool> {
    permits_entry(tx, FIRST_DRAGON, character, premium).await
}

#[cfg(any(test, feature = "test-support"))]
pub async fn fixture_first_dragon_trials_permit(
    tx: &mut Transaction<'_, Postgres>, character: CharacterId,
) -> Result<bool> {
    permits(tx, FIRST_DRAGON, character).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m18_first_dragon_requires_finished_trials_and_no_prior_defeat() {
        for (trials, defeat) in [(None,None),(Some(0),None),(Some(1),None),(Some(3),None),(Some(2),Some(1)),(Some(2),Some(-1))] {
            assert!(!trials_permit(trials, defeat), "{trials:?}/{defeat:?}");
        }
        assert!(trials_permit(Some(2), None));
        assert!(trials_permit(Some(2), Some(0)));
    }
}
