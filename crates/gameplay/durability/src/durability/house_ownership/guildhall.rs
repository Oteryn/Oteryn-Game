//! Guildhall participant of the existing House transaction. Native ports retain
//! the Guild/member/property locks through both ledger chains and custody writes.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use oteryn_gameplay_housing::guildhall::{self as rules, Escrow, GuildhallRefusal};
use serde_json::{Value, json};

pub(super) async fn bid(
    tx: &mut Tx<'_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &HouseBidRequest,
    binding: [u8; 32],
) -> Result<HouseOwnershipOutcome> {
    if super::super::bank::fee_payer_is_junior(fence.character_id) {
        return Err(HouseOwnershipError::Ineligible);
    }
    let text = super::super::admission_authority_guards::uuid_text;
    let mut p = json!({
        "operation":text(&request.operation_id), "binding":binding.as_slice(),
        "world":text(fence.runtime_scope.world_id().as_bytes()), "house":request.house_key,
        "actor":text(fence.character_id.as_bytes()), "session":text(fence.game_session_id.as_bytes()),
        "command":request.command.command_id().get().to_string(),
        "connection":fence.connection_generation.get().to_string(),
        "lease":fence.character_lease_generation.to_string(),
        "scope_generation":fence.scope_ownership_generation.get().to_string(),
        "maximum":request.maximum,"content_digest":request.content_digest.as_slice(),
        "catalogue_rent":request.catalogue_rent,
    });
    let state = port(tx, node, "prepare", &p).await?;
    if state["replayed"].as_bool() == Some(true) {
        return Ok(HouseOwnershipOutcome::Replayed);
    }
    if let Some(code) = state["refusal"].as_str() {
        return Err(refusal(code));
    }
    let number = |key: &str| state[key].as_u64().ok_or(HouseOwnershipError::InvalidInput);
    rules::qualify_acquisition(rules::AcquisitionFacts {
        rank: Some(u8::try_from(number("rank")?).map_err(|_| HouseOwnershipError::InvalidInput)?),
        guild_active: state["active"].as_bool() == Some(true),
        guild_has_hall: state["has_hall"].as_bool() != Some(false),
        guild_has_bid: state["has_other_bid"].as_bool() != Some(false),
        leader_personal_houses: u8::try_from(number("personal_houses")?)
            .map_err(|_| HouseOwnershipError::InvalidInput)?,
    })
    .map_err(HouseOwnershipError::Guildhall)?;
    let escrow = Escrow {
        guild: number("escrow_guild")?,
        leader: number("escrow_account")?,
    };
    let target = request
        .maximum
        .checked_add(number("rent")?)
        .ok_or(HouseOwnershipError::InvalidInput)?;
    if escrow.total().map_err(HouseOwnershipError::Guildhall)? == target
        && state["existing"].as_bool() == Some(true)
    {
        return Err(HouseOwnershipError::Guildhall(GuildhallRefusal::NoChange));
    }
    // Zero-rent, zero-gold first bids still create an immutable custody receipt.
    let (guild, account) = if target == 0 && state["existing"].as_bool() == Some(false) {
        (0, 0)
    } else {
        let change = escrow
            .resize(
                target,
                number("guild_balance")?,
                number("account_balance")?,
                state["same_leader"].as_bool() == Some(true),
            )
            .map_err(HouseOwnershipError::Guildhall)?;
        (change.escrow_after.guild, change.escrow_after.leader)
    };
    p["escrow_guild"] = guild.into();
    p["escrow_account"] = account.into();
    let result = port(tx, node, "apply", &p).await?;
    if let Some(code) = result["refusal"].as_str() {
        return Err(refusal(code));
    }
    Ok(HouseOwnershipOutcome::Applied)
}

async fn port(
    tx: &mut Tx<'_>,
    node: &NodeIncarnationProof,
    mode: &str,
    p: &Value,
) -> Result<Value> {
    sqlx::query_scalar(
        "SELECT game_guildhall_bid_port($1,$2,encode($3,'hex')::uuid,$4::text::numeric,$5)",
    )
    .bind(mode)
    .bind(p)
    .bind(node.fact().node_id().as_bytes().as_slice())
    .bind(node.fact().registration_revision().to_string())
    .bind(node.sql_possession_secret())
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(db) = error.as_database_error() {
            eprintln!(
                "GUILDHALL native {mode}: {} {}",
                db.code().as_deref().unwrap_or("unknown"),
                db.message()
            );
        }
        HouseOwnershipError::from(error)
    })
}

fn refusal(code: &str) -> HouseOwnershipError {
    let refusal = match code {
        "leader" => GuildhallRefusal::NotGuildLeader,
        "inactive" => GuildhallRefusal::GuildInactive,
        "hall" => GuildhallRefusal::GuildAlreadyHasHall,
        "bid" => GuildhallRefusal::GuildAlreadyBidding,
        "funds" => GuildhallRefusal::InsufficientFunds,
        "balance" => GuildhallRefusal::BalanceLimit,
        "stale_leader" => GuildhallRefusal::StaleLeader,
        "conflict" => return HouseOwnershipError::RequestConflict,
        "catalogue" => return HouseOwnershipError::CatalogueUnavailable,
        "closed" => return HouseOwnershipError::AuctionClosed,
        _ => return HouseOwnershipError::Ineligible,
    };
    HouseOwnershipError::Guildhall(refusal)
}
