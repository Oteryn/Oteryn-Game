use super::*;
use sha2::{Digest, Sha256};
pub(super) async fn execute(tx: &mut PgConnection, id: Identity, r: Request) -> Result<Value> {
    let bytes = oteryn_protocol_oteryn::coin_market::encode(&r).map_err(|_| invalid())?;
    if !matches!(r.action, 3..=6) {
        return Err(invalid());
    }
    let account = id.account()?.to_owned();
    let actor = id.actor()?.to_owned();
    let op = uuid(&r.operation)?;
    let mut hash = Sha256::new();
    hash.update(b"oteryn:coin-market:confirmation:v1");
    hash.update(&bytes);
    hash.update(account.as_bytes());
    hash.update(id.world.as_bytes());
    let binding = hash.finalize().to_vec();
    if let Some(old) =
        sqlx::query("SELECT binding,phase FROM game_coin_operations WHERE operation_id=$1::uuid")
            .bind(&op)
            .fetch_optional(&mut *tx)
            .await?
    {
        return Ok(if old.try_get::<Vec<u8>, _>("binding")? == binding {
            json!({"result":"ok","operation":op,"pending":old.try_get::<i16,_>("phase")?!=1})
        } else {
            json!({"result":"idempotency_conflict"})
        });
    }
    let mut plan = Plan {
        operation: op.clone(),
        world: id.world.clone(),
        actor: actor.clone(),
        account: account.clone(),
        kind: String::new(),
        seller: None,
        buyer: None,
        seller_character: None,
        buyer_character: None,
        buy_offer: None,
        sell_offer: None,
        reservation: None,
        parent: None,
        amount: r.amount as i64,
        price: r.price as i64,
        fee: 0,
        held: 0,
        binding,
        phase: 0,
    };
    if matches!(r.action, 3 | 4) {
        let total = r
            .amount
            .checked_mul(r.price)
            .filter(|v| *v <= 999_999_999_999)
            .ok_or_else(invalid)? as i64;
        let fee = (total / 50).clamp(20, 1_000_000);
        let side = if r.action == 4 { 1_i16 } else { 2_i16 };
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM game_coin_market_offers WHERE account_id=$1::uuid AND world_id=$2::uuid AND state IN(0,1,2) UNION ALL SELECT 1 FROM game_market_offers WHERE account_id=$1::uuid AND world_id=$2::uuid AND state IN(1,5) LIMIT 100) counted").bind(&account).bind(&id.world).fetch_one(&mut *tx).await?;
        if count >= 100 {
            return Ok(json!({"result":"offer_limit"}));
        }
        lock_accounts(tx, &id.world, std::slice::from_ref(&account)).await?;
        let needed = fee + if side == 1 { total } else { 0 };
        if balance(tx, &account, &id.world).await? < needed {
            return Ok(json!({"result":"insufficient_gold"}));
        }
        sqlx::query("INSERT INTO game_coin_market_offers(offer_id,world_id,account_id,character_id,side,amount,remaining,price,held_gold,state) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5,$6,$6,$7,$8,$9)").bind(&op).bind(&id.world).bind(&account).bind(&actor).bind(side).bind(r.amount as i64).bind(r.price as i64).bind(if side==1{total}else{0}).bind(if side==1{1_i16}else{0_i16}).execute(&mut *tx).await?;
        plan.fee = fee;
        plan.reservation = Some(op.clone());
        if side == 1 {
            plan.kind = "buy".into();
            plan.buyer = Some(account.clone());
            plan.buyer_character = Some(actor.clone());
            plan.buy_offer = Some(op.clone());
            plan.phase = 1;
        } else {
            plan.kind = "reserve".into();
            plan.seller = Some(account.clone());
            plan.seller_character = Some(actor.clone());
            plan.sell_offer = Some(op.clone());
            plan.held = fee;
        }
        insert(tx, &plan).await?;
        if side == 1 {
            gold(
                tx,
                &op,
                &account,
                &actor,
                &id.world,
                "place-escrow",
                -total,
                total,
                0,
            )
            .await?;
            gold(
                tx,
                &op,
                &account,
                &actor,
                &id.world,
                "place-fee",
                -fee,
                0,
                fee,
            )
            .await?;
        } else {
            gold(tx, &op, &account, &actor, &id.world, "place", -fee, fee, 0).await?;
        }
    } else {
        let offer = uuid(&r.offer)?;
        let row=sqlx::query("SELECT account_id::text,character_id::text,side,remaining,price,state,held_gold,expires_at>clock_timestamp() AS live FROM game_coin_market_offers WHERE offer_id=$1::uuid AND world_id=$2::uuid FOR UPDATE").bind(&offer).bind(&id.world).fetch_optional(&mut *tx).await?;
        let Some(row) = row else {
            return Ok(json!({"result":"not_found"}));
        };
        let maker: String = row.try_get("account_id")?;
        let maker_character: String = row.try_get("character_id")?;
        let side: i16 = row.try_get("side")?;
        let remaining: i64 = row.try_get("remaining")?;
        let price: i64 = row.try_get("price")?;
        let busy:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_coin_operations WHERE (buy_offer=$1::uuid OR sell_offer=$1::uuid) AND phase IN(0,2))").bind(&offer).fetch_one(&mut *tx).await?;
        if row.try_get::<i16, _>("state")? != 1
            || busy
            || (r.action == 5 && !row.try_get::<bool, _>("live")?)
        {
            return Ok(json!({"result":"offer_busy_or_closed"}));
        }
        if (r.action == 6) != (maker == account) {
            return Ok(json!({"result":"rejected"}));
        }
        if r.action == 5 && (r.amount as i64 > remaining || r.amount == 0) {
            return Ok(json!({"result":"invalid_amount"}));
        }
        lock_accounts(tx, &id.world, &[account.clone(), maker.clone()]).await?;
        plan.price = price;
        if side == 1 {
            plan.buy_offer = Some(offer.clone());
            plan.buyer = Some(maker.clone());
            plan.buyer_character = Some(maker_character.clone());
        } else {
            plan.sell_offer = Some(offer.clone());
            plan.seller = Some(maker.clone());
            plan.seller_character = Some(maker_character.clone());
        }
        if r.action == 6 {
            plan.amount = remaining;
            if side == 1 {
                plan.kind = "cancel_buy".into();
                plan.phase = 1;
                insert(tx, &plan).await?;
                sqlx::query("UPDATE game_coin_market_offers SET remaining=0,held_gold=0,state=3 WHERE offer_id=$1::uuid").bind(&offer).execute(&mut *tx).await?;
                let held: i64 = row.try_get("held_gold")?;
                gold(
                    tx,
                    &op,
                    &account,
                    &actor,
                    &id.world,
                    "cancel-buy",
                    held,
                    -held,
                    0,
                )
                .await?;
            } else {
                plan.kind = "release".into();
                plan.reservation = Some(offer.clone());
                insert(tx, &plan).await?;
                sqlx::query("UPDATE game_coin_market_offers SET state=2 WHERE offer_id=$1::uuid")
                    .bind(&offer)
                    .execute(&mut *tx)
                    .await?;
            }
        } else if side == 2 {
            let total = (r.amount as i64).checked_mul(price).ok_or_else(invalid)?;
            if balance(tx, &account, &id.world).await? < total {
                return Ok(json!({"result":"insufficient_gold"}));
            }
            plan.kind = "settle".into();
            plan.reservation = Some(offer);
            plan.buyer = Some(account.clone());
            plan.buyer_character = Some(actor.clone());
            plan.held = total;
            insert(tx, &plan).await?;
            gold(
                tx,
                &op,
                &account,
                &actor,
                &id.world,
                "take-sell",
                -total,
                total,
                0,
            )
            .await?;
        } else {
            plan.kind = "reserve_take".into();
            plan.reservation = Some(op.clone());
            plan.seller = Some(account.clone());
            plan.seller_character = Some(actor.clone());
            insert(tx, &plan).await?;
        }
    }
    Ok(json!({"result":"ok","operation":op,"pending":plan.phase!=1}))
}
pub(super) async fn balance(tx: &mut PgConnection, account: &str, world: &str) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(account).bind(world).fetch_one(tx).await?)
}
pub(super) struct Plan {
    pub operation: String,
    pub world: String,
    pub actor: String,
    pub account: String,
    pub kind: String,
    pub seller: Option<String>,
    pub buyer: Option<String>,
    pub seller_character: Option<String>,
    pub buyer_character: Option<String>,
    pub buy_offer: Option<String>,
    pub sell_offer: Option<String>,
    pub reservation: Option<String>,
    pub parent: Option<String>,
    pub amount: i64,
    pub price: i64,
    pub fee: i64,
    pub held: i64,
    pub binding: Vec<u8>,
    pub phase: i16,
}
pub(super) async fn insert(tx: &mut PgConnection, p: &Plan) -> Result<()> {
    sqlx::query("INSERT INTO game_coin_operations(operation_id,world_id,actor,account_id,kind,seller,buyer,seller_character,buyer_character,buy_offer,sell_offer,reservation_id,parent,amount,price,fee,held_gold,binding,phase) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5,$6::uuid,$7::uuid,$8::uuid,$9::uuid,$10::uuid,$11::uuid,$12::uuid,$13::uuid,$14,$15,$16,$17,$18,$19)").bind(&p.operation).bind(&p.world).bind(&p.actor).bind(&p.account).bind(&p.kind).bind(&p.seller).bind(&p.buyer).bind(&p.seller_character).bind(&p.buyer_character).bind(&p.buy_offer).bind(&p.sell_offer).bind(&p.reservation).bind(&p.parent).bind(p.amount).bind(p.price).bind(p.fee).bind(p.held).bind(&p.binding).bind(p.phase).execute(tx).await?;
    Ok(())
}
