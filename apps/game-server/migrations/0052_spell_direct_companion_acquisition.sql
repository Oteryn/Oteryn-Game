-- LOCAL CANDIDATE: immutable direct Summon/Convince source receipt.
-- Current physical master/actor authority remains in the real Channel owner.
CREATE TABLE game_spell_direct_companion_acquisition_receipts (
 transaction_id UUID PRIMARY KEY REFERENCES game_spell_item_receipts(transaction_id),
 world_id UUID NOT NULL, channel_id UUID NOT NULL,
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
 actor_local_id BIGINT NOT NULL CHECK(actor_local_id BETWEEN 1 AND 4294967295),
 actor_local_generation NUMERIC(20,0) NOT NULL CHECK(actor_local_generation BETWEEN 1 AND 18446744073709551615),
 actor_placement_identity UUID NOT NULL,
 master_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 master_game_session_id UUID NOT NULL,
 master_actor_local_id BIGINT NOT NULL CHECK(master_actor_local_id BETWEEN 1 AND 4294967295),
 master_actor_local_generation NUMERIC(20,0) NOT NULL CHECK(master_actor_local_generation BETWEEN 1 AND 18446744073709551615),
 master_placement_identity UUID NOT NULL,
 creature_key TEXT NOT NULL CHECK(octet_length(creature_key) BETWEEN 1 AND 512),
 creature_revision TEXT NOT NULL CHECK(octet_length(creature_revision) BETWEEN 1 AND 512),
 cell_x INTEGER NOT NULL,cell_y INTEGER NOT NULL,cell_z INTEGER NOT NULL CHECK(cell_z BETWEEN -32768 AND 32767),
 source_kind SMALLINT NOT NULL CHECK(source_kind IN (1,2)),
 source_json JSONB NOT NULL,
 binding_json JSONB NOT NULL CHECK(octet_length(binding_json::text)<=8192),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((actor_local_id,actor_local_generation)<>(master_actor_local_id,master_actor_local_generation))
);
CREATE INDEX game_spell_direct_companion_actor_history ON game_spell_direct_companion_acquisition_receipts
 (world_id,channel_id,ownership_generation,actor_local_id,actor_local_generation);

CREATE FUNCTION game_spell_direct_companion_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE receipt_intent JSONB; actor_binding JSONB; master_binding JSONB; expected JSONB;
BEGIN
 SELECT convert_from(r.intent,'UTF8')::jsonb INTO receipt_intent
 FROM game_spell_item_receipts r
 WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id()
 AND NEW.created_xact_id=pg_current_xact_id()
 AND (r.world_id,r.channel_id,r.ownership_generation,r.character_id,r.game_session_id)
   =(NEW.world_id,NEW.channel_id,NEW.ownership_generation,NEW.master_character_id,NEW.master_game_session_id);
 actor_binding=jsonb_build_object(
   'world',ARRAY(SELECT get_byte(uuid_send(NEW.world_id),n) FROM generate_series(0,15)n),
   'channel',ARRAY(SELECT get_byte(uuid_send(NEW.channel_id),n) FROM generate_series(0,15)n),
   'scope_generation',NEW.ownership_generation,'local_id',NEW.actor_local_id,'local_generation',NEW.actor_local_generation,
   'placement',ARRAY(SELECT get_byte(uuid_send(NEW.actor_placement_identity),n) FROM generate_series(0,15)n));
 master_binding=actor_binding || jsonb_build_object('local_id',NEW.master_actor_local_id,
   'local_generation',NEW.master_actor_local_generation,
   'placement',ARRAY(SELECT get_byte(uuid_send(NEW.master_placement_identity),n) FROM generate_series(0,15)n));
 expected=jsonb_build_object(
   'transaction',ARRAY(SELECT get_byte(uuid_send(NEW.transaction_id),n) FROM generate_series(0,15)n),
   'actor',actor_binding,'master',master_binding,
   'character',ARRAY(SELECT get_byte(uuid_send(NEW.master_character_id),n) FROM generate_series(0,15)n),
   'session',ARRAY(SELECT get_byte(uuid_send(NEW.master_game_session_id),n) FROM generate_series(0,15)n),
   'cell',jsonb_build_array(NEW.cell_x,NEW.cell_y,NEW.cell_z),
   'creature_key',NEW.creature_key,'creature_revision',NEW.creature_revision,'source',NEW.source_json);
 IF receipt_intent IS NULL OR NEW.binding_json IS DISTINCT FROM expected
 OR receipt_intent->'direct_companion' IS DISTINCT FROM expected
 OR receipt_intent->'companion' IS DISTINCT FROM 'null'::jsonb
 OR receipt_intent->'caster_origin'->'placement' IS DISTINCT FROM master_binding->'placement'
 OR EXISTS(SELECT 1 FROM game_spell_companion_acquisition_receipts WHERE transaction_id=NEW.transaction_id)
 OR EXISTS(SELECT 1 FROM game_spell_item_lines WHERE transaction_id=NEW.transaction_id AND operation_kind=3)
 OR (NEW.source_kind=1 AND (NEW.source_json->>'kind' IS DISTINCT FROM 'named'
      OR jsonb_typeof(NEW.source_json->'requested_name') IS DISTINCT FROM 'string'
      OR length(btrim(NEW.source_json->>'requested_name')) NOT BETWEEN 1 AND 512
      OR NEW.source_json IS DISTINCT FROM jsonb_build_object('kind','named','requested_name',NEW.source_json->>'requested_name')))
 OR (NEW.source_kind=2 AND (NEW.source_json->>'kind' IS DISTINCT FROM 'target'
      OR jsonb_typeof(NEW.source_json->'before_snapshot_digest') IS DISTINCT FROM 'array'
      OR jsonb_array_length(NEW.source_json->'before_snapshot_digest')<>32
      OR NEW.source_json->'before_snapshot_digest'='[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]'::jsonb
      OR NEW.source_json IS DISTINCT FROM jsonb_build_object('kind','target',
         'before_snapshot_digest',NEW.source_json->'before_snapshot_digest',
         'prior_master',NEW.source_json->'prior_master','prior_master_session',NEW.source_json->'prior_master_session')
      OR ((NEW.source_json->'prior_master'='null'::jsonb) IS DISTINCT FROM (NEW.source_json->'prior_master_session'='null'::jsonb)))) THEN
  RAISE EXCEPTION 'direct acquisition requires exact same-TX cast cost/current master/Creature source intent' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_direct_companion_proven AFTER INSERT ON game_spell_direct_companion_acquisition_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_direct_companion_proven();

CREATE FUNCTION game_spell_direct_companion_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (convert_from(NEW.intent,'UTF8')::jsonb)->'direct_companion' IS NOT NULL
 AND (convert_from(NEW.intent,'UTF8')::jsonb)->'direct_companion' <> 'null'::jsonb
 AND NOT EXISTS(SELECT 1 FROM game_spell_direct_companion_acquisition_receipts d
   WHERE d.transaction_id=NEW.transaction_id AND d.created_xact_id=pg_current_xact_id()
   AND d.created_xact_id=NEW.created_xact_id
   AND d.binding_json=(convert_from(NEW.intent,'UTF8')::jsonb)->'direct_companion') THEN
  RAISE EXCEPTION 'direct acquisition source intent requires its actual receipt in the same transaction' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_direct_companion_complete AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_direct_companion_complete();
CREATE TRIGGER game_spell_direct_companion_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
 ON game_spell_direct_companion_acquisition_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();
ALTER FUNCTION game_spell_direct_companion_proven() SET search_path FROM CURRENT;
ALTER FUNCTION game_spell_direct_companion_complete() SET search_path FROM CURRENT;
REVOKE ALL ON game_spell_direct_companion_acquisition_receipts FROM PUBLIC;
REVOKE ALL ON FUNCTION game_spell_direct_companion_proven(),game_spell_direct_companion_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_spell_direct_companion_acquisition_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_spell_direct_companion_acquisition_receipts TO oteryn_game_control;
