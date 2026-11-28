//! Qualified vacant property -> real bid command -> scheduled settlement owner.
use super::*;
use crate::durability::house_ownership::{
    HouseAuctionCatalogue, HouseAuctionProperty, HouseOwnershipOutcome,
};
use crate::foundation::{ChannelId, CharacterId, CharacterLease, RuntimeScopeRefV1, WorldId};
use crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
use sqlx::ConnectOptions;

#[allow(dead_code, reason = "shared producer supports additional Premium fixtures")]
#[path = "../../../../../social/src/premium/test_producer.rs"]
mod premium_test_producer;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AuctionCase {
    Opening,
    OfflinePremium,
    ActivationProof,
    FullCensus,
    HardCeiling,
    HardCeilingRefund,
    VacuumLock,
    BusyTile,
    ForegroundPreference,
    ReapplyMigration,
}

#[test]
fn house_auction_postgres_first_bid_opens_and_due_winner_owns() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, AuctionCase::Opening)
}

#[test]
fn house_auction_offline_premium_bidder_wins() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, AuctionCase::OfflinePremium)
}

#[test]
fn house_auction_defers_without_activation_and_clock_proof() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, AuctionCase::ActivationProof)
}

#[test]
fn house_auction_full_census_preserves_foreground_bid_budget() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, AuctionCase::FullCensus)
}

#[test]
fn house_auction_bid_and_award_above_soft_ceiling() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, AuctionCase::HardCeiling)
}

#[test]
fn house_auction_refunds_above_soft_ceiling() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, AuctionCase::HardCeilingRefund)
}

#[test]
fn house_auction_opens_during_ground_maintenance() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, AuctionCase::VacuumLock)
}

#[test]
fn house_auction_busy_tile_keeps_connection_open() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, AuctionCase::BusyTile)
}

#[test]
fn house_auction_background_work_yields_to_foreground() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, AuctionCase::ForegroundPreference)
}

#[test]
fn house_auction_migration_reapplies() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, AuctionCase::ReapplyMigration)
}

fn run_case(admin: String, case: AuctionCase) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "house auction opening PG thread panicked")??;
    Ok(())
}

fn scenario(admin: String, case: AuctionCase) -> TestResult {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(Box::pin(async move {
        reset();
        let tag = match case {
            AuctionCase::Opening => "house_auction_opening",
            AuctionCase::OfflinePremium => "house_auction_offline",
            AuctionCase::ActivationProof => "house_auction_activation",
            AuctionCase::FullCensus => "house_auction_census",
            AuctionCase::HardCeiling => "house_auction_hard_cap",
            AuctionCase::HardCeilingRefund => "house_auction_hard_refund",
            AuctionCase::VacuumLock => "house_auction_vacuum",
            AuctionCase::BusyTile => "house_auction_busy_tile",
            AuctionCase::ForegroundPreference => "house_auction_foreground",
            AuctionCase::ReapplyMigration => "house_auction_reapply",
        };
        let mut h =
            Harness::create_with_admission_seed(admin, tag, true, false)
                .await?;
        if case == AuctionCase::ReapplyMigration {
            sqlx::raw_sql(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"), "/../../../apps/game-server/migrations/3356_house_auction_opening.sql"
            ))).execute(&h.pool).await?;
            return h.cleanup().await;
        }
        prepare_current_session(&h).await?;
        let runtime_options = h
            .pool
            .connect_options()
            .as_ref()
            .clone()
            .options([("role", "oteryn_game_runtime")]);
        h.root = crate::durability::DurabilityRoot::connect_test_runtime(
            runtime_options.to_url_lossy().as_str(),
        )?;
        assert!(h.root.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let gate = SpellPremiumCoordinator::new(
            std::sync::Arc::default(),
            None,
            None,
            Some(h.root.clone()),
        );
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = Some(&gate);
        deps.initialize(&owner).await;
        let digest = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    deps.admitted.game_session_id,
                    CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                        .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime.content_pin().server_artifact_digest()
        };
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(current);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 49])
                .ok_or("House selection")?;

        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let scope = RuntimeScopeRefV1::channel(world, channel);
        let bundle = super::super::house_wire_bundle::house_bundle(world, channel)?;
        let map = bundle.map_revision();
        let generation = h
            .root
            .read_runtime_scope_assignment(scope)
            .await
            .map_err(debug)?
            .ok_or("Channel assignment")?
            .ownership_generation;
        sqlx::query("INSERT INTO game_world_reset_epochs VALUES(encode($1,'hex')::uuid,0,$2,$3,$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-11')")
            .bind(id(42).as_slice()).bind(digest.as_slice()).bind(map).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
            .bind(id(42).as_slice())
            .execute(&h.pool)
            .await?;
        let rent = i64::try_from(crate::gameplay_transport::house_query::source_house_rent(HOUSE).ok_or("catalogue rent")?)?;
        let excess = if matches!(case, AuctionCase::HardCeiling | AuctionCase::HardCeilingRefund) { 1_000_000_000_000 } else { 0 };
        let funding = rent + 900 + excess;
        sqlx::query("INSERT INTO game_public_house_catalogue VALUES(encode($1,'hex')::uuid,$2,'Admiral''s Avenue 1','private_house','liberty_bay',1,0,$3,$4)")
            .bind(id(42).as_slice()).bind(HOUSE).bind(rent).bind(digest.as_slice()).execute(&h.pool).await?;

        let catalogue = HouseAuctionCatalogue {
            scope,
            generation,
            reset_epoch: 0,
            map_revision: map,
            content_digest: digest,
        };
        let tiles: Vec<_> = bundle
            .facts()
            .house_tiles(3437)
            .map(|tile| [i32::from(tile.x), i32::from(tile.y), i32::from(tile.floor)])
            .collect();
        let property = || HouseAuctionProperty {
            key: HOUSE,
            kind: "private_house",
            rent: rent as u64,
            tiles: &tiles,
        };
        if case == AuctionCase::ForegroundPreference {
            use crate::durability::{DurabilityError, house_ownership::HouseOwnershipError};
            let demand = h.root.foreground_demand()?;
            let deferred = h.root.qualify_house_auction(&authority, &h.node, &catalogue, property()).await;
            assert!(matches!(deferred, Err(HouseOwnershipError::Unavailable(DurabilityError::RootUnavailable))));
            assert!(matches!(h.root.due_house_auctions(world, None).await,
                Err(HouseOwnershipError::Unavailable(DurabilityError::RootUnavailable))));
            let request = crate::durability::house_ownership::HouseSettlementRequest {
                world, house_key: HOUSE.into(), auction: id(76), step: 0,
            };
            assert!(matches!(h.root.house_settlement(&authority, &h.node, request, crate::durability::house_ownership::HouseSettlementPremium::Refund).await,
                Err(HouseOwnershipError::Unavailable(DurabilityError::RootUnavailable))));
            drop(demand);
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let stale = HouseAuctionCatalogue {
            generation: generation + 1,
            ..catalogue
        };
        assert!(
            !h.root
                .qualify_house_auction(&authority, &h.node, &stale, property())
                .await
                .map_err(debug)?
        );
        assert!(
            h.root
                .qualify_house_auction(&authority, &h.node, &catalogue, property())
                .await
                .map_err(debug)?
        );
        assert!(
            h.root
                .qualify_house_auction(&authority, &h.node, &catalogue, property())
                .await
                .map_err(debug)?
        );
        assert_eq!(public_status(&h).await?, "VACANT");
        let vacant = wire::HouseSummary {
            details: None,
            house: HOUSE.into(),
            kind: wire::HouseKind::PrivateHouse,
            state: 1,
            rent: u64::try_from(rent)?,
            revision: 1,
            acl_revision: 1,
            owned: false,
            move_out_ms: None,
            price: 0,
            ends_ms: None,
            own_maximum: None,
        };
        assert_eq!(
            send(&owner, &mut admitted, 57, &query()).await?,
            (
                CommandStatus::Accepted,
                wire::encode_house_page(&[vacant.clone()], None, false).map_err(debug)?
            )
        );
        let tile_bytes = crate::durability::map_item_mint_audit::map_item_spatial_position(
            tiles[0][0].try_into()?,
            tiles[0][1].try_into()?,
            tiles[0][2].try_into()?,
        );
        assert!(
            !sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM game_house_ownership WHERE house_key=$1)"
            )
            .bind(HOUSE)
            .fetch_one(&h.pool)
            .await?,
            "a listing does not create a World-reset custody participant"
        );

        let mut amount = Vec::new();
        scalar(&mut amount, 1, 200);
        assert_eq!(
            send(&owner, &mut admitted, 56, &intent(1, &amount))
                .await?
                .1,
            wire::encode_house_result(wire::HouseDisposition::Rejected, 0)
        );
        assert_eq!(public_status(&h).await?, "VACANT");
        assert!(
            !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM game_house_auctions)")
                .fetch_one(&h.pool)
                .await?
        );

        super::super::guild_bank_fixtures::bank_seed::balance(&h, id(40), id(41), funding, 210)
            .await?;
        let mut amount = Vec::new();
        scalar(&mut amount, 1, 200);
        let bid = intent(1, &amount);
        if case == AuctionCase::BusyTile {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended(
                encode($1,'hex')::uuid::text||':'||encode($2,'hex'),3356))")
                .bind(world.as_bytes().as_slice()).bind(&tile_bytes).execute(&mut *tx).await?;
            assert_eq!(
                send(&owner, &mut admitted, 56, &bid).await?,
                (CommandStatus::Accepted, wire::encode_house_result(wire::HouseDisposition::Rejected, 0)),
                "a proven pre-commit lock failure keeps the session usable after exhausting the retry budget"
            );
            tx.rollback().await?;
            assert_eq!(public_status(&h).await?, "VACANT");
        }
        let command = admitted.continuity.next_command_id;
        let mut maintenance = if case == AuctionCase::VacuumLock {
            let mut tx = h.pool.begin().await?;
            sqlx::query("LOCK TABLE game_item_ground_locations IN SHARE UPDATE EXCLUSIVE MODE")
                .execute(&mut *tx).await?;
            Some(tx)
        } else {
            None
        };
        let earliest = clock(&h).await?;
        let ok = wire::encode_house_result(wire::HouseDisposition::Ok, 0);
        assert_eq!(
            send(&owner, &mut admitted, 56, &bid).await?,
            (CommandStatus::Accepted, ok.clone()),
            "the first funded bid opens a qualified vacant house"
        );
        if let Some(tx) = maintenance.take() {
            tx.rollback().await?;
        }
        assert_eq!(
            resend(&owner, &mut admitted, command, 56, &bid).await?,
            (CommandStatus::DuplicateReplay, ok)
        );
        let registered: Vec<u8> =
            sqlx::query_scalar("SELECT spatial_position FROM game_house_tiles WHERE house_key=$1")
                .bind(HOUSE)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(registered, tile_bytes);
        assert!(
            sqlx::query_scalar::<_, bool>(
                "SELECT game_house_access_tile(encode($1,'hex')::uuid,$2,$3)"
            )
            .bind(id(42).as_slice())
            .bind(HOUSE)
            .bind(&tile_bytes)
            .fetch_one(&h.pool)
            .await?
        );
        let ground = sqlx::query(
            "INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,
              runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,
              content_revision,native_room_placement_context)
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,
              1,$4,$4,$5,$5,$4)",
        )
        .bind(id(225).as_slice())
        .bind(id(42).as_slice())
        .bind(id(43).as_slice())
        .bind(&tile_bytes)
        .bind(map)
        .execute(&h.pool)
        .await;
        assert!(matches!(ground, Err(sqlx::Error::Database(error))
            if error.message()=="Ground location is a World-global House tile"));

        let latest = clock(&h).await?;
        let opened: (i64,i64,i64,i64) = sqlx::query_as(
            "SELECT a.first_rent,b.escrow_gold,v.balance,(extract(epoch FROM a.ends_at)*1000000)::bigint
             FROM game_house_auctions a JOIN game_house_bids b USING(auction_id)
             JOIN game_house_audit_outbox o ON o.operation_id=a.auction_id
             JOIN game_account_bank_balances v ON (v.world_id,v.account_id)=(b.world_id,b.account_id)"
        ).fetch_one(&h.pool).await?;
        assert_eq!((opened.0, opened.1, opened.2), (rent, rent + 200, 700 + excess));
        let duration = oteryn_gameplay_housing::house_auction::OPENING_US;
        assert!((earliest + duration..=latest + duration).contains(&opened.3));
        assert_eq!(public_status(&h).await?, "AUCTION");
        let ends_ms: i64 = sqlx::query_scalar(
            "SELECT (extract(epoch FROM ends_at)*1000)::bigint FROM game_house_auctions",
        )
        .fetch_one(&h.pool)
        .await?;
        let active = wire::HouseSummary {
            state: 2,
            revision: 2,
            ends_ms: Some(u64::try_from(ends_ms)?),
            own_maximum: Some(200),
            ..vacant.clone()
        };
        assert_eq!(
            send(&owner, &mut admitted, 57, &query()).await?,
            (
                CommandStatus::Accepted,
                wire::encode_house_page(&[active], None, false).map_err(debug)?
            )
        );
        assert!(
            h.root
                .due_house_auctions(world, None)
                .await
                .map_err(debug)?
                .is_empty()
        );

        if case == AuctionCase::FullCensus {
            let request = seed_full_census(&h, world, &digest).await?;
            let mut amount = Vec::new();
            scalar(&mut amount, 1, 250);
            let raise = intent(1, &amount);
            let started = tokio::time::Instant::now();
            let (award, raise_result) = tokio::join!(biased;
                gate.settle_house_auction(&h.root, &authority, &h.node, None, request.clone()),
                send(&owner, &mut admitted, 56, &raise)
            );
            assert_eq!(raise_result?, (CommandStatus::Accepted, wire::encode_house_result(wire::HouseDisposition::Ok, 0)));
            let award = match award {
                Err(crate::durability::house_ownership::HouseOwnershipError::Unavailable(
                    crate::durability::DurabilityError::RootUnavailable,
                ) | crate::durability::house_ownership::HouseOwnershipError::NotPremium) =>
                    gate.settle_house_auction(&h.root, &authority, &h.node, None, request).await,
                other => other,
            };
            assert_eq!(award.map_err(debug)?, HouseOwnershipOutcome::Applied);
            assert!(started.elapsed() < std::time::Duration::from_secs(1));
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }

        sqlx::query("UPDATE game_house_auctions SET ends_at=clock_timestamp()-interval '1 second'")
            .execute(&h.pool)
            .await?;
        use crate::gameplay_transport::spell_premium_coordinator::PremiumActivation;
        let now = clock(&h).await?;
        let activation = PremiumActivation {
            activation_id: "house-auction-r2".into(),
            switch_over_us: now - 10_000_000,
        };
        let delivered = SpellPremiumCoordinator::new(
            std::sync::Arc::default(),
            Some(std::sync::Arc::new(AuctionClock(Some(now)))),
            Some(activation.clone()),
            Some(h.root.clone()),
        );
        let platform = if matches!(case, AuctionCase::OfflinePremium | AuctionCase::HardCeilingRefund | AuctionCase::ActivationProof) {
            let paid = case != AuctionCase::HardCeilingRefund;
            Some(premium_platform(&h, paid, case == AuctionCase::OfflinePremium).await?)
        } else {
            None
        };
        if case == AuctionCase::OfflinePremium {
            h.root.latch_premium_activation(&activation.activation_id, activation.switch_over_us, now).await?;
            let fresh = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
            let (session, _) = fresh.current_session_at(admitted.game_session_id).await?;
            fresh.release_logout_session(
                admitted.game_session_id,
                &crate::premium::snapshot::canonical_uuid(id(40)),
                session.current_transport().ok_or("connected bidder")?,
            ).await?;
            let (offline, _) = fresh.current_session_at(admitted.game_session_id).await?;
            assert_eq!(offline.session_state(), crate::foundation::GameSessionState::Terminal);
            assert!(sqlx::query_scalar::<_, bool>("SELECT refresh_after_us < (extract(epoch FROM clock_timestamp()-interval '40 minutes')*1000000)::bigint
                AND effective_until_us > (extract(epoch FROM clock_timestamp())*1000000)::bigint
                FROM game_premium_evidence WHERE authority_revision=1").fetch_one(&h.pool).await?);
        }
        let mut due = h
            .root
            .due_house_auctions(world, None)
            .await
            .map_err(debug)?;
        assert_eq!(due.len(), 1);
        let winner = due.pop().ok_or("due auction")?;
        let client = platform.as_ref().map(|(_, client)| client);
        if case == AuctionCase::ActivationProof {
            use crate::premium::client::fresh_nonce;
            let (producer, client) = platform.as_ref().ok_or("test Platform")?;
            let nonce = fresh_nonce().ok_or("test Premium nonce")?;
            let body = client.pull(id(40), &nonce).await.map_err(debug)?;
            let cached = crate::premium::snapshot::validate(&body, id(40), &nonce).map_err(debug)?;
            assert!(cached.refresh_after_us > now);
            h.root.accept_premium_evidence(&cached).await?;
            producer.script(|_| premium_test_producer::Reply::status(503));
            assert!(delivered.settle_house_auction(&h.root, &authority, &h.node, Some(client), winner.clone()).await.is_err());
            assert_eq!(producer.requests().len(), 2);
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_premium_activation_latch").fetch_one(&h.pool).await?, 1);
            let no_clock = SpellPremiumCoordinator::new(std::sync::Arc::default(), None, Some(activation.clone()), Some(h.root.clone()));
            let wrong_activation = PremiumActivation { activation_id: "different-activation".into(), ..activation };
            let mismatch = SpellPremiumCoordinator::new(std::sync::Arc::default(),
                Some(std::sync::Arc::new(AuctionClock(Some(now)))), Some(wrong_activation), Some(h.root.clone()));
            let pull_count = producer.requests().len();
            for coordinator in [&no_clock, &mismatch] {
                assert!(coordinator.settle_house_auction(&h.root, &authority, &h.node, Some(client), winner.clone()).await.is_err());
            }
            assert_eq!(producer.requests().len(), pull_count);
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_settlement_steps").fetch_one(&h.pool).await?, 0);
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_account_housing_slots").fetch_one(&h.pool).await?, 0);
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let settlement_gate = if platform.is_some() { &delivered } else { &gate };
        assert_eq!(
            settlement_gate.settle_house_auction(&h.root, &authority, &h.node, client, winner.clone())
                .await.map_err(debug)?, HouseOwnershipOutcome::Applied
        );
        assert_eq!(
            settlement_gate.settle_house_auction(&h.root, &authority, &h.node, None, winner)
                .await.map_err(debug)?, HouseOwnershipOutcome::Replayed
        );
        if case == AuctionCase::OfflinePremium {
            assert_eq!(platform.as_ref().ok_or("test Platform")?.0.requests().len(), 2);
            assert_eq!(public_status(&h).await?, "OWNED");
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if case == AuctionCase::HardCeilingRefund {
            let refunded: (i16, i64, i64) = sqlx::query_as(
                "SELECT b.state,b.escrow_gold,v.balance FROM game_house_bids b
                 JOIN game_account_bank_balances v USING(world_id,account_id)"
            ).fetch_one(&h.pool).await?;
            assert_eq!(refunded, (3, 0, funding));
            assert_eq!(public_status(&h).await?, "VACANT");
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let owned: (Vec<u8>,i64,i16,i64,i64) = sqlx::query_as(
            "SELECT uuid_send(game_house_effective_owner(p.world_id,p.house_key)),p.ownership_generation::bigint,b.state,b.escrow_gold,v.balance
             FROM game_house_properties p JOIN game_house_auctions a USING(world_id,house_key)
             JOIN game_house_bids b USING(auction_id)
             JOIN game_account_bank_balances v ON (v.world_id,v.account_id)=(b.world_id,b.account_id)
             JOIN game_account_housing_slots s ON (s.world_id,s.house_key,s.account_id)=(p.world_id,p.house_key,p.owner_account_id)"
        ).fetch_one(&h.pool).await?;
        assert_eq!(owned, (id(41).to_vec(), 2, 2, 0, 900 + excess));
        assert!(
            h.root
                .due_house_auctions(world, None)
                .await
                .map_err(debug)?
                .is_empty()
        );
        assert_eq!(public_status(&h).await?, "OWNED");
        let without_receipt = sqlx::query(
            "INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at,settled,requires_opening_bid)
             SELECT game_character_uuid_v7(),world_id,house_key,first_rent,clock_timestamp(),true,true
             FROM game_house_auctions WHERE house_key=$1"
        ).bind(HOUSE).execute(&h.pool).await;
        assert!(matches!(without_receipt, Err(sqlx::Error::Database(error))
            if error.message()=="HOUSE_AUCTION_WITHOUT_OPENING_BID"));
        let owned = wire::HouseSummary {
            state: 3,
            revision: 3,
            owned: true,
            ..vacant
        };
        assert_eq!(
            send(&owner, &mut admitted, 57, &query()).await?,
            (
                CommandStatus::Accepted,
                wire::encode_house_page(&[owned], None, false).map_err(debug)?
            )
        );

        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    }))
}

async fn clock(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn public_status(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT payload->0->>'status' FROM game_public_v2_heads WHERE family='house' AND subject=$1")
        .bind(HOUSE).fetch_one(&h.pool).await?)
}

struct AuctionClock(Option<i64>);

impl crate::gameplay_transport::spell_premium_coordinator::clock_registration::Registered for AuctionClock {}

impl crate::gameplay_transport::spell_premium_coordinator::RegisteredPremiumClock for AuctionClock {
    fn read_current_window(&self) -> Option<crate::premium::TrustedNow> {
        crate::premium::TrustedNow::new(self.0?, 0)
    }
}

async fn premium_platform(
    h: &Harness,
    paid: bool,
    seed_stale: bool,
) -> TestResult<(
    premium_test_producer::TestProducer,
    crate::premium::client::PremiumSnapshotClient,
)> {
    use crate::premium::client::{PremiumClientConfig, PremiumSnapshotClient, fresh_nonce};
    use premium_test_producer::{Reply, TestPki, TestProducer, snapshot};
    let stamps: Vec<String> = sqlx::query_scalar(
        "SELECT to_char(statement_timestamp() AT TIME ZONE 'UTC' + offset_minutes * interval '1 minute',
            'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
         FROM unnest(ARRAY[-85,-25,-45,-1,59,39]) WITH ORDINALITY AS offsets(offset_minutes,ord)
         ORDER BY ord",
    ).fetch_all(&h.pool).await?;
    let [old_issued, old_valid, old_refresh, issued, valid, refresh] = stamps.as_slice() else {
        return Err("six Platform timestamps".into());
    };
    let pki = TestPki::new();
    let old = [old_issued.clone(), old_valid.clone(), old_refresh.clone()];
    let producer = TestProducer::start(&pki, move |request| {
        Reply::json(snapshot(request, 1, &[
            ("authority_issued_at", old[0].clone().into()),
            ("authority_valid_until", old[1].clone().into()),
            ("refresh_after", old[2].clone().into()),
        ]))
    }).await;
    let client = PremiumSnapshotClient::new(&PremiumClientConfig {
        origin: producer.origin.clone(),
        identity_pem: pki.client_identity_pem.into_bytes(),
        platform_ca_pem: pki.ca_pem.into_bytes(),
    })?;
    if seed_stale {
        let nonce = fresh_nonce().ok_or("test Premium nonce")?;
        let body = client.pull(id(40), &nonce).await.map_err(debug)?;
        let evidence = crate::premium::snapshot::validate(&body, id(40), &nonce).map_err(debug)?;
        h.root.accept_premium_evidence(&evidence).await?;
    }
    let fresh = [issued.clone(), valid.clone(), refresh.clone()];
    producer.script(move |request| {
        let mut facts = vec![
            ("authority_issued_at", fresh[0].clone().into()),
            ("authority_valid_until", fresh[1].clone().into()),
            ("refresh_after", fresh[2].clone().into()),
        ];
        if !paid {
            facts.extend([
                ("entitlement_state", "NONE".into()),
                ("entitlement_id", serde_json::Value::Null),
                ("lifecycle_revision", 0.into()),
                ("effective_from", serde_json::Value::Null),
                ("effective_until", serde_json::Value::Null),
            ]);
        }
        Reply::json(snapshot(request, 2, &facts))
    });
    Ok((producer, client))
}

async fn seed_full_census(
    h: &Harness,
    world: WorldId,
    digest: &[u8; 32],
) -> TestResult<crate::durability::house_ownership::HouseSettlementRequest> {
    use crate::durability::house_ownership::HouseSettlementRequest;
    let house_key = "oteryn:content.house.full_census";
    let auction = id(217);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest)
        VALUES(encode($1,'hex')::uuid,$2,'private_house',2,0,$3)")
        .bind(world.as_bytes().as_slice()).bind(house_key).bind(digest.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,ownership_revision,acquisition_receipt_ref)
        VALUES(encode($1,'hex')::uuid,$2,1,game_character_uuid_v7())")
        .bind(world.as_bytes().as_slice()).bind(house_key).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,0,clock_timestamp()-interval '1 second')")
        .bind(auction.as_slice()).bind(world.as_bytes().as_slice()).bind(house_key).execute(&mut *tx).await?;
    sqlx::query("CREATE TEMP TABLE auction_fixture_accounts ON COMMIT DROP AS
        SELECT i,game_character_uuid_v7() AS account,game_character_uuid_v7() AS actor FROM generate_series(1,256) g(i)")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_account_guards(account_id) SELECT account FROM auction_fixture_accounts")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,
        profile_revision,ruleset_revision,content_revision,starter_template_revision,name)
        SELECT actor,account,encode($1,'hex')::uuid,1,1,'profile-1','ruleset-1','content-1',
            'starter-1','Bidder '||chr(65+i/26)||chr(65+i%26)
        FROM auction_fixture_accounts")
        .bind(world.as_bytes().as_slice()).execute(&mut *tx).await?;
    let inserted = sqlx::query("INSERT INTO game_house_bids(auction_id,account_id,world_id,character_id,
        maximum,first_rent,reached_order,state,escrow_gold)
        SELECT encode($1,'hex')::uuid,account,encode($2,'hex')::uuid,actor,0,0,i,1,0 FROM auction_fixture_accounts")
        .bind(auction.as_slice()).bind(world.as_bytes().as_slice()).execute(&mut *tx).await?;
    assert_eq!(inserted.rows_affected(), 256);
    tx.commit().await?;
    // Pin the actual transaction deadline independently of test-host RTT.
    sqlx::query("CREATE FUNCTION assert_auction_pass_budget() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF current_setting('transaction_timeout') = '0'
                OR current_setting('transaction_timeout')::interval > interval '500 milliseconds' THEN
                RAISE EXCEPTION 'auction settlement exceeded its background transaction budget';
            END IF;
            RETURN NEW;
        END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER assert_auction_pass_budget BEFORE INSERT ON game_house_settlement_steps
        FOR EACH ROW EXECUTE FUNCTION assert_auction_pass_budget()")
        .execute(&h.pool).await?;
    Ok(HouseSettlementRequest { world, house_key: house_key.into(), auction, step: 0 })
}
