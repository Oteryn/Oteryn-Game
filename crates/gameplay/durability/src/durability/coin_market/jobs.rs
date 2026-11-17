use super::*;
use prepare::Plan;
pub(super) async fn tick(tx: &mut PgConnection, world: &str) -> Result<()> {
    // One book step per worker tick. Pending operations retain exclusive use of their offers.
    let expired=sqlx::query("SELECT offer_id::text,account_id::text,character_id::text,side,remaining,price,held_gold FROM game_coin_market_offers o WHERE world_id=$1::uuid AND state=1 AND expires_at<=clock_timestamp() AND NOT EXISTS(SELECT 1 FROM game_coin_operations p WHERE p.phase IN(0,2) AND (p.buy_offer=o.offer_id OR p.sell_offer=o.offer_id)) ORDER BY expires_at,offer_id LIMIT 1 FOR UPDATE").bind(world).fetch_optional(&mut *tx).await?;
    if let Some(r) = expired {
        let offer: String = r.try_get("offer_id")?;
        let a: String = r.try_get("account_id")?;
        let c: String = r.try_get("character_id")?;
        let side: i16 = r.try_get("side")?;
        let operation: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut *tx)
            .await?;
        lock_accounts(tx, world, std::slice::from_ref(&a)).await?;
        let p = Plan {
            operation: operation.clone(),
            world: world.into(),
            actor: c.clone(),
            account: a.clone(),
            kind: if side == 1 { "cancel_buy" } else { "release" }.into(),
            seller: (side == 2).then_some(a.clone()),
            buyer: (side == 1).then_some(a.clone()),
            seller_character: (side == 2).then_some(c.clone()),
            buyer_character: (side == 1).then_some(c.clone()),
            buy_offer: (side == 1).then_some(offer.clone()),
            sell_offer: (side == 2).then_some(offer.clone()),
            reservation: Some(offer.clone()),
            parent: None,
            amount: r.try_get("remaining")?,
            price: r.try_get("price")?,
            fee: 0,
            held: 0,
            binding: vec![0; 32],
            phase: if side == 1 { 1 } else { 0 },
        };
        prepare::insert(tx, &p).await?;
        if side == 1 {
            let held: i64 = r.try_get("held_gold")?;
            sqlx::query("UPDATE game_coin_market_offers SET remaining=0,held_gold=0,state=3 WHERE offer_id=$1::uuid").bind(&offer).execute(&mut *tx).await?;
            gold(tx, &operation, &a, &c, world, "expiry", held, -held, 0).await?;
        } else {
            sqlx::query("UPDATE game_coin_market_offers SET state=2 WHERE offer_id=$1::uuid")
                .bind(&offer)
                .execute(&mut *tx)
                .await?;
        }
        return Ok(());
    }
    let pair=sqlx::query("SELECT b.offer_id::text AS buy,s.offer_id::text AS sell,b.account_id::text AS buyer,s.account_id::text AS seller,b.character_id::text AS buyer_character,s.character_id::text AS seller_character,least(b.remaining,s.remaining) AS amount,CASE WHEN (b.created_at,b.offer_id)<(s.created_at,s.offer_id) THEN b.price ELSE s.price END AS price FROM game_coin_market_offers b JOIN game_coin_market_offers s ON s.world_id=b.world_id AND s.side=2 AND s.state=1 AND s.account_id<>b.account_id AND s.price<=b.price WHERE b.world_id=$1::uuid AND b.side=1 AND b.state=1 AND b.expires_at>clock_timestamp() AND s.expires_at>clock_timestamp() AND NOT EXISTS(SELECT 1 FROM game_coin_operations p WHERE p.phase IN(0,2) AND (p.buy_offer=b.offer_id OR p.sell_offer=s.offer_id)) ORDER BY s.price,b.price DESC,least(b.created_at,s.created_at),b.offer_id,s.offer_id LIMIT 1 FOR UPDATE OF b,s").bind(world).fetch_optional(&mut *tx).await?;
    let Some(r) = pair else {
        return Ok(());
    };
    let buyer: String = r.try_get("buyer")?;
    let seller: String = r.try_get("seller")?;
    let buy: String = r.try_get("buy")?;
    let sell: String = r.try_get("sell")?;
    let actor: String = r.try_get("buyer_character")?;
    let operation: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut *tx)
        .await?;
    lock_accounts(tx, world, &[buyer.clone(), seller.clone()]).await?;
    let p = Plan {
        operation,
        world: world.into(),
        actor: actor.clone(),
        account: buyer.clone(),
        kind: "settle".into(),
        seller: Some(seller),
        buyer: Some(buyer),
        seller_character: Some(r.try_get("seller_character")?),
        buyer_character: Some(actor),
        buy_offer: Some(buy),
        sell_offer: Some(sell.clone()),
        reservation: Some(sell),
        parent: None,
        amount: r.try_get("amount")?,
        price: r.try_get("price")?,
        fee: 0,
        held: 0,
        binding: vec![0; 32],
        phase: 0,
    };
    prepare::insert(tx, &p).await
}
