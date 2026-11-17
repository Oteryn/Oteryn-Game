//! Read-only cosmetic prerequisites use the existing account grant and appearance owners.
use crate::domain::appearance::{AccountUnlocks, AppearanceContent, EarnedCosmetics, UnlockFact};
use crate::domain::{AccountId, CharacterId};
use crate::durability::{DurabilityError, character_appearance};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::reward_callbacks::OutfitReward;
use sqlx::{PgConnection, types::Uuid};

/// The caller already holds its granting Character root fence. The shared account
/// lock serializes this observation with consumptive items and all equivalent grants.
pub(super) async fn holds(
    connection: &mut PgConnection,
    character: CharacterId,
    native: Option<&NativeGameplayState>,
    required: &[OutfitReward],
) -> Result<bool, DurabilityError> {
    let Some(native) = native else { return Ok(false); };
    let Some(content) = native.cosmetics().filter(|c| c.source_digest() == native.source_digest()) else {
        return Ok(false);
    };
    let account: Uuid = sqlx::query_scalar("SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&mut *connection).await?;
    character_appearance::lock_account_grants(connection, account).await?;
    let sex = character_appearance::item_action_sex(connection, character).await?;
    let account = AccountId::from_bytes(*account.as_bytes()).map_err(|_| DurabilityError::InvalidStoredState)?;
    let facts = character_appearance::read_account_appearance_unlocks(connection, account, &content.earned_grant_keys()).await
        .map_err(|error| match error {
            character_appearance::AppearanceReadError::Storage(error) => DurabilityError::from(error),
            character_appearance::AppearanceReadError::InvalidStoredState => DurabilityError::InvalidStoredState,
        })?;
    let facts: Vec<_> = facts.into_iter().filter(|fact| matches!(fact.source_kind.as_str(), "quest" | "npc" | "mount_taming" | "task_board" | "item_action"))
        .map(|fact| (fact.unlock_key, fact.content_revision)).collect();
    let owned = EarnedCosmetics { content, facts: &facts };
    Ok(required.iter().all(|required| {
        content.outfit_addon_count(&required.outfit).is_some_and(|maximum| {
            (content.outfit_is_starter(&required.outfit, sex) || owned.outfit(&required.outfit) == UnlockFact::Compatible)
                && (1..=2).all(|addon| required.addons & (1 << (addon - 1)) == 0
                    || addon <= maximum && (content.addon_is_starter(&required.outfit, sex, addon)
                        || owned.addon(&required.outfit, addon) == UnlockFact::Compatible))
        })
    }))
}
