use super::value::{ValueWrite, apply_value};
use super::*;
use crate::durability::character_progression::assert_item_only_fence;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::CommandRef;
use oteryn_gameplay_housing::house_auction::value::plan_bid_value;
use oteryn_gameplay_housing::house_auction::{Bid, bid_deadline, quote};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct HouseBidRequest {
    pub operation_id: [u8; 16],
    pub command: CommandRef,
    pub house_key: String,
    pub maximum: u64,
}

/// Backend participant, not a protocol handler or a qualified catalogue producer.
/// Errors require rollback, including inside a pre-delivery Premium savepoint.
pub(super) async fn bid_in_transaction(
    tx: &mut Tx<'_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &HouseBidRequest,
    premium_allowed: bool,
) -> Result<HouseOwnershipOutcome> {
    if request.command.game_session_id() != fence.game_session_id
        || super::super::item_mint_audit::check_uuid_v7(&request.operation_id).is_err()
        || request.house_key.len() > 512
        || request.maximum > super::super::bank_audit::BANK_BALANCE_MAX
    {
        return Err(HouseOwnershipError::InvalidInput);
    }
    let world = fence.runtime_scope.world_id();
    let binding: [u8; 32] = Sha256::digest(
        serde_json::to_vec(&(
            request.operation_id,
            request.command.game_session_id().as_bytes(),
            request.command.command_id().get(),
            fence.character_id.as_bytes(),
            world.as_bytes(),
            &request.house_key,
            request.maximum,
        ))
        .map_err(|_| HouseOwnershipError::InvalidInput)?,
    )
    .into();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex')||$2,143))")
        .bind(request.command.game_session_id().as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string())
        .execute(&mut **tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex'),143))")
        .bind(request.operation_id.as_slice())
        .execute(&mut **tx)
        .await?;
    if let Some(row)=sqlx::query("SELECT request_binding FROM game_house_operations WHERE operation_id=encode($1,'hex')::uuid OR (game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric)")
        .bind(request.operation_id.as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await? {
        return if row.try_get::<Vec<u8>,_>("request_binding")?==binding {Ok(HouseOwnershipOutcome::Replayed)}else{Err(HouseOwnershipError::RequestConflict)};
    }
    if !premium_allowed {
        return Err(HouseOwnershipError::NotPremium);
    }
    assert_item_only_fence(tx, fence, node)
        .await?
        .map_err(|_| HouseOwnershipError::AuthorityRejected)?;
    let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let account: [u8; 16] = account
        .try_into()
        .map_err(|_| HouseOwnershipError::InvalidInput)?;
    let property = lock_property(tx, world.as_bytes(), &request.house_key).await?;
    if property.try_get::<&str, _>("kind")? == "guildhall" {
        return Err(HouseOwnershipError::Ineligible);
    }
    // Opening cannot be authorized by an anchor. The bundle lane supplies the
    // qualified acquisition catalogue together with accepted runtime entry/exit.
    if property.try_get::<i16, _>("state")? == 1 {
        return Err(HouseOwnershipError::CatalogueUnavailable);
    }
    if property.try_get::<i16, _>("state")? != 2 {
        return Err(HouseOwnershipError::AuctionClosed);
    }
    let auction=sqlx::query("SELECT uuid_send(auction_id) AS auction,first_rent,(extract(epoch FROM ends_at)*1000000)::bigint AS end_us,next_order FROM game_house_auction_lock(encode($1,'hex')::uuid,$2,NULL)")
        .bind(world.as_bytes().as_slice()).bind(&request.house_key).fetch_optional(&mut **tx).await?
        .ok_or(HouseOwnershipError::AuctionClosed)?;
    let auction_id = uuid(&auction, "auction")?;
    let now = clock(tx).await?;
    let end = bid_deadline(now, Some(auction.try_get("end_us")?))?;
    account_lock(tx, &account, world.as_bytes()).await?;
    let competing: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_bids WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND state=1 AND auction_id<>encode($3,'hex')::uuid)")
        .bind(account.as_slice()).bind(world.as_bytes().as_slice()).bind(auction_id.as_slice()).fetch_one(&mut **tx).await?;
    if competing {
        return Err(HouseOwnershipError::BidConflict);
    }
    let rows = held(tx, &auction_id).await?;
    let old = rows
        .iter()
        .find(|row| uuid(row, "account").ok() == Some(account));
    if old.is_none() && rows.len() == 256 {
        return Err(HouseOwnershipError::AuctionFull);
    }
    if let Some(old) = old
        && uuid(old, "character")? != *fence.character_id.as_bytes()
    {
        return Err(HouseOwnershipError::BidConflict);
    }
    let bids = rows
        .iter()
        .map(|row| {
            Ok(Bid {
                account: uuid(row, "account")?,
                maximum: gold(row, "maximum")?,
                maximum_reached_order: gold(row, "reached_order")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if quote(&bids)?.is_some_and(|q| request.maximum < q.price) {
        return Err(HouseOwnershipError::InvalidInput);
    }
    let escrow = old
        .map(|r| gold(r, "escrow_gold"))
        .transpose()?
        .unwrap_or(0);
    let first_rent = gold(&auction, "first_rent")?;
    let [balance] = super::super::bank::lock_balances(tx, world, &[account]).await?[..] else {
        return Err(HouseOwnershipError::InvalidInput);
    };
    let plan = plan_bid_value(
        balance.balance,
        escrow,
        request.maximum,
        first_rent,
        super::super::bank_audit::BANK_BALANCE_MAX,
    )?;
    let reached = match old {
        Some(old) if gold(old, "maximum")? == request.maximum => gold(old, "reached_order")?,
        _ => gold(&auction, "next_order")?,
    };
    let candidate = eligibility::read(
        tx,
        fence.character_id.as_bytes(),
        world.as_bytes(),
        Bid {
            account,
            maximum: request.maximum,
            maximum_reached_order: reached,
        },
        premium_allowed,
    )
    .await?;
    if candidate.facts.refusal() != 0 {
        return Err(HouseOwnershipError::Ineligible);
    }
    apply_value(
        tx,
        node,
        ValueWrite {
            operation: request.operation_id,
            auction: auction_id,
            world,
            house: &request.house_key,
            account,
            character: *fence.character_id.as_bytes(),
            kind: 1,
            binding,
            command: Some(request.command),
            fence: Some(fence),
            maximum: request.maximum,
            first_rent,
            price: 0,
            reached,
            ends_at_us: end,
            now_us: now,
            plan: &plan,
            eligibility: std::slice::from_ref(&candidate),
            ownership_before: super::super::character_progression::numeric_u64(
                &property,
                "ownership_generation",
            )?,
        },
    )
    .await?;
    Ok(HouseOwnershipOutcome::Applied)
}
