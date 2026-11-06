    for (stage, table, condition, command) in [
        ("debit", "game_item_instances", "NEW.definition_production_key='oteryn:item.tibia.i19151' AND EXISTS(SELECT 1 FROM game_npc_trades WHERE transaction_id=NEW.minted_transaction_id AND side=3)", 9997),
        ("mint", "game_item_audit_outbox", "EXISTS(SELECT 1 FROM game_npc_trades WHERE transaction_id=NEW.transaction_id AND side=3)", 9998),
    ] {
        sqlx::query("CREATE SEQUENCE fixture_token_failure_seen").execute(&h.pool).await?;
        sqlx::query("GRANT USAGE,SELECT ON SEQUENCE fixture_token_failure_seen TO oteryn_game_runtime").execute(&h.pool).await?;
        let function=format!("CREATE FUNCTION fixture_token_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF {condition} THEN PERFORM nextval('fixture_token_failure_seen'); RAISE EXCEPTION 'injected token exchange failure after {stage}'; END IF; RETURN NEW; END $$");
        sqlx::query(&function).execute(&h.pool).await?;
        sqlx::query(&format!("CREATE TRIGGER fixture_token_fail BEFORE INSERT ON {table} FOR EACH ROW EXECUTE FUNCTION fixture_token_fail()"))
            .execute(&h.pool).await?;
        let failed=send(owner,admitted,jorge,command,"abacus").await?;
        assert!(failed.unknown,"storage failure must not acknowledge a trade after {stage}");
        assert!(sqlx::query_scalar::<_,bool>("SELECT is_called FROM fixture_token_failure_seen").fetch_one(&h.pool).await?,"fault reached after {stage}");
        let state:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19083'),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19151'),(SELECT count(*) FROM game_npc_trades WHERE side=3)")
            .fetch_one(&h.pool).await?;
        assert_eq!(state,(20,0,0),"atomic rollback after {stage}");
        sqlx::query(&format!("DROP TRIGGER fixture_token_fail ON {table}")).execute(&h.pool).await?;
        sqlx::query("DROP FUNCTION fixture_token_fail()").execute(&h.pool).await?;
        sqlx::query("DROP SEQUENCE fixture_token_failure_seen").execute(&h.pool).await?;
    }
