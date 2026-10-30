-- DAILY30 O1-O5. Existing receipts retain v1 semantics; new claims freeze their
-- server-resolved specification so deferred sink proofs need no second reward table.
ALTER TABLE game_character_daily_rewards DROP CONSTRAINT IF EXISTS game_character_daily_rewards_lane_day_check;
ALTER TABLE game_character_daily_rewards ADD CONSTRAINT game_character_daily_rewards_lane_day_check CHECK (lane_day BETWEEN 1 AND 30);
ALTER TABLE game_character_daily_reward_claims DROP CONSTRAINT IF EXISTS game_character_daily_reward_claims_lane_day_check;
ALTER TABLE game_character_daily_reward_claims ADD CONSTRAINT game_character_daily_reward_claims_lane_day_check CHECK (lane_day BETWEEN 1 AND 30);
ALTER TABLE game_character_daily_reward_claims
    ADD COLUMN IF NOT EXISTS reward_version SMALLINT NOT NULL DEFAULT 1 CHECK (reward_version IN(1,2)),
    ADD COLUMN IF NOT EXISTS reward_spec JSONB;
ALTER TABLE game_character_daily_reward_claims DROP CONSTRAINT IF EXISTS daily_v2_reward_spec;
ALTER TABLE game_character_daily_reward_claims ADD CONSTRAINT daily_v2_reward_spec CHECK ((reward_version=1 AND reward_spec IS NULL) OR
        (reward_version=2 AND reward_spec IS NOT NULL AND NOT (reward_spec @> '[null]'::jsonb) AND jsonb_typeof(reward_spec)='array' AND jsonb_array_length(reward_spec)=6
         AND (reward_spec->>0)::integer BETWEEN 0 AND 10
         AND (reward_spec->>1)::integer BETWEEN 0 AND 2
         AND (reward_spec->>2)::integer BETWEEN 0 AND 1
         AND (reward_spec->>3)::integer IN(0,10,20,25,50)
         AND (reward_spec->>4)::integer IN(0,30,45,60)
         AND ((reward_spec->>3)::integer=0)=((reward_spec->>4)::integer=0)
         AND (reward_spec->>5)::integer=1));
ALTER TABLE game_character_daily_reward_lines DROP CONSTRAINT IF EXISTS game_character_daily_reward_lines_kind_check;
ALTER TABLE game_character_daily_reward_lines ADD CONSTRAINT game_character_daily_reward_lines_kind_check CHECK(kind IN('ITEM','PREY_WILDCARD','XP_BOOST','COSMETIC_TOKEN','BOUNTY_REROLL','TEMPLE_SCROLL'));
-- DEFAULT 50 preserves historical daily boost receipts, without updating history.
ALTER TABLE game_character_daily_reward_lines ADD COLUMN IF NOT EXISTS xp_boost_percent SMALLINT NOT NULL DEFAULT 50 CHECK(xp_boost_percent IN(0,10,20,25,50));
ALTER TABLE game_account_prey_wildcard_entries DROP CONSTRAINT prey_wildcard_entry_values;
ALTER TABLE game_account_prey_wildcard_entries ADD CONSTRAINT prey_wildcard_entry_values CHECK(
 balance_before BETWEEN 0 AND 52 AND balance_after BETWEEN 0 AND 52 AND (previous_claim_id IS NOT NULL OR balance_before=0)
 AND ((kind='DAILY_REWARD' AND requested IN(1,2,3) AND credited=least(requested,52-balance_before) AND balance_after=balance_before+credited)
 OR (kind='SPEND' AND requested BETWEEN 1 AND 15 AND credited=0 AND balance_after=balance_before-requested)));
ALTER TABLE game_party_kill_xp_members DROP CONSTRAINT IF EXISTS game_party_kill_xp_members_xp_boost_percent_check;
ALTER TABLE game_party_kill_xp_members ADD CONSTRAINT game_party_kill_xp_members_xp_boost_percent_check CHECK(xp_boost_percent IN(0,10,20,25,50));
CREATE INDEX IF NOT EXISTS game_daily_v2_xp_boost_lookup ON game_character_daily_reward_claims(character_id,world_id,reset_epoch DESC);

CREATE TABLE IF NOT EXISTS game_character_cosmetic_token_entries (
    claim_id UUID PRIMARY KEY REFERENCES game_character_daily_reward_claims(transaction_id),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    previous_claim_id UUID,
    amount SMALLINT NOT NULL CHECK(amount BETWEEN 1 AND 10),
    balance_before BIGINT NOT NULL CHECK(balance_before>=0),
    balance_after BIGINT NOT NULL CHECK(balance_after>=0 AND balance_after=balance_before+amount),
    CHECK(previous_claim_id IS NOT NULL OR balance_before=0),
    UNIQUE(character_id,world_id,claim_id),
    FOREIGN KEY(character_id,world_id,previous_claim_id) REFERENCES game_character_cosmetic_token_entries(character_id,world_id,claim_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS cosmetic_token_successor ON game_character_cosmetic_token_entries(character_id,world_id,previous_claim_id) NULLS NOT DISTINCT;
CREATE TABLE IF NOT EXISTS game_character_cosmetic_tokens (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    balance BIGINT NOT NULL CHECK(balance>=0),
    last_claim_id UUID NOT NULL,
    PRIMARY KEY(character_id,world_id),
    FOREIGN KEY(character_id,world_id,last_claim_id) REFERENCES game_character_cosmetic_token_entries(character_id,world_id,claim_id)
);
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='cosmetic_token_entries_immutable' AND tgrelid='game_character_cosmetic_token_entries'::regclass) THEN
  CREATE TRIGGER cosmetic_token_entries_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_character_cosmetic_token_entries FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
 END IF;
END $$;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='cosmetic_tokens_no_delete' AND tgrelid='game_character_cosmetic_tokens'::regclass) THEN
  CREATE TRIGGER cosmetic_tokens_no_delete BEFORE DELETE OR TRUNCATE
    ON game_character_cosmetic_tokens FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
 END IF;
END $$;
REVOKE ALL ON game_character_cosmetic_tokens,game_character_cosmetic_token_entries FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_cosmetic_tokens TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_cosmetic_token_entries TO oteryn_game_runtime;
GRANT SELECT ON game_character_cosmetic_tokens,game_character_cosmetic_token_entries TO oteryn_game_control;

CREATE OR REPLACE FUNCTION game_cosmetic_token_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE e public.game_character_cosmetic_token_entries%ROWTYPE; previous BIGINT;
BEGIN
    IF TG_TABLE_NAME='game_character_cosmetic_token_entries' THEN
        IF NEW.previous_claim_id IS NOT NULL THEN
            SELECT balance_after INTO STRICT previous FROM public.game_character_cosmetic_token_entries
                WHERE claim_id=NEW.previous_claim_id AND character_id=NEW.character_id AND world_id=NEW.world_id;
            IF previous<>NEW.balance_before THEN RAISE EXCEPTION 'token ledger discontinuity' USING ERRCODE='23514'; END IF;
        END IF;
        IF NOT EXISTS(SELECT 1 FROM public.game_character_daily_reward_claims c
            JOIN public.game_character_daily_reward_lines l ON l.claim_id=c.transaction_id
            WHERE c.transaction_id=NEW.claim_id AND c.character_id=NEW.character_id AND c.world_id=NEW.world_id
            AND c.reward_version=2 AND l.kind='COSMETIC_TOKEN'
            AND l.quantity=NEW.amount AND l.status='DELIVERED') THEN
            RAISE EXCEPTION 'token entry requires matching daily claim' USING ERRCODE='23514';
        END IF;
    END IF;
    SELECT l.* INTO STRICT e FROM public.game_character_cosmetic_tokens p JOIN public.game_character_cosmetic_token_entries l ON l.claim_id=p.last_claim_id
        WHERE p.character_id=NEW.character_id AND p.world_id=NEW.world_id AND p.balance=l.balance_after;
    IF EXISTS(SELECT 1 FROM public.game_character_cosmetic_token_entries WHERE previous_claim_id=e.claim_id) THEN
        RAISE EXCEPTION 'token balance must name latest entry' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='cosmetic_token_entry_final' AND tgrelid='game_character_cosmetic_token_entries'::regclass) THEN
  CREATE CONSTRAINT TRIGGER cosmetic_token_entry_final AFTER INSERT ON game_character_cosmetic_token_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cosmetic_token_guard();
 END IF;
END $$;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='cosmetic_token_balance_final' AND tgrelid='game_character_cosmetic_tokens'::regclass) THEN
  CREATE CONSTRAINT TRIGGER cosmetic_token_balance_final AFTER INSERT OR UPDATE ON game_character_cosmetic_tokens
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cosmetic_token_guard();
 END IF;
END $$;
REVOKE ALL ON FUNCTION game_cosmetic_token_guard() FROM PUBLIC;

CREATE TABLE IF NOT EXISTS game_character_bounty_reroll_token_entries (
    claim_id UUID PRIMARY KEY REFERENCES game_character_daily_reward_claims(transaction_id),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    previous_claim_id UUID,
    amount SMALLINT NOT NULL CHECK(amount BETWEEN 1 AND 10),
    balance_before BIGINT NOT NULL CHECK(balance_before>=0),
    balance_after BIGINT NOT NULL CHECK(balance_after>=0 AND balance_after=balance_before+amount),
    CHECK(previous_claim_id IS NOT NULL OR balance_before=0),
    UNIQUE(character_id,world_id,claim_id),
    FOREIGN KEY(character_id,world_id,previous_claim_id) REFERENCES game_character_bounty_reroll_token_entries(character_id,world_id,claim_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS bounty_reroll_token_successor ON game_character_bounty_reroll_token_entries(character_id,world_id,previous_claim_id) NULLS NOT DISTINCT;
CREATE TABLE IF NOT EXISTS game_character_bounty_reroll_tokens (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    balance BIGINT NOT NULL CHECK(balance>=0),
    last_claim_id UUID NOT NULL,
    PRIMARY KEY(character_id,world_id),
    FOREIGN KEY(character_id,world_id,last_claim_id) REFERENCES game_character_bounty_reroll_token_entries(character_id,world_id,claim_id)
);
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='bounty_reroll_token_entries_immutable' AND tgrelid='game_character_bounty_reroll_token_entries'::regclass) THEN
  CREATE TRIGGER bounty_reroll_token_entries_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_character_bounty_reroll_token_entries FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
 END IF;
END $$;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='bounty_reroll_tokens_no_delete' AND tgrelid='game_character_bounty_reroll_tokens'::regclass) THEN
  CREATE TRIGGER bounty_reroll_tokens_no_delete BEFORE DELETE OR TRUNCATE
    ON game_character_bounty_reroll_tokens FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
 END IF;
END $$;
REVOKE ALL ON game_character_bounty_reroll_tokens,game_character_bounty_reroll_token_entries FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_bounty_reroll_tokens TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_bounty_reroll_token_entries TO oteryn_game_runtime;
GRANT SELECT ON game_character_bounty_reroll_tokens,game_character_bounty_reroll_token_entries TO oteryn_game_control;

CREATE OR REPLACE FUNCTION game_bounty_reroll_token_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE e public.game_character_bounty_reroll_token_entries%ROWTYPE; previous BIGINT;
BEGIN
    IF TG_TABLE_NAME='game_character_bounty_reroll_token_entries' THEN
        IF NEW.previous_claim_id IS NOT NULL THEN
            SELECT balance_after INTO STRICT previous FROM public.game_character_bounty_reroll_token_entries
                WHERE claim_id=NEW.previous_claim_id AND character_id=NEW.character_id AND world_id=NEW.world_id;
            IF previous<>NEW.balance_before THEN RAISE EXCEPTION 'token ledger discontinuity' USING ERRCODE='23514'; END IF;
        END IF;
        IF NOT EXISTS(SELECT 1 FROM public.game_character_daily_reward_claims c
            JOIN public.game_character_daily_reward_lines l ON l.claim_id=c.transaction_id
            WHERE c.transaction_id=NEW.claim_id AND c.character_id=NEW.character_id AND c.world_id=NEW.world_id
            AND c.reward_version=2 AND l.kind='BOUNTY_REROLL'
            AND l.quantity=NEW.amount AND l.status='DELIVERED') THEN
            RAISE EXCEPTION 'token entry requires matching daily claim' USING ERRCODE='23514';
        END IF;
    END IF;
    SELECT l.* INTO STRICT e FROM public.game_character_bounty_reroll_tokens p JOIN public.game_character_bounty_reroll_token_entries l ON l.claim_id=p.last_claim_id
        WHERE p.character_id=NEW.character_id AND p.world_id=NEW.world_id AND p.balance=l.balance_after;
    IF EXISTS(SELECT 1 FROM public.game_character_bounty_reroll_token_entries WHERE previous_claim_id=e.claim_id) THEN
        RAISE EXCEPTION 'token balance must name latest entry' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='bounty_reroll_token_entry_final' AND tgrelid='game_character_bounty_reroll_token_entries'::regclass) THEN
  CREATE CONSTRAINT TRIGGER bounty_reroll_token_entry_final AFTER INSERT ON game_character_bounty_reroll_token_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_bounty_reroll_token_guard();
 END IF;
END $$;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgname='bounty_reroll_token_balance_final' AND tgrelid='game_character_bounty_reroll_tokens'::regclass) THEN
  CREATE CONSTRAINT TRIGGER bounty_reroll_token_balance_final AFTER INSERT OR UPDATE ON game_character_bounty_reroll_tokens
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_bounty_reroll_token_guard();
 END IF;
END $$;
REVOKE ALL ON FUNCTION game_bounty_reroll_token_guard() FROM PUBLIC;

CREATE OR REPLACE FUNCTION game_daily_reward_projection_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
DECLARE
    state public.game_character_daily_rewards%ROWTYPE;
    claim public.game_character_daily_reward_claims%ROWTYPE;
BEGIN
    SELECT * INTO state FROM public.game_character_daily_rewards
        WHERE character_id = NEW.character_id AND world_id = NEW.world_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'daily reward state missing' USING ERRCODE = '23514';
    END IF;
    SELECT * INTO claim FROM public.game_character_daily_reward_claims
        WHERE character_id = NEW.character_id AND world_id = NEW.world_id
        ORDER BY reset_epoch DESC LIMIT 1;
    IF FOUND THEN
        IF state.last_claim_epoch IS DISTINCT FROM claim.reset_epoch
            OR state.last_claim_day IS DISTINCT FROM claim.reset_day
            OR state.streak <> claim.streak_after
            OR state.lane_day <> (CASE WHEN claim.lane_day = CASE WHEN claim.reward_version=2 THEN 30 ELSE 7 END THEN 1 ELSE claim.lane_day + 1 END) THEN
            RAISE EXCEPTION 'daily reward projection mismatch' USING ERRCODE = '23514';
        END IF;
    ELSIF state.last_claim_epoch IS NOT NULL THEN
        RAISE EXCEPTION 'daily reward claim missing' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END $$;

-- Extend the live guard, preserving PREY-0's same-transaction SPEND proof.
DO $$ DECLARE body TEXT; needle TEXT := 'c.lane_day=3 AND NEW.requested=CASE WHEN c.premium THEN 3 ELSE 1 END'; BEGIN
 SELECT pg_get_functiondef('game_daily_wildcard_guard()'::regprocedure) INTO STRICT body;
 IF position('c.reward_version=2' IN body)>0 THEN RETURN; END IF;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing DAILY wildcard proof'; END IF;
 EXECUTE replace(body,needle,'((c.reward_version=1 AND c.lane_day=3 AND NEW.requested=CASE WHEN c.premium THEN 3 ELSE 1 END) OR (c.reward_version=2 AND NEW.requested=(c.reward_spec->>1)::integer))');
END $$;

CREATE OR REPLACE FUNCTION game_daily_reward_delivery_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE c public.game_character_daily_reward_claims%ROWTYPE; n INTEGER; total INTEGER; last_line INTEGER; expected INTEGER;
BEGIN
    SELECT * INTO STRICT c FROM public.game_character_daily_reward_claims WHERE transaction_id=NEW.claim_id;
    IF c.reward_version=2 THEN
        SELECT count(*),max(line) INTO n,last_line FROM public.game_character_daily_reward_lines WHERE claim_id=c.transaction_id;
        expected:=1 + CASE WHEN (c.reward_spec->>0)::integer>0 THEN 1 ELSE 0 END
                    + CASE WHEN (c.reward_spec->>1)::integer>0 THEN 1 ELSE 0 END
                    + CASE WHEN (c.reward_spec->>2)::integer>0 THEN 1 ELSE 0 END
                    + CASE WHEN (c.reward_spec->>4)::integer>0 THEN 1 ELSE 0 END;
        IF n<>expected OR last_line<>n-1 OR EXISTS(
            SELECT 1 FROM public.game_character_daily_reward_lines l WHERE l.claim_id=c.transaction_id
            AND (l.quantity<>CASE l.kind
                WHEN 'COSMETIC_TOKEN' THEN (c.reward_spec->>0)::integer
                WHEN 'PREY_WILDCARD' THEN (c.reward_spec->>1)::integer
                WHEN 'TEMPLE_SCROLL' THEN (c.reward_spec->>2)::integer
                WHEN 'XP_BOOST' THEN (c.reward_spec->>4)::integer
                WHEN 'BOUNTY_REROLL' THEN 1 ELSE -1 END
                OR (l.kind='TEMPLE_SCROLL' AND l.status<>'PENDING')
                OR (l.kind<>'TEMPLE_SCROLL' AND (l.status<>'DELIVERED' OR l.delivering_transaction_id<>c.transaction_id))
                OR (l.kind='XP_BOOST' AND (l.effect_started_at IS NULL OR l.effect_expires_at IS NULL OR l.xp_boost_percent<>(c.reward_spec->>3)::integer))))
            OR EXISTS(SELECT kind FROM public.game_character_daily_reward_lines WHERE claim_id=c.transaction_id GROUP BY kind HAVING count(*)<>1)
            OR NOT EXISTS(SELECT 1 FROM public.game_character_bounty_reroll_token_entries WHERE claim_id=c.transaction_id)
            OR ((c.reward_spec->>0)::integer>0 AND NOT EXISTS(SELECT 1 FROM public.game_character_cosmetic_token_entries WHERE claim_id=c.transaction_id))
            OR ((c.reward_spec->>1)::integer>0 AND NOT EXISTS(SELECT 1 FROM public.game_account_prey_wildcard_entries WHERE claim_id=c.transaction_id)) THEN
            RAISE EXCEPTION 'daily v2 requires complete atomic reward plan' USING ERRCODE='23514';
        END IF;
        IF c.jokers_spent<>coalesce((SELECT amount FROM public.game_account_daily_reward_joker_entries WHERE claim_id=c.transaction_id AND kind='SPEND'),0) THEN
            RAISE EXCEPTION 'daily claim joker spend differs from ledger' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
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

