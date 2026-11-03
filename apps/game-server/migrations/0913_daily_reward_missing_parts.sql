-- P09: complete Inbox delivery and the Store/daily XP duration chain.
INSERT INTO game_character_inbox_cause_kinds(cause_kind) VALUES('DAILY_REWARD')
ON CONFLICT DO NOTHING;

-- The original DAILY-1 item shape predated the named temple-scroll line.
DO $$
DECLARE c RECORD;
BEGIN
    FOR c IN SELECT conname FROM pg_constraint
        WHERE conrelid='game_character_daily_reward_lines'::regclass
          AND contype='c' AND pg_get_constraintdef(oid) LIKE '%kind = ''ITEM''%' LOOP
        EXECUTE format('ALTER TABLE game_character_daily_reward_lines DROP CONSTRAINT %I',c.conname);
    END LOOP;
END $$;
ALTER TABLE game_character_daily_reward_lines ADD CONSTRAINT daily_reward_item_definition CHECK(
    (kind IN('ITEM','TEMPLE_SCROLL'))=(definition IS NOT NULL));
ALTER TABLE game_character_daily_reward_lines ADD CONSTRAINT daily_reward_item_instance CHECK(
    (kind IN('ITEM','TEMPLE_SCROLL') AND status='DELIVERED')=(item_instance_id IS NOT NULL));

CREATE TABLE game_character_store_xp_boosts (
    operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    starts_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL CHECK(expires_at>starts_at),
    percent SMALLINT NOT NULL CHECK(percent=50),
    UNIQUE(character_id,world_id,operation_id)
);
CREATE INDEX game_character_store_xp_boost_active
    ON game_character_store_xp_boosts(character_id,world_id,expires_at DESC);
CREATE TRIGGER game_character_store_xp_boosts_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_character_store_xp_boosts FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
REVOKE ALL ON game_character_store_xp_boosts FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_store_xp_boosts TO oteryn_game_runtime;
GRANT SELECT ON game_character_store_xp_boosts TO oteryn_game_control;
