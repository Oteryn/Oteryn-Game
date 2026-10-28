-- DAILY4: Premium day five grants the converter and a replay-safe temple teleport receipt.
-- This replacement also admits the Premium/free quantities introduced by DAILY3.
CREATE OR REPLACE FUNCTION game_daily_reward_delivery_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE c public.game_character_daily_reward_claims%ROWTYPE; n INTEGER; total INTEGER; last_line INTEGER; expected INTEGER;
BEGIN
    SELECT * INTO STRICT c FROM public.game_character_daily_reward_claims WHERE transaction_id=NEW.claim_id;
    expected:=CASE c.lane_day
        WHEN 1 THEN CASE WHEN c.premium THEN 10 ELSE 5 END
        WHEN 2 THEN CASE WHEN c.premium THEN 10 ELSE 5 END
        WHEN 3 THEN CASE WHEN c.premium THEN 3 ELSE 1 END
        WHEN 4 THEN CASE WHEN c.premium THEN 20 ELSE 10 END
        WHEN 5 THEN CASE WHEN c.premium THEN 2 ELSE 1 END
        WHEN 6 THEN CASE WHEN c.premium THEN 2 ELSE 1 END
        WHEN 7 THEN CASE WHEN c.premium THEN 30 ELSE 10 END
    END;
    SELECT count(*),sum(quantity),max(line) INTO n,total,last_line
        FROM public.game_character_daily_reward_lines WHERE claim_id=NEW.claim_id;
    IF n<1 OR n>20 OR last_line<>n-1 OR total<>expected
        OR EXISTS(SELECT 1 FROM public.game_character_daily_reward_lines l
            WHERE l.claim_id=NEW.claim_id AND l.kind<>(CASE
                WHEN c.lane_day=3 THEN 'PREY_WILDCARD'
                WHEN c.lane_day=7 THEN 'XP_BOOST'
                WHEN c.lane_day=5 AND c.premium AND l.line=1 THEN 'TEMPLE_TELEPORT'
                ELSE 'ITEM' END)) THEN
        RAISE EXCEPTION 'daily claim requires its complete ordered reward plan' USING ERRCODE='23514';
    END IF;
    IF c.lane_day IN(3,7) AND (n<>1 OR EXISTS(SELECT 1 FROM public.game_character_daily_reward_lines l
        WHERE l.claim_id=c.transaction_id AND (l.status<>'DELIVERED' OR l.delivering_transaction_id<>c.transaction_id
            OR (c.lane_day=7 AND (l.effect_started_at IS NULL OR l.effect_expires_at IS NULL))))
        OR (c.lane_day=3 AND NOT EXISTS(SELECT 1 FROM public.game_account_prey_wildcard_entries e
            JOIN public.game_character_roots r ON r.account_id=e.account_id AND r.world_id=e.world_id
            WHERE e.claim_id=c.transaction_id AND r.character_id=c.character_id AND e.requested=expected))) THEN
        RAISE EXCEPTION 'daily effect requires atomic delivery' USING ERRCODE='23514';
    END IF;
    IF c.jokers_spent<>coalesce((SELECT amount FROM public.game_account_daily_reward_joker_entries WHERE claim_id=c.transaction_id AND kind='SPEND'),0) THEN
        RAISE EXCEPTION 'daily claim joker spend differs from ledger' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_daily_reward_delivery_guard() FROM PUBLIC;
