-- Crystal201189f8 Hairycles212–216/451–456: no new Ape City quest writer.
DO $migration$
DECLARE body TEXT; old TEXT; replacement TEXT;
BEGIN
 body:=pg_get_functiondef('game_npc_cosmetic_receipt_guard()'::regprocedure);
 old:='WHEN ''kesar'' THEN ''oteryn:npc.kesar_the_younger_day'' ELSE NULL END';
 replacement:='WHEN ''kesar'' THEN ''oteryn:npc.kesar_the_younger_day'' WHEN ''hairycles'' THEN ''oteryn:npc.hairycles'' ELSE NULL END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Hairycles NPC identity guard boundary changed';END IF;
 body:=replace(body,old,replacement);
 old:='WHEN ''oteryn:npc-cosmetic/amber/finish'' THEN ARRAY[''oteryn:appearance-grant/npc/amber/citizen/addon1'']';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Hairycles grant guard boundary changed';END IF;
 body:=replace(body,old,'WHEN ''oteryn:npc-cosmetic/hairycles/shaman'' THEN ARRAY[''oteryn:appearance-grant/npc/hairycles/shaman/base''] '||old);
 old:='(''oteryn:npc-cosmetic/amber/finish'',''oteryn:npc-cosmetic/kesar/council'')';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Hairycles equivalent grant boundary changed';END IF;
 body:=replace(body,old,'(''oteryn:npc-cosmetic/amber/finish'',''oteryn:npc-cosmetic/kesar/council'',''oteryn:npc-cosmetic/hairycles/shaman'')');
 old:='WHEN ''oteryn:npc-cosmetic/kesar/council'' THEN ''{"oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/kings_council":1}''::jsonb';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Hairycles exact track boundary changed';END IF;
 body:=replace(body,old,'WHEN ''oteryn:npc-cosmetic/hairycles/shaman'' THEN ''{"oteryn:quest-progress/quest/u7_6/the_ape_city/shaman_outfit":1}''::jsonb '||old);
 old:='WHEN ''oteryn:npc-cosmetic/kesar/council'' THEN ''[]''::jsonb';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Hairycles no-cost boundary changed';END IF;
 body:=replace(body,old,'WHEN ''oteryn:npc-cosmetic/hairycles/shaman'' THEN ''[]''::jsonb '||old);
 old:='    RETURN NULL;';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Hairycles deferred prerequisite boundary changed';END IF;
 replacement:=$proof$
    IF NEW.source_key='oteryn:npc-cosmetic/hairycles/shaman' THEN
        PERFORM 1 FROM game_character_quest_tracks hair_stage
        WHERE hair_stage.character_id=NEW.character_id
            AND hair_stage.track_key='oteryn:quest-progress/quest/u7_6/the_ape_city/questline'
            AND hair_stage.quest_key='oteryn:quest.the_ape_city_quest' AND hair_stage.value=18 FOR SHARE;
        IF NOT FOUND OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.track_changes) hair_change
            WHERE hair_change->>'key'='oteryn:quest-progress/quest/u7_6/the_ape_city/shaman_outfit'
                AND hair_change->>'quest'='oteryn:quest.the_ape_city_quest'
                AND (hair_change->>'before')::bigint IS NOT NULL AND (hair_change->>'before')::bigint<>1
                AND (hair_change->>'after')::bigint=1) THEN
            RAISE EXCEPTION 'Hairycles Shaman claim lacks completed quest and unclaimed source flag' USING ERRCODE='23514';
        END IF;
        IF NEW.grant_keys=ARRAY[]::TEXT[] AND NOT EXISTS(
            SELECT 1 FROM game_account_appearance_unlocks hair_unlock JOIN game_account_appearance_unlock_receipts hair_grant USING(grant_occurrence_id)
            WHERE hair_unlock.account_id=NEW.account_id AND hair_unlock.unlock_key='oteryn:appearance-grant/npc/hairycles/shaman/base'
                AND hair_grant.account_id=NEW.account_id AND hair_grant.content_revision='crystal-201189f-npc-cosmetics-v1') THEN
            RAISE EXCEPTION 'Hairycles cannot omit an unowned Shaman base grant' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NULL;
$proof$;
 body:=replace(body,old,replacement);
 EXECUTE body;
END $migration$;

-- Hairycles is the sixth source-qualified permanent cosmetic NPC. Keep the
-- original five identities and admit only the exact Shaman source owner.
ALTER TABLE game_npc_cosmetic_receipts
 DROP CONSTRAINT game_npc_cosmetic_receipts_npc_key_check,
 ADD CONSTRAINT game_npc_cosmetic_receipts_npc_key_check
 CHECK(npc_key IN('oteryn:npc.amber','oteryn:npc.norma','oteryn:npc.tom','oteryn:npc.cledwyn','oteryn:npc.kesar_the_younger_day','oteryn:npc.hairycles'));
