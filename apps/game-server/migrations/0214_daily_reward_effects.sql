-- DAILYFX: extend DAILY-1's atomic claim and PREY's Account + World resource.
-- No historical pending line is delivered implicitly during deployment.
ALTER TABLE game_character_daily_reward_claims
    DROP CONSTRAINT game_character_daily_reward_claims_premium_check;
ALTER TABLE game_character_daily_reward_lines
    ADD COLUMN effect_started_at TIMESTAMPTZ,
    ADD COLUMN effect_expires_at TIMESTAMPTZ,
    ADD CONSTRAINT daily_effect_interval CHECK (
        (effect_started_at IS NULL AND effect_expires_at IS NULL)
        OR (kind='XP_BOOST' AND status='DELIVERED'
            AND effect_started_at IS NOT NULL AND effect_expires_at IS NOT NULL
            AND effect_expires_at=effect_started_at+make_interval(mins=>quantity)));

-- A single server clock domain for grant and use. No client or login clock input.
CREATE FUNCTION game_daily_effect_now() RETURNS TIMESTAMPTZ
LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $$ SELECT clock_timestamp() $$;
REVOKE ALL ON FUNCTION game_daily_effect_now() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_daily_effect_now() TO oteryn_game_runtime;

CREATE TABLE game_account_prey_wildcard_entries (
    claim_id UUID PRIMARY KEY REFERENCES game_character_daily_reward_claims(transaction_id),
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    world_id UUID NOT NULL,
    previous_claim_id UUID,
    kind TEXT NOT NULL CHECK(kind='DAILY_REWARD'),
    requested SMALLINT NOT NULL CHECK(requested IN(1,3)),
    credited SMALLINT NOT NULL CHECK(credited BETWEEN 0 AND 3),
    balance_before SMALLINT NOT NULL CHECK(balance_before BETWEEN 0 AND 52),
    balance_after SMALLINT NOT NULL CHECK(balance_after BETWEEN 0 AND 52),
    UNIQUE(account_id,world_id,claim_id),
    FOREIGN KEY(account_id,world_id,previous_claim_id)
        REFERENCES game_account_prey_wildcard_entries(account_id,world_id,claim_id),
    CHECK(balance_after=balance_before+credited),
    CHECK(credited=least(requested,52-balance_before)),
    CHECK(previous_claim_id IS NOT NULL OR balance_before=0)
);
CREATE UNIQUE INDEX game_prey_wildcard_successor
    ON game_account_prey_wildcard_entries(account_id,world_id,previous_claim_id) NULLS NOT DISTINCT;
CREATE TABLE game_account_prey_wildcards (
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    world_id UUID NOT NULL,
    balance SMALLINT NOT NULL CHECK(balance BETWEEN 0 AND 52),
    last_claim_id UUID NOT NULL,
    PRIMARY KEY(account_id,world_id),
    FOREIGN KEY(account_id,world_id,last_claim_id)
        REFERENCES game_account_prey_wildcard_entries(account_id,world_id,claim_id)
);
CREATE TRIGGER prey_wildcard_entries_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_account_prey_wildcard_entries FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE TRIGGER prey_wildcards_no_delete BEFORE DELETE OR TRUNCATE
    ON game_account_prey_wildcards FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();

CREATE FUNCTION game_daily_wildcard_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE e public.game_account_prey_wildcard_entries%ROWTYPE; previous SMALLINT;
BEGIN
    IF TG_TABLE_NAME='game_account_prey_wildcard_entries' THEN
        IF NEW.previous_claim_id IS NOT NULL THEN
            SELECT balance_after INTO STRICT previous FROM public.game_account_prey_wildcard_entries
                WHERE claim_id=NEW.previous_claim_id AND account_id=NEW.account_id AND world_id=NEW.world_id;
            IF previous<>NEW.balance_before THEN
                RAISE EXCEPTION 'prey wildcard ledger discontinuity' USING ERRCODE='23514';
            END IF;
        END IF;
        IF NOT EXISTS(SELECT 1 FROM public.game_character_daily_reward_claims c
            JOIN public.game_character_roots r ON r.character_id=c.character_id
            JOIN public.game_character_daily_reward_lines l ON l.claim_id=c.transaction_id
            WHERE c.transaction_id=NEW.claim_id AND r.account_id=NEW.account_id AND c.world_id=NEW.world_id
                AND c.lane_day=3 AND NEW.requested=CASE WHEN c.premium THEN 3 ELSE 1 END
                AND l.kind='PREY_WILDCARD' AND l.status='DELIVERED' AND l.quantity=NEW.requested) THEN
            RAISE EXCEPTION 'prey wildcard requires matching DAILY grant' USING ERRCODE='23514';
        END IF;
    END IF;
    SELECT l.* INTO STRICT e FROM public.game_account_prey_wildcards p
        JOIN public.game_account_prey_wildcard_entries l ON l.claim_id=p.last_claim_id
        WHERE p.account_id=NEW.account_id AND p.world_id=NEW.world_id AND p.balance=l.balance_after;
    IF EXISTS(SELECT 1 FROM public.game_account_prey_wildcard_entries
        WHERE previous_claim_id=e.claim_id AND account_id=e.account_id AND world_id=e.world_id) THEN
        RAISE EXCEPTION 'prey wildcard projection must name latest entry' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER daily_wildcard_entry_final AFTER INSERT ON game_account_prey_wildcard_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_daily_wildcard_guard();
CREATE CONSTRAINT TRIGGER daily_wildcard_balance_final AFTER INSERT OR UPDATE ON game_account_prey_wildcards
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_daily_wildcard_guard();
REVOKE ALL ON FUNCTION game_daily_wildcard_guard() FROM PUBLIC;
REVOKE ALL ON game_account_prey_wildcards,game_account_prey_wildcard_entries FROM PUBLIC;
GRANT SELECT,INSERT ON game_account_prey_wildcards,game_account_prey_wildcard_entries TO oteryn_game_runtime;
GRANT UPDATE(balance,last_claim_id) ON game_account_prey_wildcards TO oteryn_game_runtime;

-- Keep DAILY-1's complete plan and joker proofs; only the two effect sinks change.
CREATE OR REPLACE FUNCTION game_daily_reward_delivery_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE c public.game_character_daily_reward_claims%ROWTYPE; n INTEGER; total INTEGER; last_line INTEGER; expected INTEGER;
BEGIN
    SELECT * INTO STRICT c FROM public.game_character_daily_reward_claims WHERE transaction_id=NEW.claim_id;
    expected:=CASE c.lane_day WHEN 1 THEN 5 WHEN 2 THEN 5 WHEN 3 THEN CASE WHEN c.premium THEN 3 ELSE 1 END
        WHEN 4 THEN 10 WHEN 7 THEN CASE WHEN c.premium THEN 30 ELSE 10 END ELSE 1 END;
    SELECT count(*),sum(quantity),max(line) INTO n,total,last_line FROM public.game_character_daily_reward_lines WHERE claim_id=NEW.claim_id;
    IF n<1 OR n>20 OR last_line<>n-1 OR total<>expected
        OR EXISTS(SELECT 1 FROM public.game_character_daily_reward_lines WHERE claim_id=NEW.claim_id
            AND kind<>(CASE c.lane_day WHEN 3 THEN 'PREY_WILDCARD' WHEN 7 THEN 'XP_BOOST' ELSE 'ITEM' END)) THEN
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

-- Every personal kill and party share uses the same audited modifier path.
ALTER TABLE game_party_kill_xp_members ADD COLUMN xp_boost_percent SMALLINT NOT NULL DEFAULT 0
    CHECK(xp_boost_percent IN(0,50));
CREATE INDEX game_daily_xp_boost_lookup ON game_character_daily_reward_claims(character_id,world_id,reset_epoch DESC)
    WHERE lane_day=7;
