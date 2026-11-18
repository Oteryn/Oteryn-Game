//! Select and record a reward under the Quest writer's live Character fence.
use crate::domain::{
    CharacterId,
    appearance::{AccountUnlocks, AppearanceContent, EarnedCosmetics, UnlockFact},
};
use crate::durability::{DurabilityError, character_appearance};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::world_callbacks::{Claim, ClaimReward};
use sqlx::{PgConnection, types::Uuid};

pub(crate) async fn choose<'a>(
    tx: &mut PgConnection,
    character: CharacterId,
    native: &NativeGameplayState,
    claim: &'a Claim,
) -> Result<Option<(usize, &'a ClaimReward)>, DurabilityError> {
    let mut eligible: Vec<_> = claim.rewards.iter().enumerate().collect();
    if eligible
        .iter()
        .any(|(_, reward)| reward.exclude_owned_mount.is_some())
    {
        let cosmetics = native
            .cosmetics()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let account: Uuid = sqlx::query_scalar(
            "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(character.as_bytes().as_slice())
        .fetch_one(&mut *tx)
        .await?;
        character_appearance::lock_account_grants(tx, account).await?;
        let account = crate::domain::AccountId::from_bytes(*account.as_bytes())
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let facts = character_appearance::read_account_appearance_unlocks(
            tx,
            account,
            &cosmetics.earned_grant_keys(),
        )
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        let facts: Vec<_> = facts
            .into_iter()
            .filter(|fact| {
                matches!(
                    fact.source_kind.as_str(),
                    "quest" | "npc" | "mount_taming" | "item_action" | "task_board"
                )
            })
            .map(|fact| (fact.unlock_key, fact.content_revision))
            .collect();
        let unlocks = EarnedCosmetics {
            content: cosmetics,
            facts: &facts,
        };
        if eligible.iter().any(|(_, reward)| {
            reward
                .exclude_owned_mount
                .as_ref()
                .is_some_and(|mount| !cosmetics.mount_exists(mount))
        }) {
            return Err(DurabilityError::InvalidStoredState);
        }
        eligible.retain(|(_, reward)| {
            reward
                .exclude_owned_mount
                .as_ref()
                .is_none_or(|mount| unlocks.mount(mount) != UnlockFact::Compatible)
        });
    }
    if eligible.len() <= 1 {
        return Ok(eligible.first().copied());
    }
    let draw: i32 = sqlx::query_scalar("SELECT floor(random() * $1::integer)::integer")
        .bind(i32::try_from(eligible.len()).map_err(|_| DurabilityError::InvalidStoredState)?)
        .fetch_one(tx)
        .await?;
    let index = usize::try_from(draw).map_err(|_| DurabilityError::InvalidStoredState)?;
    eligible
        .get(index)
        .copied()
        .map(Some)
        .ok_or(DurabilityError::InvalidStoredState)
}
