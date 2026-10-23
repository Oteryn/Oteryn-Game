use super::*;
#[test]
fn house_wire_postgres_bid_dispatch_replay_and_conflicting_maximum() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "house bid PG thread panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "house_wire_bid", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let gate =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
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
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
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
        // Recovery DATA, like HOUSE-1a's native tests: already open, zero first rent.
        // This does not qualify VACANT catalogue activation or a settlement scheduler.
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest) VALUES(encode($1,'hex')::uuid,$2,'private_house',2,0,$3)").bind(id(42).as_slice()).bind(HOUSE).bind(digest.as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_ownership VALUES(encode($1,'hex')::uuid,$2,NULL,1,encode($3,'hex')::uuid)").bind(id(42).as_slice()).bind(HOUSE).bind(id(75).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,0,clock_timestamp()+interval '2 days')").bind(id(76).as_slice()).bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut amount = Vec::new();
        scalar(&mut amount, 1, 0);
        let request = intent(1, &amount);
        let ok = wire::encode_house_result(wire::HouseDisposition::Ok, 0);
        assert_eq!(send(&owner, admitted, 1, 56, &request).await?.1, ok);
        assert_eq!(send(&owner, admitted, 1, 56, &request).await?.1, ok);
        let row:(i64,i64)=sqlx::query_as("SELECT maximum,escrow_gold FROM game_house_bids WHERE auction_id=encode($1,'hex')::uuid AND account_id=encode($2,'hex')::uuid").bind(id(76).as_slice()).bind(id(40).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(row, (0, 0));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_house_operations")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 1);
        let mut amount = Vec::new();
        scalar(&mut amount, 1, 1);
        assert_eq!(
            send(&owner, admitted, 1, 56, &intent(1, &amount)).await?.1,
            wire::encode_house_result(wire::HouseDisposition::Conflict, 0)
        );
        let page = send(&owner, admitted, 2, 57, &query()).await?.1;
        assert!(page.windows(HOUSE.len()).any(|w| w == HOUSE.as_bytes()));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
