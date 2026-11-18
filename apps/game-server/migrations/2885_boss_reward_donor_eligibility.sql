-- Align the retained settlement validator with the production donor planner.
-- Preserve complete census, late-death and personal reward integrity checks.
CREATE OR REPLACE FUNCTION game_boss_reward_death_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE c game_boss_reward_contributors%ROWTYPE;
    earned BOOLEAN;
BEGIN
    IF (SELECT count(*) FROM game_boss_reward_contributors WHERE world_id=NEW.world_id AND death_id=NEW.death_id)<>NEW.participant_count THEN
        RAISE EXCEPTION 'incomplete boss reward participant census' USING ERRCODE='23514';
    END IF;
    IF (SELECT count(*) FROM game_boss_reward_contributors WHERE world_id=NEW.world_id AND death_id=NEW.death_id AND eligible)<>NEW.eligible_count THEN
        RAISE EXCEPTION 'incomplete eligible boss reward census' USING ERRCODE='23514';
    END IF;
    FOR c IN SELECT * FROM game_boss_reward_contributors WHERE world_id=NEW.world_id AND death_id=NEW.death_id LOOP
        -- Crystal 201189f data/scripts/systems/reward_chest.lua:54-69 admits
        -- every nonzero contribution, as does RewardParticipation::eligible.
        earned:=c.damage_dealt>0 OR c.damage_received>0 OR c.healing_others>0;
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
