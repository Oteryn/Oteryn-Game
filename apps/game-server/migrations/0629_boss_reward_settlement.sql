-- W03: immutable boss death/contribution receipts and personal, seven-day reward entitlements.
-- Physical ItemInstance custody is minted only by the existing reward-claim owner on collection.
CREATE TABLE game_boss_reward_deaths (
    world_id UUID NOT NULL,
    death_id UUID NOT NULL,
    scope_kind SMALLINT NOT NULL CHECK (scope_kind IN (1,2)),
    channel_id UUID,
    instance_id UUID,
    scope_generation NUMERIC(20,0) NOT NULL CHECK (scope_generation>0),
    boss_key TEXT NOT NULL CHECK (octet_length(boss_key) BETWEEN 1 AND 512),
    minted_at BIGINT NOT NULL CHECK (minted_at>0),
    expires_at BIGINT NOT NULL CHECK (expires_at=minted_at+604800000),
    participant_count INTEGER NOT NULL CHECK (participant_count BETWEEN 0 AND 50),
    eligible_count INTEGER NOT NULL CHECK (eligible_count BETWEEN 0 AND 16 AND eligible_count<=participant_count),
    -- 1: awarded; 2: actual native death after the exact private room deadline.
    -- A late death is terminal, retaining its census without awarding a chest.
    settlement_outcome SMALLINT NOT NULL DEFAULT 1 CHECK (settlement_outcome IN (1,2)),
    intent BYTEA NOT NULL CHECK (octet_length(intent)>0),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(world_id,death_id),
    CHECK ((scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
        OR (scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL)),
    CHECK (settlement_outcome=1 OR scope_kind=2)
);
CREATE TABLE game_boss_reward_contributors (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    death_id UUID NOT NULL,
    source_game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    source_lease_generation NUMERIC(20,0) NOT NULL CHECK (source_lease_generation>0),
    damage_dealt NUMERIC(20,0) NOT NULL CHECK (damage_dealt>=0),
    damage_received NUMERIC(20,0) NOT NULL CHECK (damage_received>=0),
    healing_others NUMERIC(20,0) NOT NULL CHECK (healing_others>=0),
    eligible BOOLEAN NOT NULL,
    item_count INTEGER NOT NULL CHECK (item_count BETWEEN 0 AND 1000 AND (eligible OR item_count=0)),
    PRIMARY KEY(character_id,world_id,death_id),
    FOREIGN KEY(world_id,death_id) REFERENCES game_boss_reward_deaths(world_id,death_id)
);
CREATE INDEX game_boss_reward_contributors_death ON game_boss_reward_contributors(world_id,death_id,character_id);
CREATE INDEX game_boss_reward_contributors_session ON game_boss_reward_contributors(source_game_session_id);
CREATE TABLE game_boss_reward_items (
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    death_id UUID NOT NULL,
    item_ordinal INTEGER NOT NULL CHECK (item_ordinal BETWEEN 1 AND 1000),
    definition_family TEXT NOT NULL CHECK (definition_family='Item'),
    definition_production_key TEXT NOT NULL CHECK (octet_length(definition_production_key) BETWEEN 1 AND 512),
    definition_revision_ref TEXT NOT NULL CHECK (octet_length(definition_revision_ref) BETWEEN 1 AND 512),
    quantity BIGINT NOT NULL CHECK (quantity BETWEEN 1 AND 4294967295),
    collected_transaction_id UUID,
    expired BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY(character_id,world_id,death_id,item_ordinal),
    FOREIGN KEY(character_id,world_id,death_id) REFERENCES game_boss_reward_contributors(character_id,world_id,death_id),
    CHECK (NOT (expired AND collected_transaction_id IS NOT NULL))
);
CREATE INDEX game_boss_reward_items_live ON game_boss_reward_items(character_id,world_id,death_id,item_ordinal)
    WHERE collected_transaction_id IS NULL AND NOT expired;

CREATE FUNCTION game_boss_reward_creation_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE d game_boss_reward_deaths%ROWTYPE;
BEGIN
    SELECT * INTO d FROM game_boss_reward_deaths WHERE world_id=NEW.world_id AND death_id=NEW.death_id;
    IF d.death_id IS NULL OR d.created_xact_id<>pg_current_xact_id() THEN
        RAISE EXCEPTION 'boss reward creation requires same-transaction death' USING ERRCODE='23514';
    END IF;
    IF TG_TABLE_NAME='game_boss_reward_contributors' THEN
    IF NOT EXISTS (
        SELECT 1 FROM game_character_roots r JOIN game_durability_reconnect_sessions s ON s.character_id=r.character_id
        JOIN game_durability_session_use_memberships m ON m.character_id=r.character_id AND m.game_session_id=s.game_session_id
        WHERE r.character_id=NEW.character_id AND r.world_id=NEW.world_id
            AND s.game_session_id=NEW.source_game_session_id AND s.world_id=NEW.world_id
            AND s.character_lease_generation=NEW.source_lease_generation
            AND m.operation_binding IS NOT NULL
    ) THEN RAISE EXCEPTION 'boss reward participant issuance mismatch' USING ERRCODE='23514'; END IF;
    END IF;
    RETURN NEW;
END $$;
CREATE FUNCTION game_boss_reward_death_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE c game_boss_reward_contributors%ROWTYPE;
    total_dealt NUMERIC; total_received NUMERIC; total_healed NUMERIC;
    damage_dealers BIGINT; score_sum NUMERIC; earned BOOLEAN;
BEGIN
    IF (SELECT count(*) FROM game_boss_reward_contributors WHERE world_id=NEW.world_id AND death_id=NEW.death_id)<>NEW.participant_count THEN
        RAISE EXCEPTION 'incomplete boss reward participant census' USING ERRCODE='23514';
    END IF;
    IF (SELECT count(*) FROM game_boss_reward_contributors WHERE world_id=NEW.world_id AND death_id=NEW.death_id AND eligible)<>NEW.eligible_count THEN
        RAISE EXCEPTION 'incomplete eligible boss reward census' USING ERRCODE='23514';
    END IF;
    SELECT coalesce(sum(damage_dealt),0),coalesce(sum(damage_received),0),coalesce(sum(healing_others),0),count(*) FILTER (WHERE damage_dealt>0)
      INTO total_dealt,total_received,total_healed,damage_dealers
      FROM game_boss_reward_contributors WHERE world_id=NEW.world_id AND death_id=NEW.death_id;
    FOR c IN SELECT * FROM game_boss_reward_contributors WHERE world_id=NEW.world_id AND death_id=NEW.death_id LOOP
        -- The accepted 2026-10-27 anti-leech rule uses fixed-point, all-census denominators.
        score_sum:=coalesce(trunc(c.damage_dealt*1000000000/nullif(total_dealt,0)),0)
            +coalesce(trunc(c.damage_received*1000000000/nullif(total_received,0)),0)
            +coalesce(trunc(c.healing_others*1000000000/nullif(total_healed,0)),0);
        earned:=c.damage_dealt>0 AND score_sum*100*damage_dealers>=10*3*1000000000::numeric;
        IF c.eligible<>earned THEN
            RAISE EXCEPTION 'boss reward eligibility does not match complete census' USING ERRCODE='23514';
        END IF;
        IF NOT c.eligible OR NEW.settlement_outcome=2 THEN
            IF EXISTS (SELECT 1 FROM game_boss_reward_items i WHERE (i.character_id,i.world_id,i.death_id)=(c.character_id,c.world_id,c.death_id))
              OR EXISTS (SELECT 1 FROM game_character_boss_reward_chests p WHERE (p.character_id,p.world_id,p.death_id)=(c.character_id,c.world_id,c.death_id))
              OR c.item_count<>0 THEN
                RAISE EXCEPTION 'ineligible or late contributor received personal reward' USING ERRCODE='23514';
            END IF;
            CONTINUE;
        END IF;
        IF (SELECT count(*) FROM game_boss_reward_items i WHERE (i.character_id,i.world_id,i.death_id)=(c.character_id,c.world_id,c.death_id))<>c.item_count
           OR NOT EXISTS (SELECT 1 FROM game_character_boss_reward_chests p
              WHERE (p.character_id,p.world_id,p.death_id)=(c.character_id,c.world_id,c.death_id)
                AND p.boss_key=NEW.boss_key AND p.minted_at=NEW.minted_at AND p.expires_at=NEW.expires_at
                AND p.item_count=(SELECT count(*) FROM game_boss_reward_items i
                    WHERE (i.character_id,i.world_id,i.death_id)=(c.character_id,c.world_id,c.death_id)
                      AND NOT i.expired AND i.collected_transaction_id IS NULL)) THEN
            RAISE EXCEPTION 'incomplete personal boss reward group' USING ERRCODE='23514';
        END IF;
    END LOOP;
    RETURN NULL;
END $$;
CREATE TRIGGER game_boss_reward_deaths_stamp BEFORE INSERT ON game_boss_reward_deaths
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE CONSTRAINT TRIGGER game_boss_reward_deaths_complete AFTER INSERT ON game_boss_reward_deaths
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_boss_reward_death_complete();
CREATE TRIGGER game_boss_reward_deaths_immutable BEFORE UPDATE OR DELETE ON game_boss_reward_deaths
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_boss_reward_contributors_create BEFORE INSERT ON game_boss_reward_contributors
    FOR EACH ROW EXECUTE FUNCTION game_boss_reward_creation_guard();
CREATE TRIGGER game_boss_reward_contributors_immutable BEFORE UPDATE OR DELETE ON game_boss_reward_contributors
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_boss_reward_items_create BEFORE INSERT ON game_boss_reward_items
    FOR EACH ROW EXECUTE FUNCTION game_boss_reward_creation_guard();
CREATE TRIGGER game_boss_reward_deaths_no_truncate BEFORE TRUNCATE ON game_boss_reward_deaths EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_boss_reward_contributors_no_truncate BEFORE TRUNCATE ON game_boss_reward_contributors EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_boss_reward_items_no_truncate BEFORE TRUNCATE ON game_boss_reward_items EXECUTE FUNCTION game_item_reject_truncate();
DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_boss_reward_creation_guard','game_boss_reward_death_complete'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
    END LOOP;
END $$;
REVOKE ALL ON game_boss_reward_deaths,game_boss_reward_contributors,game_boss_reward_items FROM PUBLIC;
REVOKE ALL ON FUNCTION game_boss_reward_creation_guard(),game_boss_reward_death_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_boss_reward_deaths,game_boss_reward_contributors,game_boss_reward_items TO oteryn_game_runtime;
GRANT UPDATE(collected_transaction_id,expired) ON game_boss_reward_items TO oteryn_game_runtime;
