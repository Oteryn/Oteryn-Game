use super::*;

#[test]
fn house_ownership_existing_acl_fixture_uses_the_canonical_spine() -> TestResult {
    run("house_ownership_existing_acl", |h| async move {
        let source = include_str!(
            "../../../../../apps/game-server/tests/support/house_spell_privacy_cases.sql"
        );
        let source = source
            .strip_prefix("\\set ON_ERROR_STOP on\n")
            .ok_or("unexpected House ACL fixture header")?
            .replace(
                "SET search_path=spell_house_privacy_audit,pg_catalog;",
                "SET search_path=public,pg_catalog;",
            );
        sqlx::raw_sql(sqlx::AssertSqlSafe(source))
            .execute(&h.pool)
            .await?;
        assert_eq!(h.count("game_house_properties").await?, 1);
        assert_eq!(h.count("game_account_housing_slots").await?, 1);
        Ok(())
    })
}

#[test]
fn house_ownership_deferred_anchor_and_retired_guards() -> TestResult {
    run("house_ownership_integrity", |h| async move {
        seed(&h, false).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid,ownership_revision=2,acquisition_receipt_ref=encode($2,'hex')::uuid WHERE house_key=$3")
            .bind(id(fixture::CHARACTER).as_slice()).bind(id(77).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
        let error = tx
            .commit()
            .await
            .expect_err("canonical and legacy owners must agree at commit");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        sqlx::query(
            "UPDATE game_house_properties SET state=6,revision=revision+1 WHERE house_key=$1",
        )
        .bind(HOUSE)
        .execute(&h.pool)
        .await?;
        let error = sqlx::query(
            "UPDATE game_house_properties SET state=1,revision=revision+1 WHERE house_key=$1",
        )
        .bind(HOUSE)
        .execute(&h.pool)
        .await
        .expect_err("a retired property cannot reopen");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        let error = sqlx::query("DELETE FROM game_house_ownership WHERE house_key=$1")
            .bind(HOUSE)
            .execute(&h.pool)
            .await
            .expect_err("legacy anchor survives every lifecycle");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        Ok(())
    })
}

#[test]
fn house_ownership_acl_revision_and_null_owner_successor() -> TestResult {
    run("house_ownership_acl", |h| async move {
        seed(&h, true).await?;
        let root = runtime_root(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        sqlx::query("UPDATE game_house_auctions SET ends_at=clock_timestamp()-interval '1 second'")
            .execute(&h.pool)
            .await?;
        root.house_settlement(&authority, &h.node, &gate(&root), settlement_request()?)
            .await
            .map_err(debug)?;
        let before: String = sqlx::query_scalar(
            "SELECT acquisition_receipt_ref::text FROM game_house_properties WHERE house_key=$1",
        )
        .bind(HOUSE)
        .fetch_one(&h.pool)
        .await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_house_acl_receipts(receipt_id,world_id,house_key,list_id,character_id,game_session_id,command_id,ownership_revision,revision_before,revision_after,allow_everyone,members) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,-1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,100,2,0,1,true,ARRAY[]::uuid[])")
            .bind(id(85).as_slice()).bind(id(fixture::WORLD).as_slice()).bind(HOUSE).bind(id(fixture::CHARACTER).as_slice()).bind(id(fixture::SESSION).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_acl VALUES(encode($1,'hex')::uuid,$2,-1,1,true,ARRAY[]::uuid[],encode($3,'hex')::uuid)")
            .bind(id(fixture::WORLD).as_slice()).bind(HOUSE).bind(id(85).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        sqlx::query("DELETE FROM game_house_acl WHERE house_key=$1")
            .bind(HOUSE)
            .execute(&h.pool)
            .await?;
        let acl: String = sqlx::query_scalar(
            "SELECT acl_revision::text FROM game_house_properties WHERE house_key=$1",
        )
        .bind(HOUSE)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(acl, "3");
        assert_eq!(h.count("game_house_acl_receipts").await?, 1);
        // Test the storage successor only. No disposition fragment or release
        // service is installed to stand in for deferred HOUSE-1b.
        let mut tx = h.pool.begin().await?;
        sqlx::query("DELETE FROM game_account_housing_slots WHERE house_key=$1")
            .bind(HOUSE)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_house_properties SET state=1,owner_character_id=NULL,owner_account_id=NULL,ownership_generation=ownership_generation+1,revision=revision+1 WHERE house_key=$1")
            .bind(HOUSE).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_house_ownership SET owner_character_id=NULL,ownership_revision=ownership_revision+1 WHERE house_key=$1")
            .bind(HOUSE).execute(&mut *tx).await?;
        tx.commit().await?;
        let result:(String,String,bool)=sqlx::query_as("SELECT ownership_generation::text,acquisition_receipt_ref::text,owner_character_id IS NULL FROM game_house_properties WHERE house_key=$1")
            .bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(result, ("3".into(), before, true));
        assert_eq!(h.count("game_house_ownership").await?, 1);
        assert_eq!(h.count("game_account_housing_slots").await?, 0);
        Ok(())
    })
}

#[test]
fn house_ownership_settlement_excludes_not_current_premium_and_refunds_once() -> TestResult {
    run("house_ownership_excluded", |h| async move {
        seed(&h, true).await?;
        let root = runtime_root(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        sqlx::query("UPDATE game_house_auctions SET ends_at=clock_timestamp()-interval '1 second'")
            .execute(&h.pool)
            .await?;
        let unknown =
            SpellPremiumCoordinator::new(Arc::new(PremiumConsumer::default()), None, None, None);
        assert_eq!(
            root.house_settlement(&authority, &h.node, &unknown, settlement_request()?)
                .await
                .map_err(debug)?,
            HouseOwnershipOutcome::Applied
        );
        assert_eq!(amounts(&h).await?, (600, 0, 1));
        assert_eq!(h.count("game_account_housing_slots").await?, 0);
        assert_eq!(
            root.house_settlement(&authority, &h.node, &unknown, settlement_request()?)
                .await
                .map_err(debug)?,
            HouseOwnershipOutcome::Replayed
        );
        assert_eq!(amounts(&h).await?, (600, 0, 1));
        Ok(())
    })
}
