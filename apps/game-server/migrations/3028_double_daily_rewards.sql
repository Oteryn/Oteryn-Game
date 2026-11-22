-- Double Daily Rewards (Tibia news 8683). The claim freezes the awarded tier
-- in reward_spec. Legacy rows and the previous delivery guards remain valid.
ALTER TABLE game_character_daily_reward_claims DROP CONSTRAINT daily_v2_reward_spec;
ALTER TABLE game_character_daily_reward_claims ADD CONSTRAINT daily_v2_reward_spec CHECK (
    (reward_version=1 AND reward_spec IS NULL) OR
    (reward_version=2 AND reward_spec IS NOT NULL AND NOT (reward_spec @> '[null]'::jsonb)
        AND jsonb_typeof(reward_spec)='array' AND jsonb_array_length(reward_spec)=6
        AND (reward_spec->>5)::integer IN (1,2)
        AND (reward_spec->>0)::integer BETWEEN 0 AND 10*(reward_spec->>5)::integer
        AND (reward_spec->>1)::integer BETWEEN 0 AND 2*(reward_spec->>5)::integer
        AND (reward_spec->>2)::integer BETWEEN 0 AND (reward_spec->>5)::integer
        AND (reward_spec->>0)::integer % nullif((reward_spec->>5)::integer,0)=0
        AND (reward_spec->>1)::integer % nullif((reward_spec->>5)::integer,0)=0
        AND (reward_spec->>2)::integer % nullif((reward_spec->>5)::integer,0)=0
        AND (reward_spec->>3)::integer IN (0,10,20,25,50)
        AND (reward_spec->>4)::integer IN (0,30*(reward_spec->>5)::integer,
            45*(reward_spec->>5)::integer,60*(reward_spec->>5)::integer)
        AND ((reward_spec->>3)::integer=0)=((reward_spec->>4)::integer=0)
    )
);

ALTER TABLE game_character_daily_reward_lines
    DROP CONSTRAINT game_character_daily_reward_lines_quantity_check,
    ADD CONSTRAINT game_character_daily_reward_lines_quantity_check
        CHECK (quantity BETWEEN 1 AND CASE WHEN kind='XP_BOOST' THEN 120 ELSE 100 END);

ALTER TABLE game_character_cosmetic_token_entries
    DROP CONSTRAINT game_character_cosmetic_token_entries_amount_check,
    ADD CONSTRAINT game_character_cosmetic_token_entries_amount_check CHECK (amount BETWEEN 1 AND 20);

-- Keep the Store branch and let game_daily_wildcard_guard enforce exact credit
-- against any pending Store reservation, as installed by migration 3025.
ALTER TABLE game_account_prey_wildcard_entries DROP CONSTRAINT prey_wildcard_entry_values;
ALTER TABLE game_account_prey_wildcard_entries ADD CONSTRAINT prey_wildcard_entry_values CHECK (
    balance_before BETWEEN 0 AND 52 AND balance_after BETWEEN 0 AND 52
    AND (previous_claim_id IS NOT NULL OR balance_before=0)
    AND ((kind='DAILY_REWARD' AND requested IN(1,2,3,4)
          AND credited BETWEEN 0 AND requested AND balance_after=balance_before+credited)
      OR (kind='SPEND' AND requested BETWEEN 1 AND 15 AND credited=0
          AND balance_after=balance_before-requested)
      OR (kind='STORE' AND requested IN (5,20) AND credited=requested
          AND balance_after=balance_before+credited AND balance_after<=50))
);

-- Each nonstackable scroll has its own existing line proof and expiry. All
-- other reward kinds still require exactly one line when present.
DO $migration$
DECLARE
    body TEXT := pg_get_functiondef('game_daily_reward_delivery_guard()'::regprocedure);
    old_count CONSTANT TEXT := 'CASE WHEN (c.reward_spec->>2)::integer>0 THEN 1 ELSE 0 END';
    old_scroll CONSTANT TEXT := 'WHEN ''TEMPLE_SCROLL'' THEN (c.reward_spec->>2)::integer';
    old_bounty CONSTANT TEXT := 'WHEN ''BOUNTY_REROLL'' THEN 1';
    old_kinds CONSTANT TEXT := 'GROUP BY kind HAVING count(*)<>1';
    old_duration CONSTANT TEXT := 'l.effect_started_at IS NULL OR l.effect_expires_at IS NULL';
BEGIN
    IF strpos(body,old_count)=0 OR strpos(body,old_scroll)=0 OR strpos(body,old_bounty)=0
        OR strpos(body,old_kinds)=0 OR strpos(body,old_duration)=0 THEN
        RAISE EXCEPTION 'unexpected daily reward delivery guard shape';
    END IF;
    body:=replace(body,old_count,'(c.reward_spec->>2)::integer');
    body:=replace(body,old_scroll,'WHEN ''TEMPLE_SCROLL'' THEN 1');
    body:=replace(body,old_bounty,'WHEN ''BOUNTY_REROLL'' THEN (c.reward_spec->>5)::integer');
    body:=replace(body,old_kinds,
        'GROUP BY kind HAVING count(*)<>CASE WHEN kind=''TEMPLE_SCROLL'' THEN (c.reward_spec->>2)::integer ELSE 1 END'
        || ' OR (kind=''TEMPLE_SCROLL'' AND count(DISTINCT item_instance_id)<>count(*))');
    -- The same duration is recorded on the line and used by the expiry reader.
    body:=regexp_replace(body,old_duration,
        old_duration||' OR l.effect_expires_at-l.effect_started_at<>make_interval(mins=>l.quantity)');
    EXECUTE body;
END $migration$;
