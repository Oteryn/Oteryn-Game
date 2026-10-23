use super::*;

#[test]
fn market_native_operation_history_binding_immutability_and_occurrence_namespace() -> TestResult {
    run("marketreceipt", async |h| {
        let occurrence = Occurrence::Command(CommandRef::new(
            GameSessionId::decode(&id(50))?,
            CommandId::new(1)?,
        ));
        let actor = CharacterId::from_bytes(id(CHARACTER))?;
        sqlx::query("INSERT INTO game_market_operations VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,3,encode($4,'hex')::uuid,$5,$6)").bind(id(WORLD).as_slice()).bind(fixture_occurrence_key(occurrence)?).bind(id(60).as_slice()).bind(actor.as_bytes().as_slice()).bind([7_u8;32].as_slice()).bind([1_u8,2,3].as_slice()).execute(&h.pool).await?;
        let mut c = h.pool.acquire().await?;
        let first = read_operation(
            &mut c,
            world(),
            occurrence,
            OperationKind::Cancel,
            Some(actor),
            &[7; 32],
        )
        .await
        .map_err(harness::debug)?
        .ok_or("receipt")?;
        assert_eq!(first.transaction_id, id(60));
        assert_eq!(first.outcome, vec![1, 2, 3]);
        assert_eq!(
            read_operation(
                &mut c,
                world(),
                occurrence,
                OperationKind::Cancel,
                Some(actor),
                &[7; 32]
            )
            .await
            .map_err(harness::debug)?,
            Some(first)
        );
        assert!(matches!(
            read_operation(
                &mut c,
                world(),
                occurrence,
                OperationKind::Cancel,
                Some(actor),
                &[8; 32]
            )
            .await,
            Err(OperationReadError::ConflictingBinding)
        ));
        assert!(matches!(
            read_operation(
                &mut c,
                world(),
                occurrence,
                OperationKind::Accept,
                Some(actor),
                &[7; 32]
            )
            .await,
            Err(OperationReadError::ConflictingBinding)
        ));
        assert!(matches!(
            read_operation(
                &mut c,
                WorldId::from_bytes(id(99))?,
                occurrence,
                OperationKind::Cancel,
                Some(actor),
                &[7; 32]
            )
            .await,
            Err(OperationReadError::ConflictingBinding)
        ));
        assert!(matches!(
            read_operation(
                &mut c,
                world(),
                occurrence,
                OperationKind::Cancel,
                Some(CharacterId::from_bytes(id(99))?),
                &[7; 32]
            )
            .await,
            Err(OperationReadError::ConflictingBinding)
        ));
        let job = Occurrence::Job {
            offer: MarketOfferId::new(id(60))?,
            step: JobStep::Expire,
            number: 1,
        };
        assert_ne!(
            fixture_occurrence_key(job)?,
            fixture_occurrence_key(occurrence)?
        );
        assert!(
            fixture_occurrence_key(Occurrence::Job {
                offer: MarketOfferId::new(id(60))?,
                step: JobStep::Expire,
                number: 0
            })
            .is_err()
        );
        let error = sqlx::query("UPDATE game_market_operations SET outcome=$1")
            .bind([4_u8].as_slice())
            .execute(&mut *c)
            .await
            .expect_err("immutable");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        Ok(())
    })
}

#[test]
fn market_native_escrow_reads_real_items_bounded_and_offer_bound() -> TestResult {
    run("marketescrow", async |h| {
        let mut o = offer(60, Side::Sell, 40, now(h).await? - 1000);
        o.amount = 100;
        o.remaining = 100;
        seed(h, &o).await?;
        seed_escrow(h, &o, 100).await?;
        let mut tx = h.pool.begin().await?;
        let mut book = lock_book(&mut tx, o.book()).await?;
        let rows = book.escrow(&o).await?;
        assert_eq!(rows.len(), 100);
        assert_eq!(rows.iter().map(|r| u64::from(r.quantity)).sum::<u64>(), 100);
        assert!(rows.windows(2).all(|w| w[0].item < w[1].item));
        assert!(rows.iter().all(|r| r.ordinal > 0));
        let mut substituted = o.clone();
        substituted.remaining = 99;
        assert!(book.escrow(&substituted).await.is_err());
        tx.rollback().await?;
        h.pool
            .execute("ALTER TABLE game_market_offers DISABLE TRIGGER game_market_offer_guard")
            .await?;
        sqlx::query("UPDATE game_market_offers SET amount=101,remaining=101")
            .execute(&h.pool)
            .await?;
        let extra = offer(61, Side::Sell, 40, now(h).await? - 1000);
        seed(h, &extra).await?;
        seed_escrow(h, &extra, 1).await?;
        sqlx::query("UPDATE game_item_market_escrow_locations SET offer_id=encode($1,'hex')::uuid,ordinal=101 WHERE offer_id=encode($2,'hex')::uuid").bind(o.id.as_bytes().as_slice()).bind(extra.id.as_bytes().as_slice()).execute(&h.pool).await?;
        let mut tx = h.pool.begin().await?;
        let mut book = lock_book(&mut tx, o.book()).await?;
        let actual = book.offer(o.id).await?.ok_or("offer")?;
        assert!(book.escrow(&actual).await.is_err());
        tx.rollback().await?;
        Ok(())
    })
}
#[test]
fn market_native_schema_rejects_invalid_gold_custody_and_ended_matching() -> TestResult {
    run("marketguards", async |h| {
        let o = offer(60, Side::Buy, 40, now(h).await? - 1000);
        seed(h, &o).await?;
        for query in [
            "UPDATE game_market_offers SET escrow_gold=999",
            "UPDATE game_market_offers SET state=5,held_credit_gold=1000,ended_as=3",
            "UPDATE game_market_offers SET state=2,escrow_gold=0",
            "UPDATE game_market_offers SET state=3,escrow_gold=0,matching_pending=true",
        ] {
            let mut tx = h.pool.begin().await?;
            let result = sqlx::query(query).execute(&mut *tx).await;
            assert_eq!(
                result
                    .expect_err("guard")
                    .as_database_error()
                    .and_then(|e| e.code())
                    .as_deref(),
                Some("23514")
            );
            tx.rollback().await?;
        }
        Ok(())
    })
}

#[test]
fn market_stored_definition_validation_preserves_production_identity_restrictions() -> TestResult {
    run("marketdefinition", async |h| {
        // Deliberately corrupt only this isolated historical decoder fixture;
        // production constraints and immutable terms remain enabled elsewhere.
        h.pool
            .execute("ALTER TABLE game_market_offers DISABLE TRIGGER game_market_offer_guard")
            .await?;
        h.pool.execute("ALTER TABLE game_market_books DROP CONSTRAINT game_market_books_definition_key_check").await?;
        h.pool.execute("ALTER TABLE game_market_offers DROP CONSTRAINT game_market_offers_definition_revision_check").await?;
        let o = offer(60, Side::Sell, 40, 100);
        seed(h, &o).await?;
        for (key, revision) in [
            ("unnamespaced".to_owned(), "r1".to_owned()),
            ("n:é".to_owned(), "r1".to_owned()),
            ("n:test-ware".to_owned(), "r1".to_owned()),
            ("n:fixture-ware".to_owned(), "r1".to_owned()),
            ("n:synthetic".to_owned(), "r1".to_owned()),
            ("n:evidence".to_owned(), "r1".to_owned()),
            (format!("n:{}", "x".repeat(511)), "r1".to_owned()),
            ("n:ware".to_owned(), "r 1".to_owned()),
            ("n:ware".to_owned(), "révision".to_owned()),
            ("n:ware".to_owned(), "test/r1".to_owned()),
            ("n:ware".to_owned(), "".to_owned()),
            ("n:ware".to_owned(), "r".repeat(513)),
        ] {
            sqlx::query("INSERT INTO game_market_books VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT DO NOTHING")
                .bind(o.world.as_bytes().as_slice()).bind(&key).execute(&h.pool).await?;
            sqlx::query("UPDATE game_market_offers SET definition_key=$2,definition_revision=$3 WHERE offer_id=encode($1,'hex')::uuid")
                .bind(o.id.as_bytes().as_slice()).bind(&key).bind(&revision).execute(&h.pool).await?;
            let mut connection = h.pool.acquire().await?;
            assert!(
                matches!(
                    open_character_offers(&mut connection, world(), o.character).await,
                    Err(DurabilityError::InvalidStoredState)
                ),
                "{key:?} / {revision:?}"
            );
        }
        // Preserve the exact accepted boundaries; revision syntax permits graphic '+' even
        // though ProductionKey syntax does not. Validation identifies data, not admission.
        assert!(definition(&format!("n:{}", "x".repeat(510)), &"r".repeat(512)).is_ok());
        assert!(definition("n:ware", "r+1").is_ok());
        assert!(
            StoredItemDefinition::new(crate::durability::item_mint::TypedDefinitionRef {
                family: "Creature".to_owned(),
                production_key: "n:ware".to_owned(),
                revision_ref: "r1".to_owned(),
            })
            .is_err()
        );
        Ok(())
    })
}
