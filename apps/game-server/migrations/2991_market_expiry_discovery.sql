-- The scheduler holds the returned books until its settlement transaction ends.
-- LockRows skips busy books before Limit counts eligible offers.
CREATE FUNCTION game_market_lock_job_candidates(p_world UUID)
RETURNS SETOF UUID
LANGUAGE sql VOLATILE SECURITY DEFINER AS $$
    SELECT o.offer_id
    FROM game_market_offers o
    JOIN game_market_books book USING (world_id, definition_key)
    LEFT JOIN game_account_bank_balances b ON b.account_id=o.account_id AND b.world_id=o.world_id
    WHERE o.world_id=p_world
      AND (
        (o.state=1 AND (o.matching_pending
          OR o.expires_at_us<=floor(extract(epoch FROM statement_timestamp())*1000000)::bigint))
        OR (o.state=5 AND NOT EXISTS (
          SELECT 1 FROM game_market_operations attempted
          WHERE attempted.occurrence=decode('0102','hex')||uuid_send(o.offer_id)||decode('03','hex')
                ||int8send(floor(extract(epoch FROM statement_timestamp())/86400)::bigint)
        ) AND (o.held_credit_gold+coalesce(b.balance,0)<=9000000000000000 OR EXISTS (
          SELECT 1 FROM game_item_market_escrow_locations e WHERE e.offer_id=o.offer_id
        )))
      )
    ORDER BY o.expires_at_us,o.offer_id
    LIMIT 100
    FOR UPDATE OF book SKIP LOCKED
$$;

DO $$ BEGIN
    EXECUTE format(
        'ALTER FUNCTION game_market_lock_job_candidates(UUID) SET search_path=pg_catalog,%I,pg_temp',
        current_schema()
    );
END $$;
REVOKE ALL ON FUNCTION game_market_lock_job_candidates(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_market_lock_job_candidates(UUID) TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_market_lock_job_candidates(UUID) TO oteryn_game_control;
