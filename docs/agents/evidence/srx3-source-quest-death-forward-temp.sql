\connect crs_native_activity_wire_1793956128963709318
\timing on
\set ON_ERROR_STOP on
\set VERBOSITY verbose
BEGIN;
CREATE TEMP TABLE srx3_source_event_gate (LIKE game_character_soul_war_receipts INCLUDING DEFAULTS INCLUDING CONSTRAINTS) ON COMMIT DROP;
CREATE TRIGGER srx3_legacy_source BEFORE INSERT ON srx3_source_event_gate FOR EACH ROW EXECUTE FUNCTION game_soul_war_death_source_guard();
-- Copied native onDeath Quest cause; legacy XP-parent death cause is unchanged.
-- The closed Rust writer consumes an opaque canonical BEGIN/event/cohort proof,
-- then holds the existing current NodeScope/session/Character locks. This new
-- receipt arm never inserts an XP receipt/member for a non-XP participant.
ALTER TABLE srx3_source_event_gate
 ADD COLUMN source_event_death_key BYTEA;
ALTER TABLE srx3_source_event_gate
 DROP CONSTRAINT game_character_soul_war_receipts_check4,
 ADD CONSTRAINT game_soul_war_boss_source_present CHECK (
  ((source_death_key IS NOT NULL)::integer + (source_event_death_key IS NOT NULL)::integer)
   = CASE WHEN boss_key IS NULL THEN 0 ELSE 1 END
 ),
 ADD CONSTRAINT game_soul_war_source_event_namespace CHECK (
  source_event_death_key IS NULL OR (
   source_attempt IS NULL AND source_death_key IS NULL AND
   (octet_length(source_event_death_key)=52 OR
    (octet_length(source_event_death_key)=53 AND get_byte(source_event_death_key,0)=2))
  )
 );
CREATE UNIQUE INDEX game_soul_war_source_event_member
 ON srx3_source_event_gate(source_event_death_key,character_id)
 WHERE source_event_death_key IS NOT NULL;
CREATE FUNCTION pg_temp.game_soul_war_copied_death_event_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE b JSONB; expected_creatures TEXT[]; expected_event TEXT; expected_index INTEGER;
BEGIN
 IF NEW.source_event_death_key IS NULL THEN RETURN NEW; END IF;
 b:=convert_from(NEW.request_binding,'UTF8')::jsonb;
 CASE NEW.boss_key
 WHEN 'megalomania' THEN
  expected_creatures:=ARRAY['oteryn:creature.goshnar_s_megalomania_green','oteryn:creature.goshnar_s_megalomania_blue'];
  expected_event:='MegalomaniaDeath'; expected_index:=1;
 WHEN 'hatred' THEN
  expected_creatures:=ARRAY['oteryn:creature.goshnar_s_hatred'];
  expected_event:='SoulWarBossesDeath'; expected_index:=1;
 WHEN 'malice','spite','cruelty','greed' THEN
  expected_creatures:=ARRAY['oteryn:creature.goshnar_s_'||NEW.boss_key];
  expected_event:='SoulWarBossesDeath'; expected_index:=0;
 ELSE RAISE EXCEPTION 'Unqualified copied Soul War death event' USING ERRCODE='23514';
 END CASE;
 IF b->>'kind' IS DISTINCT FROM 'copied-native-death-v1'
  OR b->>'death' IS DISTINCT FROM encode(NEW.source_event_death_key,'hex')
  OR b->>'character' IS DISTINCT FROM replace(NEW.character_id::text,'-','')
  OR NOT coalesce(b->>'creature'=ANY(expected_creatures),false)
  OR b->>'revision' IS DISTINCT FROM 'definition-r1'
  OR b->>'event' IS DISTINCT FROM expected_event
  OR b->>'registration_index' IS DISTINCT FROM expected_index::text
  OR b->>'source_sha256' IS DISTINCT FROM '9e1107dc41abc16b4f56e795180c1c17c917d8e73754547c443f070ed3167916'
  OR jsonb_typeof(b->'observed_at_seconds') IS DISTINCT FROM 'number'
  OR NOT coalesce((b->>'observed_at_seconds') ~ '^[0-9]{1,19}$',false)
 THEN RAISE EXCEPTION 'Unqualified copied Soul War death event' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_soul_war_copied_death_event_guard
 BEFORE INSERT ON srx3_source_event_gate
 FOR EACH ROW EXECUTE FUNCTION pg_temp.game_soul_war_copied_death_event_guard();

 INSERT INTO srx3_source_event_gate(character_id,occurrence,request_binding,result,source_event_death_key,boss_key,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed,original_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES('00000000-0000-7000-8000-000000000001',decode('0101010101010101010101010101010101010101010101010101010101010101','hex'),convert_to(jsonb_build_object('kind','copied-native-death-v1','death','0000000000007000800000000000002a0000000000007000800000000000002b0000000000000001000000010000000000000001','character','00000000000070008000000000000001','creature','oteryn:creature.goshnar_s_greed','revision','definition-r1','event','SoulWarBossesDeath','registration_index',0,'source_sha256','9e1107dc41abc16b4f56e795180c1c17c917d8e73754547c443f070ed3167916','observed_at_seconds',1794038400)::text,'UTF8'),decode('01','hex'),decode('0000000000007000800000000000002a0000000000007000800000000000002b0000000000000001000000010000000000000001','hex'),'greed',false,0,NULL,false,false,1,1,1,0,0,'p','r','c','s','e','d','q','v');
 INSERT INTO srx3_source_event_gate(character_id,occurrence,request_binding,result,source_death_key,boss_key,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed,original_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES('00000000-0000-7000-8000-000000000001',decode('0202020202020202020202020202020202020202020202020202020202020202','hex'),decode('01','hex'),decode('01','hex'),NULL,NULL,false,0,NULL,false,false,1,1,1,0,0,'p','r','c','s','e','d','q','v');
DO $probe$ BEGIN BEGIN
 INSERT INTO srx3_source_event_gate(character_id,occurrence,request_binding,result,source_death_key,boss_key,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed,original_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES('00000000-0000-7000-8000-000000000001',decode('0101010101010101010101010101010101010101010101010101010101010101','hex'),decode('01','hex'),decode('01','hex'),decode('0000000000007000800000000000002a0000000000007000800000000000002b0000000000000001000000010000000000000001','hex'),'greed',false,0,NULL,false,false,1,1,1,0,0,'p','r','c','s','e','d','q','v');
RAISE EXCEPTION 'Expected rejection missing'; EXCEPTION WHEN check_violation THEN RAISE NOTICE 'LEGACY_NON_XP_STILL_REJECTED SQLSTATE=%',SQLSTATE; END; END $probe$;
DO $probe$ BEGIN BEGIN
 INSERT INTO srx3_source_event_gate(character_id,occurrence,request_binding,result,source_event_death_key,boss_key,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed,original_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES('00000000-0000-7000-8000-000000000001',decode('0101010101010101010101010101010101010101010101010101010101010101','hex'),convert_to(jsonb_build_object('kind','copied-native-death-v1','death','0000000000007000800000000000002a0000000000007000800000000000002b0000000000000001000000010000000000000001','character','00000000000070008000000000000001','creature','oteryn:creature.goshnar_s_megalomania_purple','revision','definition-r1','event','SoulWarBossesDeath','registration_index',0,'source_sha256','9e1107dc41abc16b4f56e795180c1c17c917d8e73754547c443f070ed3167916','observed_at_seconds',1794038400)::text,'UTF8'),decode('01','hex'),decode('0000000000007000800000000000002a0000000000007000800000000000002b0000000000000001000000010000000000000001','hex'),'greed',false,0,NULL,false,false,1,1,1,0,0,'p','r','c','s','e','d','q','v');
RAISE EXCEPTION 'Expected rejection missing'; EXCEPTION WHEN check_violation THEN RAISE NOTICE 'PURPLE_FALSE_EVENT_REJECTED SQLSTATE=%',SQLSTATE; END; END $probe$;
SELECT count(*)=2 AS closed_source_and_ordinary_controls_accepted FROM srx3_source_event_gate;
ROLLBACK;
