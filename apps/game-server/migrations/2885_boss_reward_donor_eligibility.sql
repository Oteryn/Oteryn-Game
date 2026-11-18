-- Elemental Spheres: scope donor eligibility to the four elemental overlords.
-- Preserve complete census, late-death and personal reward integrity checks.
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
