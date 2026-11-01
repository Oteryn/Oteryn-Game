//! Task-shop rewards use the existing earned-appearance owner. The task board owns only
//! its paid promotion-point count, which the Wheel adds to its ordinary eligibility budget.

use super::{TaskBoardCatalogue, TaskBoardCommand, TaskBoardRefusal, state};
use crate::content::native_gameplay::NativeCosmeticsView;
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, EarnedCosmetic, EarnedCosmetics, Sex, UnlockFact,
};
use crate::domain::{AccountId, CharacterId};
use crate::durability::DurabilityError;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_mint_audit::Type2Transaction;
use oteryn_gameplay_quests::quest::task_board::full::{
    SHOP_OFFERS, TaskBoardAction, TaskBoardData, TaskBoardEffects, TaskBoardRefusal as RuleRefusal,
    TaskBoardShopKind, TaskBoardShopOffer,
};
use sqlx::{PgConnection, Row};

pub const PROVENANCE: &str = "task-board-shop-v1";

struct Grant {
    key: String,
    target: String,
    kind: TaskBoardShopKind,
    addon: u8,
}

fn qualify(cosmetics: &NativeCosmeticsView, offer: &TaskBoardShopOffer) -> Option<Grant> {
    let target = match offer.kind {
        TaskBoardShopKind::Outfit => cosmetics.outfit_keys().find(|key| {
            [Sex::Male, Sex::Female].into_iter().any(|sex| {
                cosmetics.look_type(key, sex) == Some(u32::from(offer.appearance_or_item))
            })
        })?,
        TaskBoardShopKind::Mount => cosmetics.mount_keys().find(|key| {
            cosmetics
                .mount_metadata(key)
                .is_some_and(|(look, _, _)| look == u32::from(offer.appearance_or_item))
        })?,
        _ => return None,
    }
    .to_owned();
    let key = format!("oteryn:taskboard/shop/offer-{}", offer.id);
    let earned = match offer.kind {
        TaskBoardShopKind::Mount => EarnedCosmetic::Mount(&target),
        TaskBoardShopKind::Outfit if offer.addon == 0 => EarnedCosmetic::Outfit(&target),
        TaskBoardShopKind::Outfit => EarnedCosmetic::Addon(&target, offer.addon),
        _ => return None,
    };
    (cosmetics.earned_unlock(&key, PROVENANCE, earned) == UnlockFact::Compatible).then_some(Grant {
        key,
        target,
        kind: offer.kind,
        addon: offer.addon,
    })
}

fn owned(cosmetics: &NativeCosmeticsView, facts: &[(String, String)], grant: &Grant) -> bool {
    let earned = EarnedCosmetics {
        content: cosmetics,
        facts,
    };
    let fact = match grant.kind {
        TaskBoardShopKind::Mount => earned.mount(&grant.target),
        TaskBoardShopKind::Outfit if grant.addon == 0 => earned.outfit(&grant.target),
        TaskBoardShopKind::Outfit => earned.addon(&grant.target, grant.addon),
        _ => UnlockFact::Absent,
    };
    fact == UnlockFact::Compatible
}

async fn account(
    tx: &mut PgConnection,
    character: CharacterId,
) -> Result<sqlx::types::Uuid, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(tx)
    .await?)
}

async fn facts(
    tx: &mut PgConnection,
    account: sqlx::types::Uuid,
    cosmetics: &NativeCosmeticsView,
) -> Result<Vec<(String, String)>, DurabilityError> {
    let account = AccountId::from_bytes(*account.as_bytes())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(
        crate::durability::character_appearance::read_account_appearance_unlocks(
            tx,
            account,
            &cosmetics.earned_grant_keys(),
        )
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?
        .into_iter()
        .filter(|fact| {
            matches!(
                fact.source_kind.as_str(),
                "quest" | "npc" | "mount_taming" | "task_board"
            )
        })
        .map(|fact| (fact.unlock_key, fact.content_revision))
        .collect(),
    )
}

/// Refresh the JSON purchase index from real account entitlements before evaluating rules.
pub(super) async fn synchronize(
    tx: &mut PgConnection,
    fence: CurrentCharacterGameplayFence,
    data: &mut TaskBoardData,
    catalogue: &TaskBoardCatalogue,
) -> Result<(), DurabilityError> {
    data.shop_purchases.clear();
    let Some(cosmetics) = &catalogue.cosmetics else {
        return Ok(());
    };
    let owner = account(tx, fence.character_id).await?;
    let facts = facts(tx, owner, cosmetics).await?;
    for offer in SHOP_OFFERS {
        if let Some(grant) = qualify(cosmetics, offer)
            && owned(cosmetics, &facts, &grant)
        {
            data.shop_purchases.insert(offer.id);
        }
    }
    Ok(())
}

pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    effects: &TaskBoardEffects,
    catalogue: &TaskBoardCatalogue,
) -> Result<Option<TaskBoardRefusal>, DurabilityError> {
    if effects.outfit.is_none() && effects.mount.is_none() {
        return Ok(None);
    }
    let TaskBoardAction::BuyShop { offer } = &request.action else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let Some(offer) = SHOP_OFFERS.iter().find(|entry| entry.id == *offer) else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let Some(cosmetics) = &catalogue.cosmetics else {
        return Ok(Some(TaskBoardRefusal::UnsupportedEntitlement));
    };
    let Some(grant) = qualify(cosmetics, offer) else {
        return Ok(Some(TaskBoardRefusal::UnsupportedEntitlement));
    };
    let owner = account(tx, fence.character_id).await?;
    // The account lock precedes all earned-fact reads, so two Characters cannot pay twice.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("task-board-cosmetics:{owner}"))
        .execute(&mut ***tx)
        .await?;
    let facts = facts(tx, owner, cosmetics).await?;
    if owned(cosmetics, &facts, &grant) {
        return Ok(Some(TaskBoardRefusal::Rule(RuleRefusal::AlreadyPurchased)));
    }
    if grant.kind == TaskBoardShopKind::Outfit
        && grant.addon > 0
        && (EarnedCosmetics {
            content: cosmetics,
            facts: &facts,
        })
        .outfit(&grant.target)
            != UnlockFact::Compatible
    {
        return Ok(Some(TaskBoardRefusal::Rule(RuleRefusal::RequiresOutfit)));
    }
    let occurrence: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
        .fetch_one(&mut ***tx)
        .await?;
    let (_, cause, ordinal) = state::cause_key(request.cause);
    sqlx::query("INSERT INTO game_task_board_cosmetic_grants(grant_occurrence_id,character_id,cause_id,cause_ordinal,offer_id) VALUES($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5)")
        .bind(occurrence).bind(fence.character_id.as_bytes().as_slice())
        .bind(cause.as_slice()).bind(ordinal.to_string()).bind(i16::try_from(offer.id).map_err(|_| DurabilityError::InvalidStoredState)?)
        .execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES($1,$2,$3,$4,encode($5,'hex')::uuid,'task_board',uuid_send($1))")
        .bind(occurrence).bind(owner).bind(&grant.key).bind(PROVENANCE)
        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES($1,$2,$3)")
        .bind(owner).bind(&grant.key).bind(occurrence).execute(&mut ***tx).await?;
    Ok(None)
}

/// Existing Wheel owner calls this under its Character root lock. The board's guarded row
/// and receipt chain are the sole authority for paid promotion points.
pub async fn read_promotion_points_in_transaction(
    tx: &mut PgConnection,
    character: CharacterId,
) -> Result<u32, DurabilityError> {
    let row = sqlx::query("SELECT (body->>'promotion_points')::integer AS points FROM game_character_task_boards WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_optional(tx).await?;
    let Some(row) = row else {
        return Ok(0);
    };
    let points = u32::try_from(row.try_get::<i32, _>("points")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if points > 50 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(points)
}
