use crate::bestiary_postgres_harness as fixture;
use crate::durability::DurabilityRoot;
use crate::durability::house_management::{HouseManagementPage, QualifiedHouse};
use crate::durability::house_ownership::{
    HouseBidRequest, HouseCatalogueActivation, HouseOwnershipError, HouseOwnershipOutcome,
};
use crate::foundation::{ChannelId, CommandId, CommandRef, WorldId};
use oteryn_gameplay_dispatch::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
use oteryn_gameplay_housing::house_auction::AuctionError;
use oteryn_gameplay_social::premium::PremiumConsumer;
use std::sync::Arc;

#[test]
fn house_catalogue_activation_registers_vacant_property_and_opens_on_first_bid()
-> fixture::TestResult {
    let admin = match std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") {
        Ok(admin) => admin,
        Err(std::env::VarError::NotPresent) => {
            eprintln!("Skipping house catalogue PostgreSQL case: OTERYN_TEST_POSTGRES_ADMIN_URL is absent");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime().map_err(fixture::debug)?.block_on(async move {
                let harness = fixture::Harness::create(admin, "house_catalogue_activation", true).await?;
                let result = exercise(&harness).await;
                harness.cleanup().await?;
                result
            }).map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "house catalogue activation test panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    use fixture::{debug, id};
    const HOUSE: &str = "oteryn:content.house.catalogue_test";
    const DIGEST: [u8; 32] = [7; 32];
    let world = WorldId::decode(&id(fixture::WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(fixture::CHANNEL)).map_err(debug)?;
    sqlx::query("INSERT INTO game_content_activations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$3,$3,'test',0)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .bind(DIGEST.as_slice()).execute(&h.pool).await?;
    let root = DurabilityRoot::connect_test_runtime(&format!(
        "{}?options=-c%20role%3Doteryn_game_runtime", h.database.url
    ))?;
    assert!(root.maintain_ready_once().await?);
    let catalogue = |generation| HouseCatalogueActivation {
        world, channel, generation, sequence: 1, digest: DIGEST,
        entries: vec![serde_json::json!({"house_key":HOUSE,"kind":"private_house","rent_gold":100})],
    };
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    let fence = fixture::item_fence()?;
    let listing = || {
        root.read_house_management(
            &authority,
            &h.node,
            fence,
            HouseManagementPage {
                houses: vec![QualifiedHouse {
                    key: HOUSE.into(),
                    kind: "private_house",
                }],
                cursor: None,
                auction_order: false,
            },
            DIGEST,
        )
    };
    assert!(listing().await.map_err(debug)?.is_empty());
    let stale = root.activate_house_catalogue(&h.node, catalogue(2)).await;
    assert!(
        matches!(stale, Err(HouseOwnershipError::AuthorityRejected)),
        "{stale:?}"
    );
    assert_eq!(h.count("game_house_properties").await?, 0);
    for _ in 0..2 {
        root.activate_house_catalogue(&h.node, catalogue(1)).await.map_err(debug)?;
    }
    let vacant: (i16, i64, String, bool) = sqlx::query_as(
        "SELECT p.state,p.rent_gold,p.revision::text,a.owner_character_id IS NULL
         FROM game_house_properties p JOIN game_house_ownership a USING(world_id,house_key)
         WHERE p.house_key=$1 AND p.content_digest=$2 AND p.ownership_generation=a.ownership_revision",
    ).bind(HOUSE).bind(DIGEST.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(vacant, (1, 100, "1".into(), true));
    assert_eq!(h.count("game_house_auctions").await?, 0);

    let rows = listing().await.map_err(debug)?;
    let [row] = rows.as_slice() else {
        return Err("live town query must return exactly one activated house".into());
    };
    assert_eq!((row.house.as_str(), row.state, row.rent, row.ends_ms), (HOUSE, 1, 100, None));
    let gate = SpellPremiumCoordinator::new(Arc::new(PremiumConsumer::default()), None, None, Some(root.clone()));
    let request = |command, maximum| -> fixture::TestResult<HouseBidRequest> {
        Ok(HouseBidRequest {
            operation_id: id(command),
            command: CommandRef::new(fence.game_session_id, CommandId::new(u64::from(command)).map_err(debug)?),
            house_key: HOUSE.into(), maximum, content_digest: DIGEST, kind: "private_house", catalogue_rent: 100,
        })
    };
    let unfunded = root
        .house_bid(&authority, &h.node, &gate, fence, request(80, 701)?)
        .await;
    assert!(
        matches!(
            unfunded,
            Err(HouseOwnershipError::Value(AuctionError::Unfunded))
        ),
        "{unfunded:?}"
    );
    assert_eq!(h.count("game_house_auctions").await?, 0);
    fund_account(h).await?;
    let clock = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT (extract(epoch FROM clock_timestamp())*1000000)::bigint",
        )
        .fetch_one(&h.pool)
    };
    let first = request(81, 300)?;
    let before = clock().await?;
    assert_eq!(
        root.house_bid(&authority, &h.node, &gate, fence, first.clone())
            .await
            .map_err(debug)?,
        HouseOwnershipOutcome::Applied
    );
    let after = clock().await?;
    assert_eq!(
        root.house_bid(&authority, &h.node, &gate, fence, first)
            .await
            .map_err(debug)?,
        HouseOwnershipOutcome::Replayed
    );
    root.activate_house_catalogue(&h.node, catalogue(1))
        .await
        .map_err(debug)?;
    let opened: (i16, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT p.state,a.first_rent,b.escrow_gold,(extract(epoch FROM a.ends_at)*1000000)::bigint,(extract(epoch FROM a.ends_at)*1000)::bigint
         FROM game_house_properties p JOIN game_house_auctions a USING(world_id,house_key)
         JOIN game_house_bids b USING(auction_id) WHERE p.house_key=$1",
    ).bind(HOUSE).fetch_one(&h.pool).await?;
    assert_eq!((opened.0, opened.1, opened.2), (2, 100, 400));
    const SEVEN_DAYS_US: i64 = 7 * 24 * 60 * 60 * 1_000_000;
    let expected = before + SEVEN_DAYS_US..=after + SEVEN_DAYS_US;
    assert!(
        expected.contains(&opened.3),
        "deadline {} outside {expected:?}",
        opened.3
    );
    assert_eq!(h.count("game_house_auctions").await?, 1);
    let rows = listing().await.map_err(debug)?;
    let [row] = rows.as_slice() else {
        return Err("live town query must return exactly one auction".into());
    };
    assert_eq!(
        (row.house.as_str(), row.state, row.rent, row.own_maximum),
        (HOUSE, 2, 100, Some(300))
    );
    assert_eq!(row.ends_ms, Some(u64::try_from(opened.4)?));
    Ok(())
}

async fn fund_account(h: &fixture::Harness) -> fixture::TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx).await?;
    sqlx::query(
        "WITH ids AS (
           SELECT game_character_uuid_v7() AS occurrence, game_character_uuid_v7() AS operation,
                  game_character_uuid_v7() AS event, game_character_uuid_v7() AS platinum,
                  game_character_uuid_v7() AS gold, game_character_uuid_v7() AS backpack,
                  game_character_uuid_v7() AS entry
         ), deposit AS (
           INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,
             request_binding,acting_character_id,account_id,world_id,channel_id,
             runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,
             planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id)
           SELECT occurrence,operation,event,1,sha256('test'),encode($1,'hex')::uuid,encode($2,'hex')::uuid,
             encode($3,'hex')::uuid,encode($4,'hex')::uuid,
             1,600,0,1,sha256('test'),0,platinum,gold,backpack FROM ids
           RETURNING transaction_id,account_id,world_id,acting_character_id
         ), credit AS (
           INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,
             amount,balance_before,balance_after,acting_character_id)
           SELECT entry,transaction_id,account_id,world_id,1,600,0,600,acting_character_id
           FROM deposit CROSS JOIN ids RETURNING entry_id,account_id,world_id
         )
         INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id)
         SELECT account_id,world_id,600,entry_id FROM credit",
    )
    .bind(fixture::id(fixture::CHARACTER).as_slice())
    .bind(fixture::id(40).as_slice())
    .bind(fixture::id(fixture::WORLD).as_slice())
    .bind(fixture::id(fixture::CHANNEL).as_slice())
    .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
