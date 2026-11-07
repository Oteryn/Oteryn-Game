//! Migration 0155 + native PostgreSQL reads/locks. Administrative fixtures do not
//! admit content, create current authority or qualify the missing value writer.
#![allow(clippy::expect_used)]
use crate::bestiary_postgres_harness as harness;
use crate::domain::{AccountId, CharacterId, WorldId};
use crate::durability::market::*;
use harness::{CHARACTER, Harness, TestResult, WORLD, id};
use sqlx::Executor;

pub(super) fn world() -> WorldId {
    WorldId::from_bytes(id(WORLD)).expect("world")
}
pub(super) fn offer(seed: u8, side: Side, account: u8, created: i64) -> Offer {
    Offer {
        id: MarketOfferId::new(id(seed)).expect("offer"),
        world: world(),
        definition: definition("oteryn:content.item.vase", "r1").expect("definition"),
        side,
        account: AccountId::from_bytes(id(account)).expect("account"),
        character: CharacterId::from_bytes(id(account + 1)).expect("character"),
        piece_price: 100,
        amount: 10,
        remaining: 10,
        anonymous: false,
        created_at_us: created,
        expires_at_us: created + LIFETIME_US,
        state: OfferState::Open,
        escrow_gold: if side == Side::Buy { 1000 } else { 0 },
        held_credit_gold: 0,
        matching_pending: false,
    }
}
pub(super) fn definition(
    key: &str,
    revision: &str,
) -> Result<StoredItemDefinition, crate::durability::DurabilityError> {
    StoredItemDefinition::new(crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".to_owned(),
        production_key: key.to_owned(),
        revision_ref: revision.to_owned(),
    })
}
pub(super) fn price(o: &mut Offer, p: u64) {
    o.piece_price = p;
    o.escrow_gold = if o.side == Side::Buy && o.state == OfferState::Open {
        u64::from(o.remaining) * p
    } else {
        0
    };
}
pub(super) fn run<F>(tag: &str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, false).await?;
        let result = body(&h).await;
        h.cleanup().await?;
        result
    })
}
pub(super) async fn now(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&h.pool)
            .await?,
    )
}
pub(super) async fn seed(h: &Harness, o: &Offer) -> TestResult {
    // Administrative storage fixtures only. No live session/root capability is inferred.
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING").bind(o.account.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,$4 FROM game_character_roots WHERE character_id=encode($5,'hex')::uuid ON CONFLICT(character_id) DO NOTHING")
        .bind(o.character.as_bytes().as_slice()).bind(o.account.as_bytes().as_slice()).bind(o.world.as_bytes().as_slice()).bind(format!("Market Fixture {}",char::from(b'A'+o.account.as_bytes()[0]/10))).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    sqlx::query(
        "INSERT INTO game_market_books VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT DO NOTHING",
    )
    .bind(o.world.as_bytes().as_slice())
    .bind(o.definition.key())
    .execute(&h.pool)
    .await?;
    let (state, ended) = match o.state {
        OfferState::Open => (1_i16, None),
        OfferState::Ended(e) => (end_code(e), None),
        OfferState::CreditHeld(e) => (5, Some(end_code(e))),
    };
    sqlx::query("INSERT INTO game_market_offers(offer_id,world_id,definition_key,definition_revision,side,account_id,character_id,piece_price,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold,held_credit_gold,matching_pending,ended_at_us,end_sequence) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric,$9,$10,$11,$12,$13,$14,$15,$16::text::numeric,$17::text::numeric,$18,CASE WHEN $14<>1 THEN $12+1 ELSE NULL END,CASE WHEN $14<>1 THEN nextval('game_market_end_sequence') ELSE NULL END)")
        .bind(o.id.as_bytes().as_slice()).bind(o.world.as_bytes().as_slice()).bind(o.definition.key()).bind(o.definition.revision()).bind(if o.side==Side::Buy {1_i16}else{2}).bind(o.account.as_bytes().as_slice()).bind(o.character.as_bytes().as_slice()).bind(o.piece_price.to_string()).bind(i32::try_from(o.amount)?).bind(i32::try_from(o.remaining)?).bind(o.anonymous).bind(o.created_at_us).bind(o.expires_at_us).bind(state).bind(ended).bind(o.escrow_gold.to_string()).bind(o.held_credit_gold.to_string()).bind(o.matching_pending).execute(&h.pool).await?;
    Ok(())
}
// Administrative metadata injection, never an implementation of the ending writer.
pub(super) async fn seed_ending(
    h: &Harness,
    o: &Offer,
    ended_at_us: i64,
    sequence: i64,
) -> TestResult {
    seed(h, o).await?;
    sqlx::query("UPDATE game_market_offers SET ended_at_us=$2,end_sequence=$3 WHERE offer_id=encode($1,'hex')::uuid")
        .bind(o.id.as_bytes().as_slice()).bind(ended_at_us).bind(sequence)
        .execute(&h.pool).await?;
    Ok(())
}

pub(super) fn end_code(e: EndState) -> i16 {
    match e {
        EndState::Filled => 2,
        EndState::Cancelled => 3,
        EndState::Expired => 4,
    }
}

pub(super) async fn seed_escrow(h: &Harness, o: &Offer, count: i32) -> TestResult {
    // Historical admin item fixtures, following native Inbox tests. Disabled native MINT/
    // placement guards are confined to fixture loading; no such bypass exists in the component.
    // This does NOT qualify Market custody proofs, admitted content or conservation guards.
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL session_replication_role=replica")
        .await?;
    sqlx::query("WITH items AS(INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,$3,1,1,game_character_uuid_v7() FROM generate_series(1,$4) RETURNING item_instance_id,world_id) INSERT INTO game_item_market_escrow_locations SELECT item_instance_id,world_id,encode($5,'hex')::uuid,row_number() OVER(ORDER BY item_instance_id) FROM items")
        .bind(o.world.as_bytes().as_slice()).bind(o.definition.key()).bind(o.definition.revision()).bind(count).bind(o.id.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
