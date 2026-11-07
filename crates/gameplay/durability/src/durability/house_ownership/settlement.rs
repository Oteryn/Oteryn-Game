use super::value::{ValueWrite, apply_value};
use super::*;
use crate::durability::bank::{lock_balances, new_entry_id};
use crate::durability::runtime_scope_assignment::{
    NodeIncarnationProof, prove_current_incarnation,
};
use crate::foundation::WorldId;
use oteryn_gameplay_housing::house_auction::value::{plan_release_value, plan_winner_value};
use oteryn_gameplay_housing::house_auction::{Bid, FIRST_RENT_US, quote};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct HouseSettlementRequest {
    pub world: WorldId,
    pub house_key: String,
    pub auction: [u8; 16],
    /// Zero elects the winner; later steps refund one captured Account each.
    /// One Account operation keeps one House event per physical transaction.
    pub step: u32,
}

pub(super) async fn candidates(tx: &mut Tx<'_>, auction: &[u8; 16]) -> Result<Vec<[u8; 16]>> {
    let rows=sqlx::query("SELECT uuid_send(account_id) AS account FROM game_house_bids WHERE auction_id=encode($1,'hex')::uuid ORDER BY account_id LIMIT 257")
        .bind(auction.as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > 256 {
        return Err(HouseOwnershipError::AuctionFull);
    }
    rows.iter().map(|r| uuid(r, "account")).collect()
}

pub(super) async fn settle_in_transaction(
    tx: &mut Tx<'_>,
    node: &NodeIncarnationProof,
    request: &HouseSettlementRequest,
    accounts: &[[u8; 16]],
    premium: &BTreeSet<[u8; 16]>,
) -> Result<HouseOwnershipOutcome> {
    if request.step > 256 || request.house_key.len() > 512 {
        return Err(HouseOwnershipError::InvalidInput);
    }
    if !prove_current_incarnation(tx, node).await? {
        return Err(HouseOwnershipError::AuthorityRejected);
    }
    let fact = node.fact();
    let assignment=sqlx::query("SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1 AND state=1 AND holder_node_id=encode($2,'hex')::uuid AND holder_registration_revision=$3::text::numeric ORDER BY scope_key LIMIT 1 FOR SHARE")
        .bind(request.world.as_bytes().as_slice()).bind(fact.node_id().as_bytes().as_slice())
        .bind(fact.registration_revision().to_string()).fetch_optional(&mut **tx).await?;
    if assignment.is_none() {
        return Err(HouseOwnershipError::AuthorityRejected);
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex'),143))")
        .bind(request.auction.as_slice())
        .execute(&mut **tx)
        .await?;
    let replay: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_settlement_steps WHERE auction_id=encode($1,'hex')::uuid AND step=$2 AND world_id=encode($3,'hex')::uuid AND house_key=$4)")
        .bind(request.auction.as_slice()).bind(i64::from(request.step)).bind(request.world.as_bytes().as_slice()).bind(&request.house_key).fetch_one(&mut **tx).await?;
    if replay {
        return Ok(HouseOwnershipOutcome::Replayed);
    }
    if request.step == 0 {
        // Only settlement's first step takes Character roots. Later refund
        // steps return captured value even if its nominated Character is inactive.
        sqlx::query("SELECT r.character_id FROM game_character_roots r JOIN game_house_bids b USING(character_id) WHERE b.auction_id=encode($1,'hex')::uuid ORDER BY r.character_id LIMIT 256 FOR SHARE OF r")
            .bind(request.auction.as_slice()).fetch_all(&mut **tx).await?;
    }
    let property = lock_property(tx, request.world.as_bytes(), &request.house_key).await?;
    let auction=sqlx::query("SELECT settled,first_rent,(extract(epoch FROM ends_at)*1000000)::bigint AS end_us FROM game_house_auction_lock(encode($2,'hex')::uuid,$3,encode($1,'hex')::uuid)")
        .bind(request.auction.as_slice()).bind(request.world.as_bytes().as_slice()).bind(&request.house_key).fetch_optional(&mut **tx).await?
        .ok_or(HouseOwnershipError::InvalidInput)?;
    let now = clock(tx).await?;
    now.checked_add(FIRST_RENT_US)
        .ok_or(HouseOwnershipError::Value(AuctionError::Overflow))?;
    if now < auction.try_get("end_us")? {
        return Ok(HouseOwnershipOutcome::NotDue);
    }
    let actual = candidates(tx, &request.auction).await?;
    if accounts != actual {
        return Err(HouseOwnershipError::ConcurrentChange);
    }
    for account in accounts {
        account_lock(tx, account, request.world.as_bytes()).await?;
    }
    let rows = held(tx, &request.auction).await?;
    let settled = auction.try_get::<bool, _>("settled")?;
    if settled != (request.step > 0) {
        return Err(HouseOwnershipError::ConcurrentChange);
    }
    if rows.is_empty() {
        return if settled {
            Ok(HouseOwnershipOutcome::Replayed)
        } else {
            Err(HouseOwnershipError::InvalidInput)
        };
    }
    let mut valid = Vec::new();
    let mut census = Vec::new();
    if !settled {
        let ground: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_house_tiles t USING(world_id,spatial_position) WHERE t.world_id=encode($1,'hex')::uuid AND t.house_key=$2)")
            .bind(request.world.as_bytes().as_slice()).bind(&request.house_key).fetch_one(&mut **tx).await?;
        if ground {
            return Err(HouseOwnershipError::ConcurrentChange);
        }
        for row in &rows {
            let account = uuid(row, "account")?;
            let candidate = eligibility::read(
                tx,
                &uuid(row, "character")?,
                request.world.as_bytes(),
                Bid {
                    account,
                    maximum: gold(row, "maximum")?,
                    maximum_reached_order: gold(row, "reached_order")?,
                },
                premium.contains(&account),
            )
            .await?;
            if candidate.facts.refusal() == 0 {
                valid.push(candidate.bid);
            }
            census.push(candidate);
        }
    }
    let winner = quote(&valid)?;
    let mut applied = false;
    for row in &rows {
        let account = uuid(row, "account")?;
        let won = winner.filter(|q| q.leader == account);
        if applied || (winner.is_some() && won.is_none()) {
            continue;
        }
        let [balance] = lock_balances(tx, request.world, &[account]).await?[..] else {
            return Err(HouseOwnershipError::InvalidInput);
        };
        let maximum = gold(row, "maximum")?;
        let rent = gold(row, "first_rent")?;
        let escrow = gold(row, "escrow_gold")?;
        let cap = super::super::bank_audit::BANK_BALANCE_MAX;
        let plan = if let Some(q) = won {
            plan_winner_value(balance.balance, escrow, maximum, q.price, rent, cap)?
        } else {
            plan_release_value(balance.balance, escrow, cap)?
        };
        let operation = new_entry_id(tx).await?;
        let binding = Sha256::digest(
            [
                request.auction.as_slice(),
                &request.step.to_be_bytes(),
                account.as_slice(),
            ]
            .concat(),
        )
        .into();
        apply_value(
            tx,
            node,
            ValueWrite {
                operation,
                auction: request.auction,
                world: request.world,
                house: &request.house_key,
                account,
                character: uuid(row, "character")?,
                kind: if won.is_some() { 2 } else { 3 },
                binding,
                command: None,
                fence: None,
                maximum,
                first_rent: rent,
                price: won.map_or(0, |q| q.price),
                reached: gold(row, "reached_order")?,
                ends_at_us: auction.try_get("end_us")?,
                now_us: now,
                plan: &plan,
                eligibility: &census,
                ownership_before: super::super::character_progression::numeric_u64(
                    &property,
                    "ownership_generation",
                )?,
            },
        )
        .await?;
        applied = true;
    }
    sqlx::query("SELECT game_house_settlement_step(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,$5,encode($6,'hex')::uuid,$7::text::numeric,$8)")
        .bind(request.auction.as_slice()).bind(i64::from(request.step)).bind(request.world.as_bytes().as_slice()).bind(&request.house_key)
        .bind(winner.is_none()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
        .bind(node.sql_possession_secret()).execute(&mut **tx).await?;
    Ok(HouseOwnershipOutcome::Applied)
}
