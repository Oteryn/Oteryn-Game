//! Native library PG cases. Seeded open auctions are historical recovery DATA,
//! never a qualification or activation of D1383's absent catalogue producer.
#![allow(clippy::expect_used)]
use crate::bestiary_postgres_harness::{
    self as fixture, Harness, TestResult, configured_admin, debug, id, runtime,
};
use crate::durability::DurabilityRoot;
use crate::durability::house_ownership::{
    HouseBidRequest, HouseOwnershipError, HouseOwnershipOutcome, HouseSettlementRequest,
};
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration, WorldId};
use oteryn_gameplay_dispatch::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
use oteryn_gameplay_social::premium::PremiumConsumer;
use sqlx::Row;
use std::{future::Future, sync::Arc};

const HOUSE: &str = "oteryn:content.house.thais_1";

#[path = "house_ownership_tests/guild_participants.rs"]
mod guild_participants;
#[path = "house_ownership_tests/integrity.rs"]
mod integrity;

fn run<F, Fut>(tag: &'static str, body: F) -> TestResult
where
    F: FnOnce(Arc<Harness>) -> Fut,
    Fut: Future<Output = TestResult>,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    let rt = runtime()?;
    let h = Arc::new(rt.block_on(Harness::create(admin, tag, true))?);
    let captured = Arc::clone(&h);
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rt.block_on(body(captured))));
    let h = Arc::try_unwrap(h).map_err(|_| "House test retained its harness")?;
    rt.block_on(h.cleanup())?;
    match result {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

async fn runtime_root(h: &Harness) -> TestResult<DurabilityRoot> {
    let root = DurabilityRoot::connect_test_runtime(&format!(
        "{}?options=-c%20role%3Doteryn_game_runtime",
        h.database.url
    ))?;
    assert!(root.maintain_ready_once().await?);
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        version,
        oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM
    );
    eprintln!("HOUSE-1a PostgreSQL 18.6, native runtime role");
    Ok(root)
}

fn gate(root: &DurabilityRoot) -> SpellPremiumCoordinator {
    SpellPremiumCoordinator::new(
        Arc::new(PremiumConsumer::default()),
        None,
        None,
        Some(root.clone()),
    )
}
fn settlement_request() -> TestResult<HouseSettlementRequest> {
    Ok(HouseSettlementRequest {
        world: WorldId::decode(&id(fixture::WORLD)).map_err(debug)?,
        house_key: HOUSE.into(),
        auction: id(76),
        step: 0,
    })
}

fn request(n: u8, maximum: u64) -> TestResult<HouseBidRequest> {
    Ok(HouseBidRequest {
        operation_id: id(n),
        command: CommandRef::new(
            fixture::item_fence()?.game_session_id,
            CommandId::new(u64::from(n)).map_err(debug)?,
        ),
        house_key: HOUSE.into(),
        maximum,
    })
}
async fn seed(h: &Harness, active: bool) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest) VALUES(encode($1,'hex')::uuid,$2,'private_house',$3,100,$4)")
        .bind(id(fixture::WORLD).as_slice()).bind(HOUSE).bind(if active{2_i16}else{1_i16}).bind([7_u8;32].as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_ownership VALUES(encode($1,'hex')::uuid,$2,NULL,1,encode($3,'hex')::uuid)")
        .bind(id(fixture::WORLD).as_slice()).bind(HOUSE).bind(id(75).as_slice()).execute(&mut *tx).await?;
    if active {
        sqlx::query("INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at,next_order) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,100,clock_timestamp()+interval '1 day',2)")
            .bind(id(76).as_slice()).bind(id(fixture::WORLD).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_bids VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,500,100,1,1,600)")
            .bind(id(76).as_slice()).bind(id(40).as_slice()).bind(id(fixture::WORLD).as_slice()).bind(id(fixture::CHARACTER).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
async fn amounts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT coalesce((SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid),0),(SELECT escrow_gold FROM game_house_bids WHERE auction_id=encode($3,'hex')::uuid AND account_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_account_bank_entries WHERE house_operation_id IS NOT NULL)")
        .bind(id(40).as_slice()).bind(id(fixture::WORLD).as_slice()).bind(id(76).as_slice()).fetch_one(&h.pool).await?)
}

#[test]
fn house_ownership_recovered_bid_escrow_replay_conflict_and_session_fence() -> TestResult {
    run("house_ownership_bid", |h| async move {
        seed(&h, true).await?;
        let root = runtime_root(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let gate = gate(&root);
        let first = request(80, 300)?;
        assert_eq!(
            root.house_bid(
                &authority,
                &h.node,
                &gate,
                fixture::item_fence()?,
                first.clone()
            )
            .await
            .map_err(debug)?,
            HouseOwnershipOutcome::Applied
        );
        assert_eq!(amounts(&h).await?, (200, 400, 1));
        assert_eq!(
            root.house_bid(
                &authority,
                &h.node,
                &gate,
                fixture::item_fence()?,
                first.clone()
            )
            .await
            .map_err(debug)?,
            HouseOwnershipOutcome::Replayed
        );
        assert_eq!(amounts(&h).await?, (200, 400, 1));
        let mut conflict = first;
        let no_premium =
            SpellPremiumCoordinator::new(Arc::new(PremiumConsumer::default()), None, None, None);
        assert_eq!(
            root.house_bid(
                &authority,
                &h.node,
                &no_premium,
                fixture::item_fence()?,
                conflict.clone()
            )
            .await
            .map_err(debug)?,
            HouseOwnershipOutcome::Replayed
        );
        conflict.maximum = 301;
        assert!(matches!(
            root.house_bid(&authority, &h.node, &gate, fixture::item_fence()?, conflict)
                .await,
            Err(HouseOwnershipError::RequestConflict)
        ));
        root.house_bid(
            &authority,
            &h.node,
            &gate,
            fixture::item_fence()?,
            request(81, 400)?,
        )
        .await
        .map_err(debug)?;
        assert_eq!(amounts(&h).await?, (100, 500, 2));
        let mut stale = fixture::item_fence()?;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        assert!(matches!(
            root.house_bid(&authority, &h.node, &gate, stale, request(82, 450)?)
                .await,
            Err(HouseOwnershipError::AuthorityRejected)
        ));
        assert!(matches!(
            root.house_bid(
                &authority,
                &h.node,
                &gate,
                fixture::item_fence()?,
                request(83, 600)?
            )
            .await,
            Err(HouseOwnershipError::Value(_))
        ));
        assert_eq!(amounts(&h).await?, (100, 500, 2));
        // An identical maximum has a receipt but no ledger/value line.
        root.house_bid(
            &authority,
            &h.node,
            &gate,
            fixture::item_fence()?,
            request(84, 400)?,
        )
        .await
        .map_err(debug)?;
        assert_eq!(amounts(&h).await?, (100, 500, 2));
        let room:i64=sqlx::query_scalar("SELECT game_account_bank_credit_headroom(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
            .bind(id(40).as_slice()).bind(id(fixture::WORLD).as_slice()).fetch_one(&h.pool).await?;
        // GUILD-BANK-1's retained payouts use hard headroom; ordinary credits still
        // enforce BANK0-RL-01 separately. Liquid 100 + held House custody 500 remain reserved.
        assert_eq!(room, 9_000_000_000_000_000 - 600);
        Ok(())
    })
}

#[test]
fn house_ownership_first_rent_settlement_and_restart_replay() -> TestResult {
    run("house_ownership_winner", |h| async move {
        seed(&h, true).await?;
        let root = runtime_root(&h).await?;
        sqlx::query("UPDATE game_house_auctions SET ends_at=clock_timestamp()-interval '1 second' WHERE auction_id=encode($1,'hex')::uuid")
            .bind(id(76).as_slice()).execute(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let request = settlement_request()?;
        sqlx::query(
            "UPDATE game_house_properties SET rent_gold=200,revision=revision+1 WHERE house_key=$1",
        )
        .bind(HOUSE)
        .execute(&h.pool)
        .await?;
        assert_eq!(
            root.house_settlement(&authority, &h.node, &gate(&root), request.clone())
                .await
                .map_err(debug)?,
            HouseOwnershipOutcome::Applied
        );
        assert_eq!(amounts(&h).await?, (500, 0, 2));
        let row=sqlx::query("SELECT state,ownership_generation::text,acquisition_receipt_ref::text,paid_until>clock_timestamp()+interval '29 days' AS paid FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
            .bind(id(fixture::WORLD).as_slice()).bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(row.try_get::<i16, _>("state")?, 3);
        assert_eq!(row.try_get::<String, _>("ownership_generation")?, "2");
        assert!(row.try_get::<bool, _>("paid")?);
        assert_eq!(h.count("game_account_housing_slots").await?, 1);
        assert_eq!(h.count("game_house_audit_outbox").await?, 1);
        let access:(String,String)=sqlx::query_as("SELECT role,acl_revision::text FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)")
            .bind(id(fixture::WORLD).as_slice()).bind(HOUSE).bind(id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(access, ("OWNER".into(), "1".into()));
        let restarted = runtime_root(&h).await?;
        let reopened = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            restarted
                .house_settlement(&reopened, &h.node, &gate(&restarted), request)
                .await
                .map_err(debug)?,
            HouseOwnershipOutcome::Replayed
        );
        assert_eq!(amounts(&h).await?, (500, 0, 2));
        Ok(())
    })
}

#[test]
fn house_ownership_catalogue_and_premium_fail_closed() -> TestResult {
    run("house_ownership_gates", |h| async move {
        seed(&h, false).await?;
        let root = runtime_root(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        assert!(matches!(
            root.house_bid(
                &authority,
                &h.node,
                &gate(&root),
                fixture::item_fence()?,
                request(80, 300)?
            )
            .await,
            Err(HouseOwnershipError::CatalogueUnavailable)
        ));
        let unknown =
            SpellPremiumCoordinator::new(Arc::new(PremiumConsumer::default()), None, None, None);
        let mut held = h.pool.begin().await?;
        sqlx::query("SELECT 1 FROM game_house_properties WHERE house_key=$1 FOR UPDATE")
            .bind(HOUSE)
            .execute(&mut *held)
            .await?;
        assert!(matches!(
            root.house_bid(
                &authority,
                &h.node,
                &unknown,
                fixture::item_fence()?,
                request(81, 300)?
            )
            .await,
            Err(HouseOwnershipError::NotPremium)
        ));
        held.rollback().await?;
        assert_eq!(h.count("game_house_operations").await?, 0);
        assert_eq!(h.count("game_account_bank_entries").await?, 0);
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let denied = sqlx::query("UPDATE game_house_properties SET revision=revision+1")
            .execute(&mut *tx)
            .await
            .expect_err("runtime direct property write must be denied");
        assert_eq!(
            denied.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("42501")
        );
        tx.rollback().await?;
        Ok(())
    })
}
