use super::*;

#[test]
fn market_own_open_offers_are_scoped_ordered_and_include_expiry_backlog() -> TestResult {
    run("marketown", async |h| {
        let mut older = offer(60, Side::Buy, 40, 100);
        older.anonymous = true;
        let newer = offer(61, Side::Sell, 40, 200);
        let tied = offer(62, Side::Buy, 40, 200);
        let mut ended = offer(63, Side::Sell, 40, 300);
        ended.state = OfferState::Ended(EndState::Cancelled);
        let mut held = offer(64, Side::Buy, 40, 300);
        held.state = OfferState::CreditHeld(EndState::Cancelled);
        held.escrow_gold = 0;
        held.held_credit_gold = 1000;
        let other_character = offer(65, Side::Sell, 70, 400);
        for o in [&older, &newer, &tied, &ended, &held, &other_character] {
            seed(h, o).await?;
        }
        assert!(now(h).await? > tied.expires_at_us);
        let mut connection = h.pool.acquire().await?;
        assert_eq!(
            open_character_offers(&mut connection, world(), older.character).await?,
            vec![tied, newer, older.clone()]
        );
        assert!(
            open_character_offers(
                &mut connection,
                WorldId::from_bytes(id(90))?,
                older.character,
            )
            .await?
            .is_empty()
        );
        assert_eq!(
            open_character_offers(&mut connection, world(), other_character.character).await?,
            vec![other_character]
        );
        Ok(())
    })
}

#[test]
fn market_own_open_offers_reject_overflow_instead_of_truncating() -> TestResult {
    run("marketownbound", async |h| {
        for seed_id in 100..200 {
            seed(h, &offer(seed_id, Side::Sell, 40, 100)).await?;
        }
        let character = CharacterId::from_bytes(id(41))?;
        {
            let mut connection = h.pool.acquire().await?;
            assert_eq!(
                open_character_offers(&mut connection, world(), character)
                    .await?
                    .len(),
                100
            );
        }
        // Prove native admission first, then inject a corrupt historical fixture
        // to retain the existing decoder's independent overflow assertion.
        assert!(seed(h, &offer(200, Side::Sell, 40, 100)).await.is_err());
        h.pool
            .execute("ALTER TABLE game_market_offers DISABLE TRIGGER game_market_offer_guard")
            .await?;
        seed(h, &offer(200, Side::Sell, 40, 100)).await?;
        let mut connection = h.pool.acquire().await?;
        assert!(matches!(
            open_character_offers(&mut connection, world(), character).await,
            Err(DurabilityError::InvalidStoredState)
        ));
        Ok(())
    })
}

#[test]
fn market_book_pages_follow_best_price_time_and_id_without_skips() -> TestResult {
    run("marketpages", async |h| {
        let created = now(h).await? - 1000;
        let key = offer(60, Side::Sell, 40, created).book();
        for (side, first_id, account) in [(Side::Sell, 100, 40), (Side::Buy, 150, 70)] {
            let mut expected = Vec::new();
            for index in 0..35 {
                let mut o = offer(
                    first_id + index,
                    side,
                    account,
                    created - i64::from(index % 3),
                );
                price(&mut o, [2, 9, 10, 100, 101][usize::from(index % 5)]);
                seed(h, &o).await?;
                expected.push(o);
            }
            expected.sort_by(|a, b| {
                let prices = match side {
                    Side::Sell => a.piece_price.cmp(&b.piece_price),
                    Side::Buy => b.piece_price.cmp(&a.piece_price),
                };
                prices
                    .then(a.created_at_us.cmp(&b.created_at_us))
                    .then(a.id.cmp(&b.id))
            });
            let mut connection = h.pool.acquire().await?;
            let first = book_page(&mut connection, &key, side, None).await?;
            assert_eq!(first.offers, expected[..32]);
            let cursor = first.next.expect("continuation");
            let second = book_page(&mut connection, &key, side, Some(&cursor)).await?;
            assert_eq!(second.offers, expected[32..]);
            assert_eq!(second.next, None);
            let wrong_side = if side == Side::Buy {
                Side::Sell
            } else {
                Side::Buy
            };
            assert!(matches!(
                book_page(&mut connection, &key, wrong_side, Some(&cursor)).await,
                Err(DurabilityError::InvalidStoredState)
            ));
            let wrong_book = BookKey {
                world: WorldId::from_bytes(id(90))?,
                ..key.clone()
            };
            assert!(matches!(
                book_page(&mut connection, &wrong_book, side, Some(&cursor)).await,
                Err(DurabilityError::InvalidStoredState)
            ));
            for (piece_price, created_at_us) in [
                (0, cursor.created_at_us),
                (MAX_TOTAL + 1, cursor.created_at_us),
                (cursor.piece_price, -1),
            ] {
                let invalid_cursor = BookCursor {
                    piece_price,
                    created_at_us,
                    ..cursor.clone()
                };
                assert!(matches!(
                    book_page(&mut connection, &key, side, Some(&invalid_cursor)).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
            }
        }
        Ok(())
    })
}

#[test]
fn market_book_pages_exclude_expired_future_ended_and_other_wares() -> TestResult {
    run("marketpagefilter", async |h| {
        let created = now(h).await? - 1000;
        let mut live = offer(60, Side::Sell, 40, created);
        live.anonymous = true;
        let expired = offer(61, Side::Sell, 40, 0);
        let future = offer(62, Side::Sell, 40, created + 1_000_000_000);
        let mut ended = offer(63, Side::Sell, 40, created);
        ended.state = OfferState::Ended(EndState::Cancelled);
        let mut other_ware = offer(64, Side::Sell, 40, created);
        other_ware.definition = definition("oteryn:content.item.other", "r1")?;
        for o in [&live, &expired, &future, &ended, &other_ware] {
            seed(h, o).await?;
        }
        let mut connection = h.pool.acquire().await?;
        let page = book_page(&mut connection, &live.book(), Side::Sell, None).await?;
        assert_eq!(page.offers, vec![live.clone()]);
        assert_eq!(page.next, None);
        let other_world = BookKey {
            world: WorldId::from_bytes(id(90))?,
            ..live.book()
        };
        assert!(
            book_page(&mut connection, &other_world, Side::Sell, None)
                .await?
                .offers
                .is_empty()
        );
        Ok(())
    })
}

#[test]
fn market_ended_history_uses_ending_order_scopes_and_keeps_held_credit() -> TestResult {
    run("markethistory", async |h| {
        let mut created_newer = offer(60, Side::Sell, 40, 200);
        created_newer.state = OfferState::Ended(EndState::Cancelled);
        let mut ended_newer = offer(61, Side::Buy, 40, 100);
        ended_newer.anonymous = true;
        ended_newer.state = OfferState::CreditHeld(EndState::Cancelled);
        ended_newer.escrow_gold = 0;
        ended_newer.held_credit_gold = 1000;
        let mut tied = offer(62, Side::Sell, 40, 150);
        tied.state = OfferState::Ended(EndState::Filled);
        tied.remaining = 0;
        let mut other_character = offer(63, Side::Sell, 70, 100);
        other_character.state = OfferState::Ended(EndState::Cancelled);
        let mut other_world = created_newer.clone();
        other_world.id = MarketOfferId::new(id(64))?;
        other_world.world = WorldId::from_bytes(id(90))?;
        other_world.character = CharacterId::from_bytes(id(91))?;
        seed_ending(h, &created_newer, 300, 1).await?;
        seed_ending(h, &ended_newer, 400, 2).await?;
        seed_ending(h, &tied, 400, 3).await?;
        seed_ending(h, &other_character, 500, 4).await?;
        seed_ending(h, &other_world, 600, 5).await?;
        seed(h, &offer(65, Side::Sell, 40, 700)).await?;
        let mut connection = h.pool.acquire().await?;
        assert_eq!(
            ended_character_offers(&mut connection, world(), created_newer.character).await?,
            vec![
                EndedOffer {
                    offer: tied,
                    ended_at_us: 400,
                    end_sequence: 3
                },
                EndedOffer {
                    offer: ended_newer,
                    ended_at_us: 400,
                    end_sequence: 2
                },
                EndedOffer {
                    offer: created_newer,
                    ended_at_us: 300,
                    end_sequence: 1
                },
            ]
        );
        assert_eq!(
            ended_character_offers(&mut connection, world(), other_character.character).await?,
            vec![EndedOffer {
                offer: other_character,
                ended_at_us: 500,
                end_sequence: 4
            }]
        );
        assert_eq!(
            ended_character_offers(&mut connection, other_world.world, other_world.character)
                .await?,
            vec![EndedOffer {
                offer: other_world,
                ended_at_us: 600,
                end_sequence: 5
            }]
        );
        Ok(())
    })
}

#[test]
fn market_ended_history_limits_retention_backlog_to_latest_600() -> TestResult {
    run("markethistorybound", async |h| {
        let mut expected = Vec::new();
        for index in 0_u16..602 {
            let mut o = offer(100, Side::Sell, 40, 1000 - i64::from(index));
            let mut unique = *o.id.as_bytes();
            unique[14..].copy_from_slice(&index.to_be_bytes());
            o.id = MarketOfferId::new(unique)?;
            o.state = OfferState::Ended(EndState::Cancelled);
            let ended_at_us = 2000 + i64::from(index / 2);
            let end_sequence = i64::from(index) + 1;
            seed_ending(h, &o, ended_at_us, end_sequence).await?;
            expected.push(EndedOffer {
                offer: o,
                ended_at_us,
                end_sequence,
            });
        }
        expected.reverse();
        expected.truncate(600);
        let mut connection = h.pool.acquire().await?;
        assert_eq!(
            ended_character_offers(&mut connection, world(), CharacterId::from_bytes(id(41))?)
                .await?,
            expected
        );
        Ok(())
    })
}

#[test]
fn market_ended_history_rejects_missing_invalid_and_duplicate_end_metadata() -> TestResult {
    run("markethistorybad", async |h| {
        // Corrupt storage solely in this isolated decoder fixture. Separate
        // migration tests assert these same constraints remain enforced.
        h.pool.execute("ALTER TABLE game_market_offers DROP CONSTRAINT game_market_ending_metadata, DROP CONSTRAINT game_market_end_sequence_unique").await?;
        let mut o = offer(60, Side::Sell, 40, 100);
        o.state = OfferState::Ended(EndState::Cancelled);
        seed(h, &o).await?;
        for (ended_at_us, end_sequence) in [
            (None, Some(1_i64)),
            (Some(200_i64), None),
            (Some(-1), Some(1)),
            (Some(99), Some(1)),
            (Some(200), Some(0)),
            (Some(200), Some(-1)),
        ] {
            sqlx::query("UPDATE game_market_offers SET ended_at_us=$2,end_sequence=$3 WHERE offer_id=encode($1,'hex')::uuid")
                .bind(o.id.as_bytes().as_slice()).bind(ended_at_us).bind(end_sequence)
                .execute(&h.pool).await?;
            let mut connection = h.pool.acquire().await?;
            assert!(matches!(
                ended_character_offers(&mut connection, world(), o.character).await,
                Err(DurabilityError::InvalidStoredState)
            ));
        }
        sqlx::query("UPDATE game_market_offers SET ended_at_us=200,end_sequence=1")
            .execute(&h.pool)
            .await?;
        let mut duplicate = o.clone();
        duplicate.id = MarketOfferId::new(id(61))?;
        seed_ending(h, &duplicate, 201, 1).await?;
        let mut connection = h.pool.acquire().await?;
        assert!(matches!(
            ended_character_offers(&mut connection, world(), o.character).await,
            Err(DurabilityError::InvalidStoredState)
        ));
        Ok(())
    })
}
