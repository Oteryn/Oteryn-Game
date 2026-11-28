//! Qualified vacant property -> real bid command -> scheduled settlement owner.
use super::*;
use crate::durability::house_ownership::{
    HouseAuctionCatalogue, HouseAuctionProperty, HouseOwnershipOutcome,
};
use crate::foundation::{ChannelId, CharacterId, CharacterLease, RuntimeScopeRefV1, WorldId};
use crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
use sqlx::ConnectOptions;

#[test]
fn house_auction_postgres_first_bid_opens_and_due_winner_owns() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "house auction opening PG thread panicked")??;
    Ok(())
}

fn scenario(admin: String) -> TestResult {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(Box::pin(async move {
        reset();
        let mut h =
            Harness::create_with_admission_seed(admin, "house_auction_opening", true, false)
                .await?;
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
        let funding = rent + 900;
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
        let command = admitted.continuity.next_command_id;
        let earliest = clock(&h).await?;
        let ok = wire::encode_house_result(wire::HouseDisposition::Ok, 0);
        assert_eq!(
            send(&owner, &mut admitted, 56, &bid).await?,
            (CommandStatus::Accepted, ok.clone()),
            "the first funded bid opens a qualified vacant house"
        );
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
        assert_eq!((opened.0, opened.1, opened.2), (rent, rent + 200, 700));
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

        sqlx::query("UPDATE game_house_auctions SET ends_at=clock_timestamp()-interval '1 second'")
            .execute(&h.pool)
            .await?;
        let mut due = h
            .root
            .due_house_auctions(world, None)
            .await
            .map_err(debug)?;
        assert_eq!(due.len(), 1);
        let winner = due.pop().ok_or("due auction")?;
        assert_eq!(
            h.root
                .house_settlement(&authority, &h.node, &gate, winner.clone())
                .await
                .map_err(debug)?,
            HouseOwnershipOutcome::Applied
        );
        assert_eq!(
            h.root
                .house_settlement(&authority, &h.node, &gate, winner)
                .await
                .map_err(debug)?,
            HouseOwnershipOutcome::Replayed
        );
        let owned: (Vec<u8>,i64,i16,i64,i64) = sqlx::query_as(
            "SELECT uuid_send(game_house_effective_owner(p.world_id,p.house_key)),p.ownership_generation::bigint,b.state,b.escrow_gold,v.balance
             FROM game_house_properties p JOIN game_house_auctions a USING(world_id,house_key)
             JOIN game_house_bids b USING(auction_id)
             JOIN game_account_bank_balances v ON (v.world_id,v.account_id)=(b.world_id,b.account_id)
             JOIN game_account_housing_slots s ON (s.world_id,s.house_key,s.account_id)=(p.world_id,p.house_key,p.owner_account_id)"
        ).fetch_one(&h.pool).await?;
        assert_eq!(owned, (id(41).to_vec(), 2, 2, 0, 900));
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
