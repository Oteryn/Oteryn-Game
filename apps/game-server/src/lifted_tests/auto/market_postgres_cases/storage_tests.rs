use super::*;

fn sqlstate<T>(result: Result<T, sqlx::Error>) -> TestResult<String> {
    match result {
        Ok(_) => Err("expected a rejected Market write".into()),
        Err(error) => error
            .as_database_error()
            .and_then(|error| error.code())
            .map(|code| code.into_owned())
            .ok_or_else(|| error.into()),
    }
}

// Copies a disclosed administrative fixture inside a caller-owned transaction.
// This is not a value-moving writer and does not establish actor authority.
async fn copy_offer(
    connection: &mut PgConnection,
    source: u8,
    target: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO game_market_offers(offer_id,world_id,definition_key,definition_revision,side,account_id,character_id,piece_price,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold,held_credit_gold,matching_pending) \
         SELECT encode($2,'hex')::uuid,world_id,definition_key,definition_revision,side,account_id,character_id,piece_price,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold,held_credit_gold,matching_pending \
         FROM game_market_offers WHERE offer_id=encode($1,'hex')::uuid"
    ).bind(id(source).as_slice()).bind(id(target).as_slice()).execute(connection).await?;
    Ok(())
}

#[test]
fn market_storage_capacity_serializes_last_slot_across_sides_and_books() -> TestResult {
    run("market_slot_race", async |h| {
        for index in 100..199 {
            seed(
                h,
                &offer(
                    index,
                    if index % 2 == 0 {
                        Side::Buy
                    } else {
                        Side::Sell
                    },
                    40,
                    100,
                ),
            )
            .await?;
        }
        let mut first = h.pool.begin().await?;
        copy_offer(&mut first, 100, 200).await?;
        let mut second = PgConnection::connect(&h.database.url).await?;
        second.execute("SET statement_timeout='15s'").await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut second)
            .await?;
        let later = tokio::spawn(async move { copy_offer(&mut second, 101, 201).await });
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Lock')")
                    .bind(pid).fetch_one(&h.pool).await?;
                if waiting { return Ok::<_, sqlx::Error>(()); }
                tokio::task::yield_now().await;
            }
        }).await??;
        first.commit().await?;
        assert_eq!(sqlstate(later.await?)?, "23514");
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_market_offers")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, MAX_OPEN_OFFERS as i64);
        // Changing the ware cannot evade a per-character limit.
        let mut other_book = offer(202, Side::Sell, 40, 100);
        other_book.definition = definition("oteryn:content.item.bowl", "r1")?;
        assert!(seed(h, &other_book).await.is_err());
        // Another Character has its own allowance.
        seed(h, &offer(203, Side::Buy, 70, 100)).await?;
        Ok(())
    })
}

#[test]
fn market_storage_bounds_binding_and_end_metadata_are_enforced() -> TestResult {
    run("market_storage_bounds", async |h| {
        let good = offer(60, Side::Sell, 40, 100);
        seed(h, &good).await?;
        for (amount, piece_price) in [
            (0, 1),
            (64_001, 1),
            (1, 0),
            (1, MAX_TOTAL + 1),
            (2, MAX_TOTAL),
        ] {
            let mut invalid = good.clone();
            invalid.id = MarketOfferId::new(id(61))?;
            invalid.amount = amount;
            invalid.remaining = amount;
            invalid.piece_price = piece_price;
            assert!(seed(h, &invalid).await.is_err(), "{amount} × {piece_price}");
        }
        for statement in [
            "UPDATE game_market_offers SET expires_at_us=expires_at_us-1",
            "UPDATE game_market_offers SET created_at_us=9223372036854775807",
            "UPDATE game_market_offers SET state=4",
            "UPDATE game_market_offers SET ended_at_us=101,end_sequence=1",
            "UPDATE game_market_offers SET character_id=game_character_uuid_v7()",
            "UPDATE game_market_offers SET account_id=game_character_uuid_v7()",
            "UPDATE game_market_offers SET world_id=game_character_uuid_v7()",
            "UPDATE game_market_offers SET anonymous=true",
        ] {
            assert_eq!(
                sqlstate(h.pool.execute(statement).await)?,
                "23514",
                "{statement}"
            );
        }
        // New offers must bind to their real root even for a valid AccountId.
        let mut mismatch = good.clone();
        mismatch.id = MarketOfferId::new(id(61))?;
        mismatch.world = WorldId::from_bytes(id(90))?;
        assert!(seed(h, &mismatch).await.is_err());
        // A valid ending frees a slot and survives a fresh connection.
        h.pool.execute("UPDATE game_market_offers SET state=4,ended_at_us=expires_at_us,end_sequence=nextval('game_market_end_sequence')").await?;
        let mut connection = PgConnection::connect(&h.database.url).await?;
        let ended = ended_character_offers(&mut connection, world(), good.character).await?;
        assert_eq!(ended.len(), 1);
        assert_eq!(ended[0].offer.state, OfferState::Ended(EndState::Expired));
        assert_eq!(ended[0].ended_at_us, good.expires_at_us);
        assert_eq!(
            sqlstate(
                connection
                    .execute(
                        "UPDATE game_market_offers SET state=1,ended_at_us=NULL,end_sequence=NULL"
                    )
                    .await
            )?,
            "23514"
        );
        connection.close().await?;
        Ok(())
    })
}

#[test]
fn market_storage_runtime_reads_but_cannot_write_without_fenced_owner() -> TestResult {
    run("market_storage_roles", async |h| {
        let created = now(h).await? - 1_000;
        let mut buy = offer(60, Side::Buy, 40, created);
        buy.anonymous = true;
        seed(h, &buy).await?;
        seed(h, &offer(61, Side::Sell, 70, created)).await?;
        let mut connection = PgConnection::connect(&h.database.url).await?;
        connection.execute("SET ROLE oteryn_game_runtime").await?;
        assert_eq!(
            open_character_offers(&mut connection, world(), buy.character).await?,
            [buy.clone()]
        );
        let page = book_page(&mut connection, &buy.book(), Side::Buy, None).await?;
        assert_eq!(page.offers, [buy]);
        for statement in [
            "INSERT INTO game_market_books SELECT * FROM game_market_books",
            "UPDATE game_market_books SET definition_key='oteryn:content.item.bowl'",
            "DELETE FROM game_market_books",
            "INSERT INTO game_market_offers SELECT * FROM game_market_offers",
            "UPDATE game_market_offers SET remaining=0",
            "DELETE FROM game_market_offers",
            "TRUNCATE game_market_offers",
            "INSERT INTO game_market_operations SELECT * FROM game_market_operations",
            "UPDATE game_market_operations SET outcome=outcome",
            "DELETE FROM game_market_operations",
            "TRUNCATE game_market_operations",
            "INSERT INTO game_item_market_escrow_locations SELECT * FROM game_item_market_escrow_locations",
            "UPDATE game_item_market_escrow_locations SET ordinal=1",
            "DELETE FROM game_item_market_escrow_locations",
            "SELECT nextval('game_market_end_sequence')",
        ] {
            assert_eq!(
                sqlstate(connection.execute(statement).await)?,
                "42501",
                "{statement}"
            );
        }
        connection.execute("RESET ROLE").await?;
        // Even the administrative owner cannot rewrite an audit occurrence.
        for statement in [
            "DELETE FROM game_market_operations",
            "TRUNCATE game_market_operations",
        ] {
            assert_eq!(sqlstate(connection.execute(statement).await)?, "23514");
        }
        connection.close().await?;
        Ok(())
    })
}
