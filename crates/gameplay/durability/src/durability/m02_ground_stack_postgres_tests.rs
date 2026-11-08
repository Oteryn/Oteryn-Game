mod m02_ground_stacks {
    use super::*;
    use crate::durability::quest_world_callback::matches_ground_stack_for_test as matches;
    use oteryn_gameplay_quests::quest::world_callbacks::GroundStackEntry;

    #[test]
    fn m02_ground_stack_postgres_exact_order_quantity_and_scope() -> TestResult {
        std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
            run("encx_m02_ground_stack", async |h| {
                let started = std::time::Instant::now();
                let at = target();
                let gold = TypedDefinitionRef { production_key: "oteryn:item.tibia.i3031".into(), ..definition() };
                mint_quantity_with_definition(h, 1000, 1, &at, 2, gold.clone()).await?;
                let mut elsewhere = at.clone();
                elsewhere.spatial_position[0] ^= 1;
                mint_quantity_with_definition(h, 1001, 1, &elsewhere, 9, gold.clone()).await?;
                mint_quantity_with_definition(h, 1002, 1, &at, 1, gold.clone()).await?;
                let item = gold.production_key;
                let expected = vec![GroundStackEntry { item:item.clone(),count:2 },
                    GroundStackEntry { item:item.clone(),count:1 }];
                let mut tx = h.pool.begin().await?;
                let (_permit, scope) = authority(&mut tx,h).await?;
                assert!(matches(&mut tx,&scope,&at,&expected).await?);
                // Physical ordinals need not be consecutive: the unrelated drop
                // between these two roots must not disturb their relative order.
                let mut reversed = expected.clone(); reversed.reverse();
                assert!(!matches(&mut tx,&scope,&at,&reversed).await?);
                let mut wrong_quantity=expected.clone();wrong_quantity[0].count=1;
                assert!(!matches(&mut tx,&scope,&at,&wrong_quantity).await?);
                let mut wrong_identity=expected.clone();wrong_identity[1].item="oteryn:item.tibia.i3035".into();
                assert!(!matches(&mut tx,&scope,&at,&wrong_identity).await?);
                assert!(!matches(&mut tx,&scope,&at,&expected[..1]).await?);
                for case in 0..4 {
                    let mut wrong=at.clone();
                    match case {
                        0=>wrong.map_revision="foreign-map".into(),
                        1=>wrong.content_revision="foreign-content".into(),
                        2=>wrong.placement_context[0]^=1,
                        _=>wrong.spatial_position[0]^=2,
                    }
                    assert!(!matches(&mut tx,&scope,&wrong,&expected).await?);
                }
                let mut concurrent=h.pool.begin().await?;
                let acquired:bool=sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended(encode($1,'hex')||encode($2,'hex'),33))")
                    .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice())
                    .fetch_one(&mut *concurrent).await?;
                assert!(!acquired,"Ground owner lock prevents a concurrent phantom drop");
                concurrent.rollback().await?;
                tx.commit().await?;
                let mut different_transaction = h.pool.begin().await?;
                assert!(matches(&mut different_transaction, &scope, &at, &expected).await.is_err(),
                    "a prior transaction's scope token cannot authorize a new Ground readset");
                different_transaction.rollback().await?;
                assert_eq!(h.count("game_item_instances").await?,3);
                assert_eq!(h.count("game_item_ground_locations").await?,3);
                eprintln!("M02 real PostgreSQL exact Ground stack, order/quantity/custody/context and phantom lock: {:?}",started.elapsed());
                Ok(())
            }).map_err(|e|e.to_string())
        })?.join().map_err(|_|"M02 PostgreSQL worker panicked")?.map_err(Into::into)
    }
}
