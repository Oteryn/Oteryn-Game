-- SRX-3 additive source content and temporal rules.
-- Main 1100/1101/1152 own the canonical tagged BYTEA death namespace,
-- exact Instance Ground admission and corpse MINT guard chain.
-- The canonical Instance corpse deadline and receipt-scope extension is owned
-- by M29 migration2200. Retain only independent source corpse registries here.
-- Keep immutable Channel audit bodies in their original shape when the later
-- canonical Instance extension adds instance_id. JSON subtraction is valid
-- before that column exists and idempotent for historical Channel receipts.
CREATE FUNCTION game_spell_field_temporal_receipt_json(game_spell_field_temporal_receipts)
RETURNS JSONB LANGUAGE sql STABLE STRICT AS $$
    SELECT CASE WHEN ($1).channel_id IS NOT NULL THEN to_jsonb($1)-'instance_id'
        ELSE to_jsonb($1) END
$$;
REVOKE ALL ON FUNCTION game_spell_field_temporal_receipt_json(game_spell_field_temporal_receipts) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_spell_field_temporal_receipt_json(game_spell_field_temporal_receipts) TO oteryn_game_runtime,oteryn_game_control;
DO $$ DECLARE body TEXT; amended TEXT; BEGIN
    EXECUTE format('ALTER FUNCTION game_spell_field_temporal_receipt_json(game_spell_field_temporal_receipts) SET search_path = pg_catalog, %I, pg_temp',current_schema());
    body:=pg_get_functiondef('game_spell_field_temporal_receipt_proven()'::regprocedure);
    IF position('to_jsonb(NEW)' IN body)=0 THEN RAISE EXCEPTION 'SRX-3 field chain audit proof changed'; END IF;
    amended:=replace(body,'to_jsonb(NEW)','game_spell_field_temporal_receipt_json(NEW)');
    EXECUTE amended;
END $$;

-- Crystal 201189f data/items/items.xml:12116-12126,16272,16589,63392-63509,76817-76890:
-- original source corpse edges and durations, including the terminal body.
DO $$ DECLARE body TEXT; old TEXT; extra TEXT; BEGIN
 body:=pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
 old:='WHEN ''oteryn:item.tibia.i54678'' THEN';
 extra:=$cases$
 WHEN 'oteryn:item.tibia.i111' THEN ARRAY[111,4240,4241,4242]
 WHEN 'oteryn:item.tibia.i5993' THEN ARRAY[5993,4094,4158,4159]
 WHEN 'oteryn:item.tibia.i5995' THEN ARRAY[5995,4097,4098,4099]
 WHEN 'oteryn:item.tibia.i6081' THEN ARRAY[6081,4247,4248,4246]
 WHEN 'oteryn:item.tibia.i23478' THEN ARRAY[23478,23479,23480,23481]
 -- Pinned Crystal items.xml:46793-46805: normal Outburst's actual three bodies.
 WHEN 'oteryn:item.tibia.i23564' THEN ARRAY[23564,23565,23566]
 WHEN 'oteryn:item.tibia.i32610' THEN ARRAY[32610,32609,32608,32607]
 WHEN 'oteryn:item.tibia.i33792' THEN ARRAY[33792]
 WHEN 'oteryn:item.tibia.i33793' THEN ARRAY[33793]
 WHEN 'oteryn:item.tibia.i33797' THEN ARRAY[33797,33796,33795,33794]
 WHEN 'oteryn:item.tibia.i33801' THEN ARRAY[33801,33800,33799,33798]
 WHEN 'oteryn:item.tibia.i33805' THEN ARRAY[33805,33804,33803,33802]
 WHEN 'oteryn:item.tibia.i33809' THEN ARRAY[33809,33808,33807,33806]
 WHEN 'oteryn:item.tibia.i33813' THEN ARRAY[33813,33812,33811,33810]
 WHEN 'oteryn:item.tibia.i33817' THEN ARRAY[33817,33816,33815,33814]
 WHEN 'oteryn:item.tibia.i33876' THEN ARRAY[33876]
 WHEN 'oteryn:item.tibia.i33891' THEN ARRAY[33891]
 WHEN 'oteryn:item.tibia.i33897' THEN ARRAY[33897,33896,33895,33894]
 WHEN 'oteryn:item.tibia.i33901' THEN ARRAY[33901,33900,33899,33898]
 WHEN 'oteryn:item.tibia.i33905' THEN ARRAY[33905,33904,33903,33902]
 WHEN 'oteryn:item.tibia.i33909' THEN ARRAY[33909,33908,33907,33906]
 WHEN 'oteryn:item.tibia.i33949' THEN ARRAY[33949]
 WHEN 'oteryn:item.tibia.i33984' THEN ARRAY[33984]
 WHEN 'oteryn:item.tibia.i34121' THEN ARRAY[34121,34120,34119,34118]
 WHEN 'oteryn:item.tibia.i34125' THEN ARRAY[34125,34124,34123,34122]
 WHEN 'oteryn:item.tibia.i34133' THEN ARRAY[34133,34132,34131,34130]
 WHEN 'oteryn:item.tibia.i39344' THEN ARRAY[39344,39343,39342,39341]
 WHEN 'oteryn:item.tibia.i5973' THEN ARRAY[5973,4025,4026,4027]
 WHEN 'oteryn:item.tibia.i6028' THEN ARRAY[6028,4206,4207,4208]
 WHEN 'oteryn:item.tibia.i33859' THEN ARRAY[33859,33858,33857,33856]
 WHEN 'oteryn:item.tibia.i33863' THEN ARRAY[33863,33862,33861,33860]
 WHEN 'oteryn:item.tibia.i33867' THEN ARRAY[33867,33866,33865,33864]
 WHEN 'oteryn:item.tibia.i33871' THEN ARRAY[33871,33870,33869,33868]
 WHEN 'oteryn:item.tibia.i33875' THEN ARRAY[33875,33874,33873,33872]
 WHEN 'oteryn:item.tibia.i33889' THEN ARRAY[33889,33888,33887,33886]
 WHEN 'oteryn:item.tibia.i44012' THEN ARRAY[44012,44013,44014]
 WHEN 'oteryn:item.tibia.i44015' THEN ARRAY[44015,44016,44017]
 WHEN 'oteryn:item.tibia.i44018' THEN ARRAY[44018,44019,44020]
 WHEN 'oteryn:item.tibia.i44021' THEN ARRAY[44021,44022,44023]
 WHEN 'oteryn:item.tibia.i44024' THEN ARRAY[44024,44025,44026]
 WHEN 'oteryn:item.tibia.i43669' THEN ARRAY[43669,43668,43667,43666]
 WHEN 'oteryn:item.tibia.i43555' THEN ARRAY[43555,43554,43553,43552]
 WHEN 'oteryn:item.tibia.i43559' THEN ARRAY[43559,43558,43557,43556]
 WHEN 'oteryn:item.tibia.i43567' THEN ARRAY[43567,43566,43565,43564]
 WHEN 'oteryn:item.tibia.i43571' THEN ARRAY[43571,43570,43569,43568]
 WHEN 'oteryn:item.tibia.i43579' THEN ARRAY[43579,43578,43577,43576]
 WHEN 'oteryn:item.tibia.i43583' THEN ARRAY[43583,43582,43581,43580]
 WHEN 'oteryn:item.tibia.i43816' THEN ARRAY[43816,43815,43814,43813]
 WHEN 'oteryn:item.tibia.i43820' THEN ARRAY[43820,43819,43818,43817]
 WHEN 'oteryn:item.tibia.i43824' THEN ARRAY[43824,43823,43822,43821]
 WHEN 'oteryn:item.tibia.i43828' THEN ARRAY[43828,43827,43826,43825]
 WHEN 'oteryn:item.tibia.i43832' THEN ARRAY[43832,43831,43830,43829]
 WHEN 'oteryn:item.tibia.i43836' THEN ARRAY[43836,43835,43834,43833]
 WHEN 'oteryn:item.tibia.i43840' THEN ARRAY[43840,43839,43838,43837]
 WHEN 'oteryn:item.tibia.i43844' THEN ARRAY[43844,43843,43842,43841]
 $cases$;
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
  RAISE EXCEPTION 'SRX-3 source corpse registry layout changed'; END IF;
 EXECUTE replace(body,old,extra||old);
END $$;
CREATE FUNCTION game_source_corpse_durations_v1(p_key TEXT) RETURNS BIGINT[]
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT CASE
 WHEN p_key='oteryn:item.tibia.i111' THEN ARRAY[10000,1200000,1200000,1200000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i5993' THEN ARRAY[10000,300000,300000,60000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i5995' THEN ARRAY[10000,300000,300000,60000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i6081' THEN ARRAY[10000,1200000,1200000,1200000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i23478' THEN ARRAY[10000,300000,300000,60000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i23564' THEN ARRAY[300000,300000,60000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i32610' THEN ARRAY[10000,300000,100000,50000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33792' THEN ARRAY[0]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33793' THEN ARRAY[0]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33797' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33801' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33805' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33809' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33813' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33817' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33876' THEN ARRAY[300000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33891' THEN ARRAY[0]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33897' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33901' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33905' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33909' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33949' THEN ARRAY[300000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33984' THEN ARRAY[300000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i34121' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i34125' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i34133' THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN p_key IN ('oteryn:item.tibia.i43832','oteryn:item.tibia.i43836') THEN ARRAY[5000,5000,5000,5000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i43669' THEN ARRAY[10000,300000,300000,0]::BIGINT[]
 WHEN p_key IN ('oteryn:item.tibia.i44012','oteryn:item.tibia.i44015','oteryn:item.tibia.i44018','oteryn:item.tibia.i44021','oteryn:item.tibia.i44024') THEN ARRAY[5000,5000,5000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i33859' THEN ARRAY[10000,900000,600000,600000]::BIGINT[]
 WHEN p_key IN ('oteryn:item.tibia.i33863','oteryn:item.tibia.i33867','oteryn:item.tibia.i33871','oteryn:item.tibia.i33875','oteryn:item.tibia.i33889') THEN ARRAY[10000,600000,600000,600000]::BIGINT[]
 WHEN game_summer_corpse_chain(p_key) IS NOT NULL THEN ARRAY[10000,300000,300000,60000]::BIGINT[] END
$$;
DO $$ DECLARE body TEXT; old TEXT; replacement TEXT; BEGIN
 body:=pg_get_functiondef('game_summer_corpse_schedule()'::regprocedure);
 old:='DECLARE chain INTEGER[]; stage INTEGER; item game_item_instances%ROWTYPE; content TEXT;';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN RAISE EXCEPTION 'SRX-3 corpse schedule declaration changed'; END IF;
 body:=replace(body,old,'DECLARE chain INTEGER[]; durations BIGINT[]; elapsed BIGINT:=0; stage INTEGER; item game_item_instances%ROWTYPE; content TEXT;');
 old:='FOR stage IN 1..3 LOOP';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN RAISE EXCEPTION 'SRX-3 corpse schedule loop changed'; END IF;
 body:=replace(body,old,'durations:=game_source_corpse_durations_v1(item.definition_production_key); IF array_length(durations,1) IS DISTINCT FROM array_length(chain,1) THEN RAISE EXCEPTION ''source corpse durations differ from complete chain'' USING ERRCODE=''23514''; END IF; FOR stage IN 1..array_length(chain,1)-1 LOOP elapsed:=elapsed+durations[stage];');
 old:='CASE WHEN stage=1 THEN 10000 ELSE 300000 END';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN RAISE EXCEPTION 'SRX-3 corpse stage duration changed'; END IF;
 body:=replace(body,old,'durations[stage]');
 old:='NEW.materialized_at+CASE stage WHEN 1 THEN 10000 WHEN 2 THEN 310000 ELSE 610000 END';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN RAISE EXCEPTION 'SRX-3 corpse schedule deadline changed'; END IF;
 EXECUTE replace(body,old,'NEW.materialized_at+elapsed');
 body:=pg_get_functiondef('game_spell_field_temporal_schedule_proven()'::regprocedure);
 old:='corpse RECORD; chain INTEGER[];';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN RAISE EXCEPTION 'SRX-3 corpse policy declaration changed'; END IF;
 body:=replace(body,old,'corpse RECORD; chain INTEGER[]; durations BIGINT[];');
 old:='chain:=game_summer_corpse_chain(corpse.definition_production_key);';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN RAISE EXCEPTION 'SRX-3 corpse policy source changed'; END IF;
 body:=replace(body,old,old||' durations:=game_source_corpse_durations_v1(corpse.definition_production_key);');
 old:='NEW.source_stage NOT BETWEEN 1 AND 3';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN RAISE EXCEPTION 'SRX-3 corpse policy stage bound changed'; END IF;
 body:=replace(body,old,'NEW.source_stage NOT BETWEEN 1 AND array_length(chain,1)-1 OR array_length(durations,1) IS DISTINCT FROM array_length(chain,1)');
 old:='(CASE WHEN NEW.source_stage=1 THEN 10000 ELSE 300000 END)';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN RAISE EXCEPTION 'SRX-3 corpse policy duration changed'; END IF;
 body:=replace(body,old,'durations[NEW.source_stage]');
 old:='(CASE NEW.source_stage WHEN 1 THEN 10000 WHEN 2 THEN 310000 ELSE 610000 END)';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN RAISE EXCEPTION 'SRX-3 corpse policy deadline changed'; END IF;
 EXECUTE replace(body,old,'(SELECT sum(v) FROM unnest(durations) WITH ORDINALITY d(v,n) WHERE n<=NEW.source_stage)');
END $$;
-- Deadline is derived from the immutable original MINT identity, never the
-- current transformed body or presence of transform schedules. Single-stage
-- source bodies have no transform schedule. Unknown legacy bodies retain 60 s.
CREATE OR REPLACE FUNCTION game_corpse_retirement_deadline(p_item UUID) RETURNS BIGINT
LANGUAGE sql STABLE STRICT AS $$
 -- A zero terminal lifetime means source Item::canDecay=false. It is not
 -- an immediate timeout; only explicit InstanceRetire can remove that body.
 SELECT CASE WHEN original.d[array_length(original.d,1)]=0 THEN NULL
 ELSE m.materialized_at+COALESCE((
   SELECT sum(duration) FROM pg_catalog.unnest(original.d) AS source(duration)),60000) END
 FROM game_item_mint_receipts m CROSS JOIN LATERAL (
  SELECT game_source_corpse_durations_v1(m.loot_table_production_key) d
 ) original
 WHERE m.item_instance_id=p_item AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
$$;
DO $$ DECLARE body TEXT; f TEXT; old TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_corpse_terminal_stage_ready()','game_item_decay_retire_consistency_guard()'] LOOP
  -- The former takes its original Item identity; no overload is admitted.
  IF f='game_corpse_terminal_stage_ready()' THEN
   body:=pg_get_functiondef('game_corpse_terminal_stage_ready(uuid)'::regprocedure);
  ELSE body:=pg_get_functiondef(f::regprocedure); END IF;
  old:='r.source_stage=3';
  IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
   RAISE EXCEPTION 'SRX-3 terminal corpse stage layout changed: %',f; END IF;
  EXECUTE replace(body,old,'r.source_stage=(SELECT max(final.source_stage) FROM game_spell_field_temporal_schedules final WHERE final.item_instance_id=r.item_instance_id AND final.source_ordinal=0)');
 END LOOP;
END $$;
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_source_corpse_durations_v1(text)','game_corpse_retirement_deadline(uuid)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_source_corpse_durations_v1(text),game_corpse_retirement_deadline(uuid) TO oteryn_game_runtime,oteryn_game_control;

-- The physical native death namespace is Channel52 or Instance tag2+52.
-- Keep every existing kind, party, damage and terminal-source requirement.
DO $$ DECLARE c RECORD; body TEXT; old TEXT; seen INTEGER:=0; BEGIN
 FOR c IN SELECT conname,pg_get_constraintdef(oid) definition FROM pg_constraint
   WHERE conrelid='game_character_rotten_blood_receipts'::regclass AND contype='c'
     AND position('octet_length(source_death_key)' IN pg_get_constraintdef(oid))>0 LOOP
  seen:=seen+1;body:=c.definition;old:='octet_length(source_death_key) = 52';
  IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
   RAISE EXCEPTION 'SRX-3 Rotten Blood source-death constraint layout changed'; END IF;
  body:=replace(body,old,'(octet_length(source_death_key) = 52 OR (octet_length(source_death_key) = 53 AND substring(source_death_key from 1 for 1) = decode(''02'',''hex'')))');
  EXECUTE format('ALTER TABLE game_character_rotten_blood_receipts DROP CONSTRAINT %I',c.conname);
  EXECUTE format('ALTER TABLE game_character_rotten_blood_receipts ADD CONSTRAINT %I %s',c.conname,body);
 END LOOP;
 IF seen<>1 THEN RAISE EXCEPTION 'SRX-3 Rotten Blood source-death namespace constraint missing or ambiguous'; END IF;
END $$;

-- Crystal rottenEssences.lua:1-28. One real unit and one Quest flag share a
-- physical Type2 transaction. Source USE is valid in Channel, Activity or House.
DO $$ DECLARE body TEXT; actual INTEGER[]; constraint_name TEXT; BEGIN
 FOREACH constraint_name IN ARRAY ARRAY['game_item_use_records_cause_kind_check','game_item_use_cause_shape'] LOOP
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_item_use_records'::regclass AND conname=constraint_name;
 -- Read only positive cause alternatives, not the independent shape3/4 bounds.
 SELECT array_agg(n ORDER BY n) INTO actual FROM (
  SELECT DISTINCT n FROM (
   SELECT trim(part)::INTEGER n FROM regexp_matches(body,
    'cause_kind[[:space:]]*=[[:space:]]*ANY[[:space:]]*\([[:space:]]*ARRAY\[([^]]+)\]','g') AS found(value)
    CROSS JOIN LATERAL regexp_split_to_table(value[1],',') part
   UNION ALL SELECT (value[1])::INTEGER FROM regexp_matches(body,
    'cause_kind[[:space:]]*=[[:space:]]*([0-9]+)','g') AS found(value)
  ) values_of_cause
 ) distinct_causes;
 IF actual IS DISTINCT FROM ARRAY[1,2,3,328,470,471,494,495,527,578] OR left(body,6)<>'CHECK ' THEN
  RAISE EXCEPTION 'SRX-3 ItemUse cause registry differs: %',actual; END IF;
 EXECUTE format('ALTER TABLE game_item_use_records DROP CONSTRAINT %I',constraint_name);
 -- Preserve every old cause/shape conjunction verbatim. In particular the
 -- Cube remains limited to3/4 and Hireling uses remain burn-only.
 EXECUTE format('ALTER TABLE game_item_use_records ADD CONSTRAINT %I CHECK (%s OR (cause_kind=4 AND shape=1))',constraint_name,substr(body,7));
 END LOOP;
END $$;
ALTER TABLE game_item_use_records ALTER COLUMN channel_id DROP NOT NULL;
ALTER TABLE game_item_use_records ADD COLUMN activity_instance_id UUID
 CHECK(activity_instance_id IS NULL OR game_character_is_uuid_v7(activity_instance_id));
ALTER TABLE game_item_use_records ADD CONSTRAINT game_rotten_essence_use_scope CHECK(
 (activity_instance_id IS NULL AND channel_id IS NOT NULL)
 OR (activity_instance_id IS NOT NULL AND channel_id IS NULL AND source_house_key IS NULL AND cause_kind=4));
ALTER TABLE game_item_use_records DROP CONSTRAINT game_item_use_house_source;
ALTER TABLE game_item_use_records ADD CONSTRAINT game_item_use_house_source CHECK(
 (cause_kind=470 AND source_house_key IS NOT NULL AND game_house_scope_key_valid(source_house_key))
 OR (cause_kind IN(4,471) AND (source_house_key IS NULL OR game_house_scope_key_valid(source_house_key)))
 OR (cause_kind NOT IN(4,470,471) AND source_house_key IS NULL));
ALTER TABLE game_item_use_records ADD CONSTRAINT game_rotten_essence_exact_source CHECK(cause_kind<>4 OR (
 source_definition_family='Item' AND source_definition_revision_ref='definition-r1'
 AND source_definition_production_key IN('oteryn:item.tibia.i43501','oteryn:item.tibia.i43502','oteryn:item.tibia.i43503','oteryn:item.tibia.i43504')
 AND shape=1 AND source_kind=1 AND source_parent_item_instance_id IS NOT NULL
 AND flask_item_instance_id IS NULL));
ALTER TABLE game_item_use_records ADD COLUMN rotten_essence_transition_key TEXT GENERATED ALWAYS AS(
 CASE WHEN cause_kind=4 THEN CASE source_definition_production_key
  WHEN 'oteryn:item.tibia.i43501' THEN 'oteryn:quest-transition/rotten-blood/essence-murcion'
  WHEN 'oteryn:item.tibia.i43502' THEN 'oteryn:quest-transition/rotten-blood/essence-chagorz'
  WHEN 'oteryn:item.tibia.i43503' THEN 'oteryn:quest-transition/rotten-blood/essence-ichgahal'
  WHEN 'oteryn:item.tibia.i43504' THEN 'oteryn:quest-transition/rotten-blood/essence-vemiath' END END) STORED;
ALTER TABLE game_item_use_records ADD CONSTRAINT game_rotten_essence_quest_fk
 FOREIGN KEY(character_id,game_session_id,command_id,rotten_essence_transition_key)
 REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
 DEFERRABLE INITIALLY DEFERRED;

CREATE FUNCTION game_rotten_essence_item_pair_guard() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE track TEXT;
BEGIN
 IF NEW.cause_kind<>4 THEN RETURN NULL; END IF;
 track:=CASE NEW.source_definition_production_key
  WHEN 'oteryn:item.tibia.i43501' THEN 'oteryn:quest-progress/storage/52280'
  WHEN 'oteryn:item.tibia.i43502' THEN 'oteryn:quest-progress/storage/52282'
  WHEN 'oteryn:item.tibia.i43503' THEN 'oteryn:quest-progress/storage/52281'
  WHEN 'oteryn:item.tibia.i43504' THEN 'oteryn:quest-progress/storage/52283' END;
 IF track IS NULL OR NEW.created_xact_id<>pg_current_xact_id()
  OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts q
   WHERE q.character_id=NEW.character_id AND q.cause_id=NEW.game_session_id AND q.cause_ordinal=NEW.command_id
    AND q.transition_key=NEW.rotten_essence_transition_key AND q.cause_kind='use'
    AND q.quest_key='oteryn:quest/rotten-blood' AND NOT q.completes
    AND q.track_keys=ARRAY[track] AND q.values_before=ARRAY[0]::BIGINT[] AND q.values_after=ARRAY[1]::BIGINT[]
    AND q.created_xact_id=pg_current_xact_id())
  OR EXISTS(SELECT 1 FROM game_character_quest_xp_obligations x WHERE x.character_id=NEW.character_id
   AND x.cause_id=NEW.game_session_id AND x.cause_ordinal=NEW.command_id AND x.transition_key=NEW.rotten_essence_transition_key)
 THEN RAISE EXCEPTION 'Rotten essence USE requires its exact same-xact Quest flag' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
  JOIN game_runtime_scope_assignments a ON a.world_id=NEW.world_id AND a.state=1
   AND a.ownership_generation=NEW.runtime_scope_ownership_generation
  WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id AND s.world_id=NEW.world_id
   AND s.runtime_scope_world_id=NEW.world_id AND s.current_generation=NEW.expected_session_generation
   AND s.scope_ownership_generation=NEW.runtime_scope_ownership_generation AND s.session_state IN(1,2)
   AND (
    (NEW.activity_instance_id IS NULL AND NEW.source_house_key IS NULL AND s.runtime_scope_kind=1
     AND s.runtime_scope_channel_id=NEW.channel_id AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL
     AND a.scope_kind=1 AND a.channel_id=NEW.channel_id)
    OR (NEW.source_house_key IS NOT NULL AND NEW.activity_instance_id IS NULL AND s.runtime_scope_kind=2
     AND s.runtime_scope_channel_id IS NULL AND s.runtime_scope_house_key=NEW.source_house_key
     AND s.runtime_scope_instance_id=game_house_scope_instance_id(NEW.world_id,NEW.source_house_key)
     AND s.origin_channel_id=NEW.channel_id AND a.scope_kind=2 AND a.house_key=NEW.source_house_key)
    OR (NEW.activity_instance_id IS NOT NULL AND NEW.source_house_key IS NULL AND s.runtime_scope_kind=2
     AND s.runtime_scope_channel_id IS NULL AND s.runtime_scope_house_key IS NULL AND s.runtime_scope_instance_id=NEW.activity_instance_id
     AND a.scope_kind=3 AND a.activity_instance_id=NEW.activity_instance_id
     AND EXISTS(SELECT 1 FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id)
      JOIN game_activity_handoff_participants p USING(handoff_id)
      WHERE g.world_id=NEW.world_id AND g.instance_id=NEW.activity_instance_id AND g.ownership_generation=a.ownership_generation AND g.state=2
       AND e.holder_node_id=a.holder_node_id AND e.holder_registration_revision=a.holder_registration_revision
       AND p.character_id=NEW.character_id AND p.destination_game_session_id=NEW.game_session_id
       AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE r.handoff_id=p.handoff_id AND r.character_id=p.character_id)))
   )) THEN RAISE EXCEPTION 'Rotten essence USE requires its exact live source scope' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_rotten_essence_item_pair AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_essence_item_pair_guard();
CREATE FUNCTION game_rotten_essence_quest_pair_guard() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.transition_key NOT IN('oteryn:quest-transition/rotten-blood/essence-murcion','oteryn:quest-transition/rotten-blood/essence-chagorz',
  'oteryn:quest-transition/rotten-blood/essence-ichgahal','oteryn:quest-transition/rotten-blood/essence-vemiath') THEN RETURN NULL; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_item_use_records u WHERE u.cause_kind=4 AND u.character_id=NEW.character_id
  AND u.game_session_id=NEW.cause_id AND u.command_id=NEW.cause_ordinal AND u.rotten_essence_transition_key=NEW.transition_key
  AND u.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Rotten essence Quest flag lacks its exact one-unit USE' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_rotten_essence_quest_pair AFTER INSERT ON game_character_quest_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_essence_quest_pair_guard();
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_rotten_essence_item_pair_guard()','game_rotten_essence_quest_pair_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_control',f);
 END LOOP;
END $$;

-- Elder's final source factory body43666 is Generic (no Container), and has
-- no decay timer. Unexpected contents must stay in custody, never be orphaned.
-- This predicate grants no transform: the existing receipt guard still proves
-- every source stage, deadline, current scoped owner, audit and custody tuple.
CREATE FUNCTION game_source_corpse_terminal_contents_v1(p_item UUID,p_target TEXT)
RETURNS BOOLEAN LANGUAGE sql STABLE STRICT SECURITY DEFINER AS $$
 SELECT p_target<>'oteryn:item.tibia.i43666' OR (
  NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=p_item)
  AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=p_item))
$$;
DO $$ DECLARE body TEXT; old TEXT; BEGIN
 EXECUTE format('ALTER FUNCTION game_source_corpse_terminal_contents_v1(uuid,text) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_source_corpse_terminal_contents_v1(uuid,text) FROM PUBLIC;
 body:=pg_get_functiondef('game_spell_field_temporal_receipt_proven()'::regprocedure);
 old:='IF p.target_family IS NOT NULL AND NOT (';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
  RAISE EXCEPTION 'SRX-3 corpse terminal custody guard layout changed'; END IF;
 EXECUTE replace(body,old,$guard$
 IF p.target_family IS NOT NULL AND game_source_corpse_terminal_contents_v1(NEW.item_instance_id,p.target_production_key) IS NOT TRUE THEN
  RAISE EXCEPTION 'Generic source corpse terminal requires empty direct and nested custody' USING ERRCODE='23514'; END IF;
 IF p.target_family IS NOT NULL AND NOT (
 $guard$);
END $$;
GRANT EXECUTE ON FUNCTION game_source_corpse_terminal_contents_v1(uuid,text) TO oteryn_game_runtime,oteryn_game_control;

-- Exact Crystal201189f five Rotten Blood lever recipes; source entry prerequisite level250.
CREATE FUNCTION game_rotten_blood_boss_catalogue_activate(
    p_boss TEXT,p_world UUID,p_channel UUID,p_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA,
    p_activation NUMERIC,p_server BYTEA,p_client BYTEA,p_content_revision TEXT
) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE current_sequence NUMERIC; current_server BYTEA; current_client BYTEA; scope BYTEA; recipe RECORD;
BEGIN
    IF NOT game_runtime_scope_assignment_history_valid()
        OR NOT game_node_prove_current_incarnation(p_node,p_registration,p_secret)
        OR octet_length(p_server)<>32 OR octet_length(p_client)<>32
        OR octet_length(p_content_revision) NOT BETWEEN 1 AND 128 THEN RETURN false; END IF;
    IF p_boss NOT IN ('oteryn:boss.ichgahal','oteryn:boss.murcion','oteryn:boss.vemiath','oteryn:boss.chagorz','oteryn:boss.bakragore') THEN RETURN false; END IF;
    scope:=decode('01','hex')||uuid_send(p_world)||uuid_send(p_channel);
    PERFORM 1 FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g ON g.scope_key=scope
        WHERE a.scope_key=scope AND a.scope_kind=1 AND a.world_id=p_world AND a.channel_id=p_channel
            AND a.activity_instance_id IS NULL AND a.house_key IS NULL AND a.state=1
            AND a.ownership_generation=p_generation AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration
            AND g.ready AND g.ownership_generation=p_generation FOR SHARE OF a,g;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT activation_sequence,server_artifact_digest,client_artifact_digest INTO current_sequence,current_server,current_client
        FROM game_content_activations WHERE world_id=p_world AND channel_id=p_channel
        ORDER BY activation_sequence DESC LIMIT 1 FOR SHARE;
    IF NOT FOUND OR current_sequence<>p_activation OR current_server<>p_server OR current_client<>p_client THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_boss,985));
    IF EXISTS(SELECT 1 FROM game_boss_room_catalogue WHERE boss_key=p_boss
        AND content_revision<>p_content_revision)
        AND EXISTS(SELECT 1 FROM game_activity_handoff_groups WHERE activity_key=p_boss AND state=2)
    THEN RETURN false; END IF;
    SELECT * INTO STRICT recipe FROM (VALUES
      ('oteryn:boss.ichgahal','oteryn:creature.ichgahal',72000,'[{"x":32978,"y":32333,"floor":15},{"x":32977,"y":32333,"floor":15},{"x":32976,"y":32333,"floor":15},{"x":32975,"y":32333,"floor":15},{"x":32974,"y":32333,"floor":15}]'::jsonb,'[{"x":33004,"y":32341,"floor":15},{"x":33004,"y":32341,"floor":15},{"x":33004,"y":32341,"floor":15},{"x":33004,"y":32341,"floor":15},{"x":33004,"y":32341,"floor":15}]'::jsonb,'{"x":33004,"y":32343,"floor":15}'::jsonb,'{"x":33008,"y":32333,"floor":15}'::jsonb,'oteryn:room.rotten-blood-ichgahal-source'),
      ('oteryn:boss.murcion','oteryn:creature.murcion',72000,'[{"x":32974,"y":32365,"floor":15},{"x":32975,"y":32365,"floor":15},{"x":32976,"y":32365,"floor":15},{"x":32977,"y":32365,"floor":15},{"x":32978,"y":32365,"floor":15}]'::jsonb,'[{"x":33009,"y":32372,"floor":15},{"x":33009,"y":32372,"floor":15},{"x":33009,"y":32372,"floor":15},{"x":33009,"y":32372,"floor":15},{"x":33009,"y":32372,"floor":15}]'::jsonb,'{"x":33904,"y":31693,"floor":15}'::jsonb,'{"x":33008,"y":32365,"floor":15}'::jsonb,'oteryn:room.rotten-blood-murcion-source'),
      ('oteryn:boss.vemiath','oteryn:creature.vemiath',72000,'[{"x":33078,"y":32333,"floor":15},{"x":33077,"y":32333,"floor":15},{"x":33076,"y":32333,"floor":15},{"x":33075,"y":32333,"floor":15},{"x":33074,"y":32333,"floor":15}]'::jsonb,'[{"x":33043,"y":32341,"floor":15},{"x":33043,"y":32341,"floor":15},{"x":33043,"y":32341,"floor":15},{"x":33043,"y":32341,"floor":15},{"x":33043,"y":32341,"floor":15}]'::jsonb,'{"x":34001,"y":31820,"floor":15}'::jsonb,'{"x":33044,"y":32332,"floor":15}'::jsonb,'oteryn:room.rotten-blood-vemiath-source'),
      ('oteryn:boss.chagorz','oteryn:creature.chagorz',72000,'[{"x":33074,"y":32367,"floor":15},{"x":33075,"y":32367,"floor":15},{"x":33076,"y":32367,"floor":15},{"x":33077,"y":32367,"floor":15},{"x":33078,"y":32367,"floor":15}]'::jsonb,'[{"x":33044,"y":32373,"floor":15},{"x":33044,"y":32373,"floor":15},{"x":33044,"y":32373,"floor":15},{"x":33044,"y":32373,"floor":15},{"x":33044,"y":32373,"floor":15}]'::jsonb,'{"x":33904,"y":31693,"floor":15}'::jsonb,'{"x":33044,"y":32362,"floor":15}'::jsonb,'oteryn:room.rotten-blood-chagorz-source'),
      ('oteryn:boss.bakragore','oteryn:creature.bakragore',244800,'[{"x":33010,"y":32392,"floor":15},{"x":33011,"y":32392,"floor":15},{"x":33012,"y":32392,"floor":15},{"x":33013,"y":32392,"floor":15},{"x":33014,"y":32392,"floor":15},{"x":33078,"y":32398,"floor":15},{"x":33077,"y":32398,"floor":15},{"x":33076,"y":32398,"floor":15},{"x":33075,"y":32398,"floor":15},{"x":33074,"y":32398,"floor":15}]'::jsonb,'[{"x":33044,"y":32407,"floor":15},{"x":33044,"y":32407,"floor":15},{"x":33044,"y":32407,"floor":15},{"x":33044,"y":32407,"floor":15},{"x":33044,"y":32407,"floor":15},{"x":33044,"y":32407,"floor":15},{"x":33044,"y":32407,"floor":15},{"x":33044,"y":32407,"floor":15},{"x":33044,"y":32407,"floor":15},{"x":33044,"y":32407,"floor":15}]'::jsonb,'{"x":34106,"y":32052,"floor":13}'::jsonb,'{"x":33044,"y":32394,"floor":15}'::jsonb,'oteryn:room.rotten-blood-bakragore-source')
    ) AS recipes(boss,creature,cooldown,levers,entries,exit,boss_position,room) WHERE boss=p_boss;
    INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,
        cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,
        exit_position,boss_position,after_kill_seconds)
    VALUES(p_boss,p_content_revision,recipe.creature,1,5,250,recipe.cooldown,1200,
        recipe.levers,recipe.room,p_boss,true,'crystal-201189f',recipe.entries,recipe.exit,recipe.boss_position,60)
    ON CONFLICT(boss_key) DO UPDATE SET content_revision=excluded.content_revision,boss_creature_key=excluded.boss_creature_key,
        minimum_players=excluded.minimum_players,maximum_players=excluded.maximum_players,minimum_level=excluded.minimum_level,
        cooldown_seconds=excluded.cooldown_seconds,time_limit_seconds=excluded.time_limit_seconds,lever_positions=excluded.lever_positions,
        room_template_key=excluded.room_template_key,reward_key=excluded.reward_key,enabled=excluded.enabled,
        template_revision=excluded.template_revision,entry_positions=excluded.entry_positions,exit_position=excluded.exit_position,
        boss_position=excluded.boss_position,after_kill_seconds=excluded.after_kill_seconds;
    RETURN true;
END $$;
REVOKE ALL ON FUNCTION game_rotten_blood_boss_catalogue_activate(TEXT,UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_rotten_blood_boss_catalogue_activate(TEXT,UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT) TO oteryn_game_runtime;


DO $$ BEGIN EXECUTE format('ALTER FUNCTION %I.game_rotten_blood_boss_catalogue_activate(TEXT,UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema(),current_schema()); END $$;

-- Source plain corpse Items remain ordinary Ground custody, not containers.
-- Only an original, still-Ground lethal MINT is eligible; a later loop drop
-- keeps its existing loop provenance and native coordinate representation.
CREATE FUNCTION game_item_plain_corpse_source(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE STRICT AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_mint_receipts m
 JOIN game_item_instances i ON i.item_instance_id=m.item_instance_id
 JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
 WHERE m.item_instance_id=p_item AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
 AND m.draw_ordinal=0 AND m.materialized_at IS NOT NULL
 AND m.transaction_id=i.minted_transaction_id AND i.lifecycle=1 AND i.quantity=1
 AND m.loot_table_family='Item' AND i.definition_family='Item'
 AND m.loot_table_production_key=i.definition_production_key
 AND m.loot_table_revision_ref=i.definition_revision_ref
 AND i.definition_production_key IN ('oteryn:item.tibia.i33792','oteryn:item.tibia.i33793',
 'oteryn:item.tibia.i33876','oteryn:item.tibia.i33891','oteryn:item.tibia.i33949','oteryn:item.tibia.i33984')
 AND g.world_id=m.death_world_id AND g.scope_kind=m.death_scope_kind
 AND g.channel_id IS NOT DISTINCT FROM m.death_channel_id
 AND g.instance_id IS NOT DISTINCT FROM m.death_instance_id
 AND g.runtime_scope_ownership_generation=m.death_scope_ownership_generation
 AND g.corpse_ref=(CASE WHEN m.death_scope_kind=2 THEN decode('02','hex') ELSE decode('','hex') END
   ||uuid_send(m.death_world_id)||uuid_send(coalesce(m.death_channel_id,m.death_instance_id))
   ||decode(lpad(to_hex(trunc(m.death_scope_ownership_generation/4294967296)::bigint),8,'0')
      ||lpad(to_hex(mod(m.death_scope_ownership_generation,4294967296)::bigint),8,'0'),'hex')
   ||substring(int8send(m.death_actor_local_id) from 5 for 4)
   ||decode(lpad(to_hex(trunc(m.death_actor_local_generation/4294967296)::bigint),8,'0')
      ||lpad(to_hex(mod(m.death_actor_local_generation,4294967296)::bigint),8,'0'),'hex'))
 AND NOT game_item_loop_drop_ref(g.corpse_ref))
$$;
DO $$ DECLARE body TEXT; old TEXT; BEGIN
 EXECUTE format('ALTER FUNCTION game_item_plain_corpse_source(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_item_plain_corpse_source(uuid) FROM PUBLIC;
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 old:='game_item_map_harvest_source(NEW.item_instance_id)';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
  RAISE EXCEPTION 'SRX-3 plain corpse Ground admission layout changed'; END IF;
 EXECUTE replace(body,old,old||' OR game_item_plain_corpse_source(NEW.item_instance_id)');
END $$;
GRANT EXECUTE ON FUNCTION game_item_plain_corpse_source(uuid) TO oteryn_game_runtime;

-- SRX-3 literal public Source-Channel flame phase, shared across Players.
-- Immutable provenance of a real source USE; not a replacement GameStorage map.
CREATE TABLE game_rotten_bak_preparations (
 character_id UUID NOT NULL,
 cause_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL,
 transition_key TEXT NOT NULL,
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=33),
 world_id UUID NOT NULL,
 source_channel_id UUID NOT NULL,
 source_generation NUMERIC(20,0) NOT NULL CHECK(source_generation BETWEEN 1 AND 18446744073709551615),
 reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch BETWEEN 0 AND 18446744073709551615),
 activation_sequence NUMERIC(20,0) NOT NULL CHECK(activation_sequence BETWEEN 1 AND 18446744073709551615),
 server_digest BYTEA NOT NULL CHECK(octet_length(server_digest)=32),
 client_digest BYTEA NOT NULL CHECK(octet_length(client_digest)=32),
 frame_digest BYTEA NOT NULL CHECK(octet_length(frame_digest)=32),
 map_digest BYTEA NOT NULL CHECK(octet_length(map_digest)=32),
 flavor SMALLINT NOT NULL CHECK(flavor BETWEEN 0 AND 3),
 source_phase_ordinal NUMERIC(20,0) NOT NULL CHECK(source_phase_ordinal BETWEEN 0 AND 18446744073709551614),
 committed_revision NUMERIC(20,0) NOT NULL CHECK(committed_revision BETWEEN 2 AND 18446744073709551615),
 flame_key NUMERIC(20,0), flame_revision NUMERIC(20,0),
 pillar_key NUMERIC(20,0), pillar_revision NUMERIC(20,0),
 CHECK((flame_key IS NULL)=(flame_revision IS NULL)),
 CHECK((pillar_key IS NULL)=(pillar_revision IS NULL)),
 CHECK(flame_key IS NULL OR flame_key BETWEEN 0 AND 18446744073709551615),
 CHECK(pillar_key IS NULL OR pillar_key BETWEEN 0 AND 18446744073709551615),
 CHECK(flame_revision IS NULL OR flame_revision BETWEEN 1 AND 18446744073709551615),
 CHECK(pillar_revision IS NULL OR pillar_revision BETWEEN 1 AND 18446744073709551615),
 UNIQUE(world_id,source_channel_id,source_generation,reset_epoch,activation_sequence,source_phase_ordinal,flavor),
 PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
 FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
 REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
);
CREATE FUNCTION game_rotten_bak_preparation_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE suffix TEXT; track TEXT; BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'source preparation receipts are immutable' USING ERRCODE='23514'; END IF;
 suffix := (ARRAY['murcion','ichgahal','chagorz','vemiath'])[NEW.flavor+1];
 track := (ARRAY['52280','52281','52282','52283'])[NEW.flavor+1];
 IF NEW.transition_key<>'oteryn:quest-callback/rotten-blood/prepare-'||suffix
 OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r
 WHERE (r.character_id,r.cause_id,r.cause_ordinal,r.transition_key)=
       (NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
 AND r.cause_kind='use' AND r.request_binding=NEW.request_binding
 AND r.committed_character_revision=NEW.committed_revision
 AND r.track_keys=ARRAY['oteryn:quest-progress/storage/'||track]
 AND r.values_before=ARRAY[1]::bigint[] AND r.values_after=ARRAY[0]::bigint[]
 AND r.xmin::text::bigint=txid_current())
 OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a
 WHERE a.world_id=NEW.world_id AND a.channel_id=NEW.source_channel_id
 AND a.state=1 AND a.ownership_generation=NEW.source_generation)
 OR NOT EXISTS(SELECT 1 FROM game_content_activations a
 WHERE a.world_id=NEW.world_id AND a.channel_id=NEW.source_channel_id
 AND a.activation_sequence=NEW.activation_sequence
 AND a.server_artifact_digest=NEW.server_digest AND a.client_artifact_digest=NEW.client_digest
 AND a.frame_binding_digest=NEW.frame_digest
 AND NOT EXISTS(SELECT 1 FROM game_content_activations later WHERE later.world_id=a.world_id
 AND later.channel_id=a.channel_id AND later.activation_sequence>a.activation_sequence))
 OR NEW.source_phase_ordinal<>game_rotten_bak_global_phase_ordinal(NEW.world_id,NEW.source_channel_id,NEW.source_generation,NEW.reset_epoch,NEW.activation_sequence)
 OR NEW.reset_epoch<>coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=NEW.world_id),0)
 THEN RAISE EXCEPTION 'source flame requires its exact same-transaction burn/current source fence' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER game_rotten_bak_preparation_guard AFTER INSERT ON game_rotten_bak_preparations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_bak_preparation_guard();
CREATE TRIGGER game_rotten_bak_preparation_immutable BEFORE UPDATE OR DELETE ON game_rotten_bak_preparations
 FOR EACH ROW EXECUTE FUNCTION game_rotten_bak_preparation_guard();
GRANT SELECT,INSERT ON game_rotten_bak_preparations TO oteryn_game_runtime;

-- Admission records provenance ONLY. Source first-think, not entry, consumes flags.
CREATE TABLE game_rotten_bak_phase_origins (
 handoff_id UUID PRIMARY KEY REFERENCES game_activity_handoff_groups(handoff_id),
 world_id UUID NOT NULL, source_channel_id UUID NOT NULL,
 source_generation NUMERIC(20,0) NOT NULL, reset_epoch NUMERIC(20,0) NOT NULL,
 activation_sequence NUMERIC(20,0) NOT NULL,
 server_digest BYTEA NOT NULL CHECK(octet_length(server_digest)=32),
 client_digest BYTEA NOT NULL CHECK(octet_length(client_digest)=32),
 frame_digest BYTEA NOT NULL CHECK(octet_length(frame_digest)=32),
 map_digest BYTEA NOT NULL CHECK(octet_length(map_digest)=32)
);
CREATE FUNCTION game_rotten_bak_phase_origin_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'source phase origins are immutable' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_activity_handoff_groups g
 JOIN game_activity_runtime_entries e USING(handoff_id)
 JOIN game_activity_handoff_participants m ON m.handoff_id=g.handoff_id AND m.character_id=e.initiating_character_id
 JOIN game_content_activations a ON a.world_id=m.world_id AND a.channel_id=m.origin_channel_id
 AND a.activation_sequence=NEW.activation_sequence
 WHERE g.handoff_id=NEW.handoff_id AND g.activity_key='oteryn:boss.bakragore'
 AND g.state=2 AND g.xmin::text::bigint=txid_current()
 AND m.world_id=NEW.world_id AND m.origin_channel_id=NEW.source_channel_id
 AND m.source_ownership_generation=NEW.source_generation
 AND m.coordinate_frame_digest=NEW.frame_digest AND m.map_revision_digest=NEW.map_digest
 AND m.server_artifact_digest=NEW.server_digest
 AND a.server_artifact_digest=NEW.server_digest AND a.client_artifact_digest=NEW.client_digest
 AND a.frame_binding_digest=NEW.frame_digest
 AND NEW.reset_epoch=coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=NEW.world_id),0)
 AND NOT EXISTS(SELECT 1 FROM game_content_activations later WHERE later.world_id=a.world_id
 AND later.channel_id=a.channel_id AND later.activation_sequence>a.activation_sequence))
 THEN RAISE EXCEPTION 'phase origin requires exact current admission provenance' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER game_rotten_bak_phase_origin_guard AFTER INSERT ON game_rotten_bak_phase_origins
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_bak_phase_origin_guard();
CREATE TRIGGER game_rotten_bak_phase_origin_immutable BEFORE UPDATE OR DELETE ON game_rotten_bak_phase_origins
 FOR EACH ROW EXECUTE FUNCTION game_rotten_bak_phase_origin_guard();
CREATE TABLE game_rotten_bak_phase_preparations (
 handoff_id UUID PRIMARY KEY REFERENCES game_rotten_bak_phase_origins(handoff_id),
 essence_mask SMALLINT NOT NULL CHECK(essence_mask BETWEEN 0 AND 15),
 source_phase_ordinal NUMERIC(20,0) NOT NULL CHECK(source_phase_ordinal BETWEEN 1 AND 18446744073709551615),
 world_id UUID NOT NULL, source_channel_id UUID NOT NULL,
 source_generation NUMERIC(20,0) NOT NULL, reset_epoch NUMERIC(20,0) NOT NULL,
 activation_sequence NUMERIC(20,0) NOT NULL,
 think_actor_local_id BIGINT NOT NULL CHECK(think_actor_local_id BETWEEN 1 AND 4294967295),
 think_actor_local_generation NUMERIC(20,0) NOT NULL CHECK(think_actor_local_generation BETWEEN 1 AND 18446744073709551615),
 think_sequence NUMERIC(20,0) NOT NULL CHECK(think_sequence BETWEEN 0 AND 18446744073709551615),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 UNIQUE(world_id,source_channel_id,source_generation,reset_epoch,activation_sequence,source_phase_ordinal)
);
CREATE FUNCTION game_rotten_bak_global_phase_ordinal(w UUID,c UUID,g NUMERIC,e NUMERIC,a NUMERIC)
 RETURNS NUMERIC LANGUAGE sql STABLE STRICT AS $$
 SELECT coalesce((SELECT source_phase_ordinal FROM game_rotten_bak_phase_preparations
 WHERE world_id=w AND source_channel_id=c AND source_generation=g AND reset_epoch=e
 AND activation_sequence=a ORDER BY source_phase_ordinal DESC LIMIT 1),0)
$$;
CREATE INDEX game_rotten_bak_phase_current ON game_rotten_bak_preparations(
 world_id,source_channel_id,source_generation,reset_epoch,activation_sequence,source_phase_ordinal,flavor);
CREATE TABLE game_rotten_bak_preparation_claims (
 handoff_id UUID NOT NULL REFERENCES game_rotten_bak_phase_preparations(handoff_id),
 flavor SMALLINT NOT NULL CHECK(flavor BETWEEN 0 AND 3),
 character_id UUID NOT NULL, cause_id UUID NOT NULL, cause_ordinal NUMERIC(20,0) NOT NULL,
 transition_key TEXT NOT NULL,
 PRIMARY KEY(handoff_id,flavor),
 UNIQUE(character_id,cause_id,cause_ordinal,transition_key),
 FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
 REFERENCES game_rotten_bak_preparations(character_id,cause_id,cause_ordinal,transition_key)
);
-- No Character-in-roster condition: who lit the public flame is audit DATA only.
CREATE FUNCTION game_rotten_bak_phase_claim_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE mask SMALLINT; BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'source phase claims are immutable' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_rotten_bak_phase_origins o JOIN game_activity_handoff_groups g USING(handoff_id)
 WHERE o.handoff_id=NEW.handoff_id AND g.activity_key='oteryn:boss.bakragore' AND g.state=2
 AND (o.world_id,o.source_channel_id,o.source_generation,o.reset_epoch,o.activation_sequence)=
 (NEW.world_id,NEW.source_channel_id,NEW.source_generation,NEW.reset_epoch,NEW.activation_sequence))
 THEN RAISE EXCEPTION 'first-think claim requires its exact source origin' USING ERRCODE='23514'; END IF;
 SELECT coalesce(bit_or(1::integer<<c.flavor),0)::smallint INTO mask FROM game_rotten_bak_preparation_claims c WHERE c.handoff_id=NEW.handoff_id;
 IF mask<>NEW.essence_mask THEN RAISE EXCEPTION 'phase mask requires its exact global flavor claims' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER game_rotten_bak_phase_claim_guard AFTER INSERT ON game_rotten_bak_phase_preparations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_bak_phase_claim_guard();
CREATE TRIGGER game_rotten_bak_phase_claim_immutable BEFORE UPDATE OR DELETE ON game_rotten_bak_phase_preparations
 FOR EACH ROW EXECUTE FUNCTION game_rotten_bak_phase_claim_guard();
CREATE FUNCTION game_rotten_bak_flavor_claim_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'source flavor claims are immutable' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_rotten_bak_phase_preparations h JOIN game_rotten_bak_preparations p
 ON (p.character_id,p.cause_id,p.cause_ordinal,p.transition_key)=
 (NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
 WHERE h.handoff_id=NEW.handoff_id AND p.flavor=NEW.flavor
 AND (p.world_id,p.source_channel_id,p.source_generation,p.reset_epoch,p.activation_sequence,p.source_phase_ordinal)=
 (h.world_id,h.source_channel_id,h.source_generation,h.reset_epoch,h.activation_sequence,h.source_phase_ordinal-1)
 AND h.xmin::text::bigint=txid_current())
 THEN RAISE EXCEPTION 'flavor claim requires the actual global phase source receipt' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER game_rotten_bak_flavor_claim_guard AFTER INSERT ON game_rotten_bak_preparation_claims
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_bak_flavor_claim_guard();
CREATE TRIGGER game_rotten_bak_flavor_claim_immutable BEFORE UPDATE OR DELETE ON game_rotten_bak_preparation_claims
 FOR EACH ROW EXECUTE FUNCTION game_rotten_bak_flavor_claim_guard();
REVOKE ALL ON FUNCTION game_rotten_bak_global_phase_ordinal(UUID,UUID,NUMERIC,NUMERIC,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_rotten_bak_global_phase_ordinal(UUID,UUID,NUMERIC,NUMERIC,NUMERIC) TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_rotten_bak_phase_origins,game_rotten_bak_phase_preparations,game_rotten_bak_preparation_claims TO oteryn_game_runtime;

-- Technical completion of a committed FIRST-think phase's external fixed-map
-- reset. Missing publication blocks the next phase's public activations; it
-- never means an unprepared mask. Bytes bind eight source-ordered reset results.
CREATE TABLE game_rotten_bak_phase_publications (
 handoff_id UUID PRIMARY KEY REFERENCES game_rotten_bak_phase_preparations(handoff_id),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 reset_results BYTEA NOT NULL CHECK(octet_length(reset_results)=168),
 node_id UUID NOT NULL, node_registration_revision NUMERIC(20,0) NOT NULL
 CHECK(node_registration_revision BETWEEN 1 AND 18446744073709551615)
);
CREATE FUNCTION game_rotten_bak_phase_publication_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE i INTEGER; active INTEGER; inactive INTEGER; BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'phase publication receipts are immutable' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_rotten_bak_phase_preparations p
 JOIN game_rotten_bak_phase_origins o USING(handoff_id)
 JOIN game_runtime_scope_assignments a ON a.world_id=o.world_id AND a.channel_id=o.source_channel_id
 WHERE p.handoff_id=NEW.handoff_id AND p.request_binding=NEW.request_binding
 AND a.state=1 AND a.ownership_generation=o.source_generation
 AND a.holder_node_id=NEW.node_id AND a.holder_registration_revision=NEW.node_registration_revision
 AND EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=o.world_id AND c.channel_id=o.source_channel_id
 AND c.activation_sequence=o.activation_sequence AND c.server_artifact_digest=o.server_digest
 AND c.client_artifact_digest=o.client_digest AND c.frame_binding_digest=o.frame_digest
 AND NOT EXISTS(SELECT 1 FROM game_content_activations later WHERE later.world_id=c.world_id AND later.channel_id=c.channel_id AND later.activation_sequence>c.activation_sequence))
 AND o.reset_epoch=coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=o.world_id),0))
 THEN RAISE EXCEPTION 'phase publication requires its exact current origin owner/header' USING ERRCODE='23514'; END IF;
 FOR i IN 0..7 LOOP
  active := CASE WHEN i%2=1 THEN 43508 ELSE (ARRAY[43984,43498,43988,43986])[i/2+1] END;
  inactive := CASE WHEN i%2=1 THEN 43507 ELSE (ARRAY[43983,43497,43987,43985])[i/2+1] END;
  IF get_byte(NEW.reset_results,i*21) NOT IN(0,1)
   OR (get_byte(NEW.reset_results,i*21)=0 AND substring(NEW.reset_results FROM i*21+2 FOR 16)<>decode(repeat('00',16),'hex'))
   OR (get_byte(NEW.reset_results,i*21)=1 AND substring(NEW.reset_results FROM i*21+10 FOR 8)=decode(repeat('00',8),'hex'))
   OR get_byte(NEW.reset_results,i*21+17)*256+get_byte(NEW.reset_results,i*21+18)<>active
   OR get_byte(NEW.reset_results,i*21+19)*256+get_byte(NEW.reset_results,i*21+20)<>inactive
  THEN RAISE EXCEPTION 'phase publication requires eight literal source reset results' USING ERRCODE='23514'; END IF;
 END LOOP;
 RETURN NEW;
END $$;
CREATE TRIGGER game_rotten_bak_phase_publication_insert BEFORE INSERT ON game_rotten_bak_phase_publications
 FOR EACH ROW EXECUTE FUNCTION game_rotten_bak_phase_publication_guard();
CREATE TRIGGER game_rotten_bak_phase_publication_immutable BEFORE UPDATE OR DELETE ON game_rotten_bak_phase_publications
 FOR EACH ROW EXECUTE FUNCTION game_rotten_bak_phase_publication_guard();
GRANT SELECT,INSERT ON game_rotten_bak_phase_publications TO oteryn_game_runtime;

-- Literal Outburst source recipe; level1 represents donor no-level-gate on canonical level>=1.
CREATE FUNCTION game_outburst_boss_catalogue_activate(
    p_boss TEXT,p_world UUID,p_channel UUID,p_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA,
    p_activation NUMERIC,p_server BYTEA,p_client BYTEA,p_content_revision TEXT
) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE current_sequence NUMERIC; current_server BYTEA; current_client BYTEA; scope BYTEA; recipe RECORD;
BEGIN
    IF NOT game_runtime_scope_assignment_history_valid()
        OR NOT game_node_prove_current_incarnation(p_node,p_registration,p_secret)
        OR octet_length(p_server)<>32 OR octet_length(p_client)<>32
        OR octet_length(p_content_revision) NOT BETWEEN 1 AND 128 THEN RETURN false; END IF;
    IF p_boss<>'oteryn:boss.outburst' THEN RETURN false; END IF;
    scope:=decode('01','hex')||uuid_send(p_world)||uuid_send(p_channel);
    PERFORM 1 FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g ON g.scope_key=scope
        WHERE a.scope_key=scope AND a.scope_kind=1 AND a.world_id=p_world AND a.channel_id=p_channel
            AND a.activity_instance_id IS NULL AND a.house_key IS NULL AND a.state=1
            AND a.ownership_generation=p_generation AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration
            AND g.ready AND g.ownership_generation=p_generation FOR SHARE OF a,g;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT activation_sequence,server_artifact_digest,client_artifact_digest INTO current_sequence,current_server,current_client
        FROM game_content_activations WHERE world_id=p_world AND channel_id=p_channel
        ORDER BY activation_sequence DESC LIMIT 1 FOR SHARE;
    IF NOT FOUND OR current_sequence<>p_activation OR current_server<>p_server OR current_client<>p_client THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_boss,985));
    IF EXISTS(SELECT 1 FROM game_boss_room_catalogue WHERE boss_key=p_boss
        AND content_revision<>p_content_revision)
        AND EXISTS(SELECT 1 FROM game_activity_handoff_groups WHERE activity_key=p_boss AND state=2)
    THEN RETURN false; END IF;
    SELECT 'oteryn:creature.outburst'::text AS creature,72000 AS cooldown,
 '[{"x":32207,"y":31284,"floor":14},{"x":32207,"y":31285,"floor":14},{"x":32207,"y":31286,"floor":14},{"x":32207,"y":31287,"floor":14},{"x":32207,"y":31288,"floor":14}]'::jsonb AS levers,
 '[{"x":32234,"y":31292,"floor":14},{"x":32234,"y":31292,"floor":14},{"x":32234,"y":31292,"floor":14},{"x":32234,"y":31292,"floor":14},{"x":32234,"y":31292,"floor":14}]'::jsonb AS entries,
 '{"x":32218,"y":31375,"floor":14}'::jsonb AS exit,'{"x":32234,"y":31284,"floor":14}'::jsonb AS boss_position,'oteryn:room.callback-outburst-source'::text AS room INTO recipe;
    INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,
        cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,
        exit_position,boss_position,after_kill_seconds)
    VALUES(p_boss,p_content_revision,recipe.creature,1,5,1,recipe.cooldown,1200,
        recipe.levers,recipe.room,p_boss,true,'crystal-201189f',recipe.entries,recipe.exit,recipe.boss_position,60)
    ON CONFLICT(boss_key) DO UPDATE SET content_revision=excluded.content_revision,boss_creature_key=excluded.boss_creature_key,
        minimum_players=excluded.minimum_players,maximum_players=excluded.maximum_players,minimum_level=excluded.minimum_level,
        cooldown_seconds=excluded.cooldown_seconds,time_limit_seconds=excluded.time_limit_seconds,lever_positions=excluded.lever_positions,
        room_template_key=excluded.room_template_key,reward_key=excluded.reward_key,enabled=excluded.enabled,
        template_revision=excluded.template_revision,entry_positions=excluded.entry_positions,exit_position=excluded.exit_position,
        boss_position=excluded.boss_position,after_kill_seconds=excluded.after_kill_seconds;
    RETURN true;
END $$;
REVOKE ALL ON FUNCTION game_outburst_boss_catalogue_activate(TEXT,UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_outburst_boss_catalogue_activate(TEXT,UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT) TO oteryn_game_runtime;


DO $$ BEGIN EXECUTE format('ALTER FUNCTION %I.game_outburst_boss_catalogue_activate(TEXT,UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema(),current_schema()); END $$;

-- Crystal Item::transformItem starts the target ItemType lifetime. Preserve
-- Game.createItem's unstarted decay; only this sealed footprint transform starts
-- the native 43297 (248 seconds, remove) policy on the existing scoped timer.
DO $migration$
DECLARE body TEXT; begin_at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_spell_field_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'source transform temporal proof layout changed'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF NEW.source_transaction_id IS NOT NULL AND EXISTS (
   SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.source_transaction_id
    AND l.ordinal=NEW.source_ordinal AND l.item_instance_id=NEW.item_instance_id AND l.operation_kind=7
 ) THEN
  DECLARE r game_spell_item_receipts%ROWTYPE; l game_spell_item_lines%ROWTYPE; op JSONB; clock JSONB;
  BEGIN
   SELECT * INTO r FROM game_spell_item_receipts WHERE transaction_id=NEW.source_transaction_id;
   SELECT * INTO l FROM game_spell_item_lines WHERE transaction_id=NEW.source_transaction_id AND ordinal=NEW.source_ordinal;
   op:=convert_from(r.intent,'UTF8')::jsonb->'operations'->(NEW.source_ordinal-1);
   clock:=op->'intrinsic_decay';
   IF r.cause_kind IS DISTINCT FROM 3 OR r.runtime_scope_kind IS DISTINCT FROM 2 OR r.channel_id IS NOT NULL
    OR r.creature_definition_key NOT IN ('oteryn:creature.vemiath','oteryn:creature.bakragore')
    OR r.spell_family IS DISTINCT FROM 'Creature' OR r.spell_production_key IS DISTINCT FROM r.creature_definition_key
    OR r.spell_revision IS DISTINCT FROM 'definition-r1' OR r.creature_definition_revision IS DISTINCT FROM 'definition-r1'
    OR r.binding IS DISTINCT FROM sha256(r.intent) OR r.catalog_digest IS DISTINCT FROM NEW.content_digest
    OR r.created_xact_id IS DISTINCT FROM pg_current_xact_id() OR l.created_xact_id IS DISTINCT FROM r.created_xact_id
    OR NEW.created_xact_id IS DISTINCT FROM r.created_xact_id
    OR (l.world_id,l.runtime_scope_kind,l.channel_id,l.instance_id) IS DISTINCT FROM (r.world_id,r.runtime_scope_kind,r.channel_id,r.instance_id)
    OR l.operation_kind IS DISTINCT FROM 7 OR l.quantity_before IS DISTINCT FROM 1 OR l.quantity_after IS DISTINCT FROM 1
    OR l.item_instance_id IS DISTINCT FROM NEW.item_instance_id OR l.content_generation_digest IS DISTINCT FROM NEW.content_digest
    OR l.definition_family IS DISTINCT FROM 'Item' OR l.definition_production_key IS DISTINCT FROM 'oteryn:item.tibia.i43297'
    OR l.definition_revision IS DISTINCT FROM 'definition-r1'
    OR convert_from(r.intent,'UTF8')::jsonb#>>'{source_event,rule}' IS DISTINCT FROM 'rotten-blood.footprint.transform'
    OR op->>'kind' IS DISTINCT FROM 'transform_ground'
    OR op->'before' IS DISTINCT FROM jsonb_build_object('family','Item','key','oteryn:item.tibia.i43590','revision','definition-r1')
    OR op->'definition' IS DISTINCT FROM jsonb_build_object('family','Item','key','oteryn:item.tibia.i43297','revision','definition-r1')
    OR jsonb_typeof(clock) IS DISTINCT FROM 'object' OR (SELECT count(*) FROM jsonb_object_keys(clock))<>3
    OR jsonb_typeof(clock->'original_execution_us') IS DISTINCT FROM 'string'
    OR (clock->>'original_execution_us') !~ '^[0-9]{1,20}$'
    OR (clock->>'original_execution_us')::numeric IS NULL OR (clock->>'original_execution_us')::numeric NOT BETWEEN 0 AND 18446744073709551615
    OR jsonb_typeof(clock->'execution_unix_ms') IS DISTINCT FROM 'number'
    OR jsonb_typeof(clock->'duration_millis') IS DISTINCT FROM 'number'
    OR (clock->>'execution_unix_ms')::bigint IS NULL OR (clock->>'execution_unix_ms')::bigint<=0
    OR (clock->>'execution_unix_ms')::bigint>r.occurred_at_unix_ms
    OR (clock->>'duration_millis')::bigint IS DISTINCT FROM 248000
    OR NEW.source_stage IS DISTINCT FROM 1 OR NEW.duration_millis IS DISTINCT FROM 248000
    OR NEW.expires_at_unix_ms IS DISTINCT FROM (clock->>'execution_unix_ms')::bigint+248000
    OR NEW.definition_family IS DISTINCT FROM 'Item' OR NEW.definition_production_key IS DISTINCT FROM 'oteryn:item.tibia.i43297'
    OR NEW.definition_revision IS DISTINCT FROM 'definition-r1'
    OR NEW.target_family IS NOT NULL OR NEW.target_production_key IS NOT NULL OR NEW.target_revision IS NOT NULL
    OR NEW.blocks_movement OR NEW.blocks_projectile OR NEW.immovable_block_solid
    OR NOT EXISTS(SELECT 1 FROM game_spell_item_audit_outbox a WHERE a.transaction_id=r.transaction_id
      AND a.event_id=r.event_id AND a.envelope=r.intent AND a.envelope_sha256=r.binding AND a.created_xact_id=r.created_xact_id)
   THEN RAISE EXCEPTION 'source transform timer requires exact same-XID footprint and native ItemType policy' USING ERRCODE='23514'; END IF;
   RETURN NULL;
  END;
 END IF;
$arm$ from begin_at for 5);
END $migration$;
