-- Free canonical main1315 names without a gap in the published bootstrap proof.
-- Recreate the same constraint triggers under private names in this transaction.
DO $migration$
DECLARE body TEXT; trigger_definition TEXT; site RECORD;
BEGIN
    IF to_regprocedure('game_father_values_proven()') IS NULL THEN RETURN; END IF;
    SELECT pg_get_functiondef('game_father_values_proven()'::regprocedure) INTO STRICT body;
    IF strpos(body,'required_xp')>0 THEN RETURN; END IF;
    IF strpos(body,'expected_amount BIGINT')=0 OR strpos(body,'x.amount=expected_amount')=0 THEN
        RAISE EXCEPTION 'Father bootstrap proof provenance changed';
    END IF;
    FOR site IN SELECT * FROM (VALUES
        ('game_father_npc_values','game_character_quest_receipts','game_questpc8_bootstrap_father_npc_values'),
        ('game_father_material_values','game_item_craft_records','game_questpc8_bootstrap_father_material_values'),
        ('game_father_cape_values','game_reward_claim_mint_receipts','game_questpc8_bootstrap_father_cape_values')
    ) AS sites(old_name,relation_name,new_name) LOOP
        SELECT pg_get_triggerdef(t.oid) INTO STRICT trigger_definition FROM pg_trigger t
        WHERE t.tgname=site.old_name AND t.tgrelid=site.relation_name::regclass
          AND t.tgfoid='game_father_values_proven()'::regprocedure AND NOT t.tgisinternal;
        IF strpos(trigger_definition,'CREATE CONSTRAINT TRIGGER '||site.old_name||' ')<>1 THEN
            RAISE EXCEPTION 'Father bootstrap constraint shape changed';
        END IF;
        EXECUTE format('DROP TRIGGER %I ON %s',site.old_name,site.relation_name::regclass);
        EXECUTE replace(trigger_definition,'CREATE CONSTRAINT TRIGGER '||site.old_name||' ',
            'CREATE CONSTRAINT TRIGGER '||site.new_name||' ');
    END LOOP;
    ALTER FUNCTION game_father_values_proven() RENAME TO game_questpc8_bootstrap_father_values_proven;
END $migration$;
