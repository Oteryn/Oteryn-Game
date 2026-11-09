use super::*;
use prepare::Plan;
pub(super) async fn load(
    tx: &mut PgConnection,
    operation: &str,
    world: &str,
) -> Result<Option<Plan>> {
    let r=sqlx::query("SELECT operation_id::text,world_id::text,actor::text,account_id::text,kind,seller::text,buyer::text,seller_character::text,buyer_character::text,buy_offer::text,sell_offer::text,reservation_id::text,parent::text,amount,price,fee,held_gold,binding,phase FROM game_coin_operations WHERE operation_id=$1::uuid AND world_id=$2::uuid FOR UPDATE").bind(operation).bind(world).fetch_optional(tx).await?;
    r.map(|r| {
        Ok(Plan {
            operation: r.try_get("operation_id")?,
            world: r.try_get("world_id")?,
            actor: r.try_get("actor")?,
            account: r.try_get("account_id")?,
            kind: r.try_get("kind")?,
            seller: r.try_get("seller")?,
            buyer: r.try_get("buyer")?,
            seller_character: r.try_get("seller_character")?,
            buyer_character: r.try_get("buyer_character")?,
            buy_offer: r.try_get("buy_offer")?,
            sell_offer: r.try_get("sell_offer")?,
            reservation: r.try_get("reservation_id")?,
            parent: r.try_get("parent")?,
            amount: r.try_get("amount")?,
            price: r.try_get("price")?,
            fee: r.try_get("fee")?,
            held: r.try_get("held_gold")?,
            binding: r.try_get("binding")?,
            phase: r.try_get("phase")?,
        })
    })
    .transpose()
}
fn required(v: &Option<String>) -> Result<&str> {
    v.as_deref().ok_or_else(invalid)
}
pub(super) fn request(p: &Plan) -> Result<PlatformRequest> {
    Ok(match p.kind.as_str() {
        "reserve" | "reserve_take" => PlatformRequest::Reserve {
            operation: p.operation.clone(),
            account: required(&p.seller)?.into(),
            amount: p.amount as u64,
            price: p.price as u64,
        },
        "settle" => PlatformRequest::Settle {
            operation: p.operation.clone(),
            reservation: required(&p.reservation)?.into(),
            buyer: required(&p.buyer)?.into(),
            amount: p.amount as u64,
        },
        "release" => PlatformRequest::Release {
            operation: p.operation.clone(),
            account: required(&p.seller)?.into(),
            reservation: required(&p.reservation)?.into(),
        },
        _ => return Err(invalid()),
    })
}
pub(super) async fn execute(
    tx: &mut PgConnection,
    world: &str,
    operation: &str,
    proof: &ReceiptProof,
) -> Result<()> {
    let mut p = load(tx, operation, world).await?.ok_or_else(invalid)?;
    if p.phase != 0 {
        return Ok(());
    }
    if proof.request() != &request(&p)?
        || proof.receipt().account_id != required(&p.seller)?
        || proof.receipt().amount != p.amount.to_string()
    {
        return Err(invalid());
    }
    let accounts = [Some(p.account.clone()), p.seller.clone(), p.buyer.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    lock_accounts(tx, world, &accounts).await?;
    sqlx::query(
        "INSERT INTO game_coin_receipts(operation_id,canonical_receipt) VALUES($1::uuid,$2)",
    )
    .bind(operation)
    .bind(proof.canonical().map_err(|_| invalid())?)
    .execute(&mut *tx)
    .await?;
    match p.kind.as_str() {
        "reserve" => {
            let expired:bool=sqlx::query_scalar("SELECT expires_at<=clock_timestamp() FROM game_coin_market_offers WHERE offer_id=$1::uuid").bind(required(&p.sell_offer)?).fetch_one(&mut *tx).await?;
            if expired {
                gold(
                    tx,
                    operation,
                    &p.account,
                    &p.actor,
                    world,
                    "expired-before-open",
                    p.held,
                    -p.held,
                    0,
                )
                .await?;
                sqlx::query("UPDATE game_coin_market_offers SET state=2 WHERE offer_id=$1::uuid")
                    .bind(required(&p.sell_offer)?)
                    .execute(&mut *tx)
                    .await?;
                let child: String = sqlx::query_scalar(
                    "SELECT game_market_identity($1::uuid,'coin-expiry-release')::text",
                )
                .bind(operation)
                .fetch_one(&mut *tx)
                .await?;
                let child = Plan {
                    operation: child,
                    world: p.world.clone(),
                    actor: p.actor.clone(),
                    account: p.account.clone(),
                    kind: "release".into(),
                    seller: p.seller.clone(),
                    buyer: None,
                    seller_character: p.seller_character.clone(),
                    buyer_character: None,
                    buy_offer: None,
                    sell_offer: p.sell_offer.clone(),
                    reservation: p.reservation.clone(),
                    parent: Some(p.operation.clone()),
                    amount: p.amount,
                    price: p.price,
                    fee: 0,
                    held: 0,
                    binding: p.binding.clone(),
                    phase: 0,
                };
                prepare::insert(tx, &child).await?;
                p.phase = 2;
            } else {
                gold(
                    tx, operation, &p.account, &p.actor, world, "open-fee", 0, -p.held, p.fee,
                )
                .await?;
                sqlx::query("UPDATE game_coin_market_offers SET state=1 WHERE offer_id=$1::uuid AND state=0").bind(required(&p.sell_offer)?).execute(&mut *tx).await?;
            }
        }
        "reserve_take" => {
            let child: String =
                sqlx::query_scalar("SELECT game_market_identity($1::uuid,'coin-settle')::text")
                    .bind(operation)
                    .fetch_one(&mut *tx)
                    .await?;
            p.phase = 2;
            let child = Plan {
                operation: child,
                world: p.world.clone(),
                actor: p.actor.clone(),
                account: p.account.clone(),
                kind: "settle".into(),
                seller: p.seller.clone(),
                buyer: p.buyer.clone(),
                seller_character: p.seller_character.clone(),
                buyer_character: p.buyer_character.clone(),
                buy_offer: p.buy_offer.clone(),
                sell_offer: None,
                reservation: Some(p.operation.clone()),
                parent: Some(p.operation.clone()),
                amount: p.amount,
                price: p.price,
                fee: 0,
                held: 0,
                binding: p.binding.clone(),
                phase: 0,
            };
            prepare::insert(tx, &child).await?;
        }
        "settle" => {
            let total = p.amount.checked_mul(p.price).ok_or_else(invalid)?;
            gold(
                tx,
                operation,
                required(&p.seller)?,
                required(&p.seller_character)?,
                world,
                "sale",
                total,
                -total,
                0,
            )
            .await?;
            if let Some(buy) = &p.buy_offer {
                let row=sqlx::query("SELECT remaining,price,held_gold FROM game_coin_market_offers WHERE offer_id=$1::uuid AND state=1 FOR UPDATE").bind(buy).fetch_one(&mut *tx).await?;
                let remaining: i64 = row.try_get("remaining")?;
                let limit_price: i64 = row.try_get("price")?;
                if remaining < p.amount
                    || limit_price < p.price
                    || row.try_get::<i64, _>("held_gold")? != remaining * limit_price
                {
                    return Err(invalid());
                }
                let refund = p.amount * (limit_price - p.price);
                if refund > 0 {
                    gold(
                        tx,
                        operation,
                        required(&p.buyer)?,
                        required(&p.buyer_character)?,
                        world,
                        "price-improvement",
                        refund,
                        -refund,
                        0,
                    )
                    .await?;
                }
                sqlx::query("UPDATE game_coin_market_offers SET remaining=remaining-$2,held_gold=(remaining-$2)*price,state=CASE WHEN remaining=$2 THEN 4 ELSE 1 END WHERE offer_id=$1::uuid").bind(buy).bind(p.amount).execute(&mut *tx).await?;
            } else if p.held != total {
                return Err(invalid());
            }
            if let Some(sell) = &p.sell_offer {
                let n=sqlx::query("UPDATE game_coin_market_offers SET remaining=remaining-$2,state=CASE WHEN remaining=$2 THEN 4 ELSE 1 END WHERE offer_id=$1::uuid AND state=1 AND remaining>=$2").bind(sell).bind(p.amount).execute(&mut *tx).await?.rows_affected();
                if n != 1 {
                    return Err(invalid());
                }
            }
            if let Some(parent) = &p.parent {
                sqlx::query("UPDATE game_coin_operations SET phase=1,last_error='' WHERE operation_id=$1::uuid AND phase=2").bind(parent).execute(&mut *tx).await?;
            }
        }
        "release" => {
            let n=sqlx::query("UPDATE game_coin_market_offers SET remaining=0,state=3 WHERE offer_id=$1::uuid AND state=2 AND remaining=$2").bind(required(&p.sell_offer)?).bind(p.amount).execute(&mut *tx).await?.rows_affected();
            if n != 1 {
                return Err(invalid());
            }
        }
        _ => return Err(invalid()),
    }
    if p.kind == "release"
        && let Some(parent) = &p.parent
    {
        sqlx::query("UPDATE game_coin_operations SET phase=1,last_error='' WHERE operation_id=$1::uuid AND phase=2").bind(parent).execute(&mut *tx).await?;
    }
    sqlx::query("UPDATE game_coin_operations SET phase=$2,held_gold=0,last_error='' WHERE operation_id=$1::uuid").bind(operation).bind(if p.phase==2{2_i16}else{1_i16}).execute(tx).await?;
    Ok(())
}
