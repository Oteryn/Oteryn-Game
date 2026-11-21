-- Elemental Spheres: scope donor eligibility to the four elemental overlords.
-- Preserve complete census, late-death and personal reward integrity checks.
-- Consistency guard for encounter-rules::boss::reward_eligibility_for_boss.
CREATE OR REPLACE FUNCTION game_boss_reward_death_complete() RETURNS trigger LANGUAGE plpgsql AS $$
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
        IF NEW.boss_key IN ('oteryn:creature.earth_overlord', 'oteryn:creature.energy_overlord',
                            'oteryn:creature.fire_overlord', 'oteryn:creature.ice_overlord') THEN
            -- Crystal 201189f reward_chest.lua:54-69; this exception is local to Elemental Spheres.
            earned:=c.damage_dealt>0 OR c.damage_received>0 OR c.healing_others>0;
        ELSE
            -- Preserve 0629 and boss-rewards-v1's accepted anti-leech policy verbatim.
            score_sum:=coalesce(trunc(c.damage_dealt*1000000000/nullif(total_dealt,0)),0)
                +coalesce(trunc(c.damage_received*1000000000/nullif(total_received,0)),0)
                +coalesce(trunc(c.healing_others*1000000000/nullif(total_healed,0)),0);
            earned:=c.damage_dealt>0 AND score_sum*100*damage_dealers>=10*3*1000000000::numeric;
        END IF;
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

-- CREATE OR REPLACE must retain the 0629 fixed schema search path.
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_boss_reward_death_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

-- The canonical receipt is shared by chest, XP and quest settlement.
CREATE TABLE game_boss_death_settlements (
    world_id UUID NOT NULL,
    death_id UUID NOT NULL,
    boss_key TEXT NOT NULL,
    minted_at BIGINT NOT NULL CHECK(minted_at>0),
    scope_kind SMALLINT NOT NULL CHECK(scope_kind IN (1,2)),
    scope_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation>0),
    personal_rewards BOOLEAN NOT NULL,
    intent BYTEA NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(world_id,death_id)
);
CREATE TRIGGER game_boss_death_settlements_stamp BEFORE INSERT ON game_boss_death_settlements
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_boss_death_settlements_immutable BEFORE UPDATE OR DELETE ON game_boss_death_settlements
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_boss_death_settlements_no_truncate BEFORE TRUNCATE ON game_boss_death_settlements
    EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_boss_death_settlements FROM PUBLIC;
GRANT SELECT,INSERT ON game_boss_death_settlements TO oteryn_game_runtime;

CREATE FUNCTION game_boss_death_quest_skip_create_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM game_boss_death_settlements d
        WHERE d.world_id=NEW.world_id AND d.death_id=NEW.death_id
          AND d.created_xact_id=pg_current_xact_id()
          AND d.boss_key IN ('oteryn:creature.earth_overlord','oteryn:creature.energy_overlord','oteryn:creature.fire_overlord','oteryn:creature.ice_overlord')) THEN
        RAISE EXCEPTION 'quest skip requires the same-transaction Overlord death' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_boss_death_quest_skip_create_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_boss_death_quest_skip_create_guard() FROM PUBLIC;

-- Only an incompatible definition records a terminal no-grant result.
CREATE TABLE game_boss_death_quest_skips (
    world_id UUID NOT NULL,
    death_id UUID NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    quest_key TEXT NOT NULL CHECK(quest_key='oteryn:quest.elemental_spheres_quest'),
    definition_hash BYTEA NOT NULL CHECK(octet_length(definition_hash)=32),
    request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=33),
    reason TEXT NOT NULL CHECK(reason='incompatible_definition'),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(world_id,death_id,character_id,quest_key),
    FOREIGN KEY(world_id,death_id) REFERENCES game_boss_death_settlements(world_id,death_id)
);
CREATE TRIGGER game_boss_death_quest_skips_stamp BEFORE INSERT ON game_boss_death_quest_skips
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_boss_death_quest_skips_create BEFORE INSERT ON game_boss_death_quest_skips
    FOR EACH ROW EXECUTE FUNCTION game_boss_death_quest_skip_create_guard();
CREATE TRIGGER game_boss_death_quest_skips_immutable BEFORE UPDATE OR DELETE ON game_boss_death_quest_skips
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_boss_death_quest_skips_no_truncate BEFORE TRUNCATE ON game_boss_death_quest_skips
    EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_boss_death_quest_skips FROM PUBLIC;
GRANT SELECT,INSERT ON game_boss_death_quest_skips TO oteryn_game_runtime;
