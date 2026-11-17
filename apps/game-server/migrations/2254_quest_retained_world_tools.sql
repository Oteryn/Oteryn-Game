-- M13: retained tools are read participants, never fictional BURN/MINT events.
CREATE TABLE game_quest_world_tool_reads (
 character_id UUID NOT NULL,
 cause_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
 transition_key TEXT NOT NULL CHECK(octet_length(transition_key) BETWEEN 1 AND 128),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
 proof BYTEA NOT NULL CHECK(octet_length(proof) BETWEEN 1 AND 16384),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
 FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
 REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_quest_world_tool_reads_item ON game_quest_world_tool_reads(item_instance_id);
CREATE FUNCTION game_quest_world_tool_read_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p JSONB; source_bytes JSONB;
BEGIN
 p:=convert_from(NEW.proof,'UTF8')::JSONB;
 SELECT jsonb_agg(get_byte(uuid_send(NEW.item_instance_id),n) ORDER BY n) INTO source_bytes FROM generate_series(0,15) bytes(n);
 IF NEW.created_xact_id<>pg_current_xact_id()
  OR p #>> '{callback,key}' IS DISTINCT FROM NEW.transition_key
  OR p #>> '{callback,trigger}' IS DISTINCT FROM 'use'
  OR p #>> '{callback,requires_world_admission}' IS DISTINCT FROM 'true'
  OR p #>> '{callback,item_on_target,selected_instance}' IS DISTINCT FROM 'true'
  OR p #>> '{callback,item_on_target,reusable}' IS DISTINCT FROM 'true'
  OR p #>> '{callback,item_on_target,count}' IS DISTINCT FROM '1'
  OR p #> '{context,selected_tool,source}' IS DISTINCT FROM source_bytes
  OR jsonb_typeof(p #> '{context,selected_tool,intent}') IS DISTINCT FROM 'array'
  OR jsonb_array_length(p #> '{context,selected_tool,intent}') NOT BETWEEN 1 AND 4096
 THEN RAISE EXCEPTION 'retained World tool lacks bound read terms' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(p #> '{context,selected_tool,intent}') part(value)
   WHERE CASE WHEN part.value::TEXT ~ '^[0-9]{1,3}$' THEN part.value::TEXT::INTEGER NOT BETWEEN 0 AND 255 ELSE true END)
 THEN RAISE EXCEPTION 'retained World tool lacks bounded ingress bytes' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_character_quest_receipts q WHERE
  (q.character_id,q.cause_id,q.cause_ordinal,q.transition_key)=(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
  AND q.created_xact_id=pg_current_xact_id() AND q.cause_kind='use' AND q.quest_key=p #>> '{callback,quest}')
 THEN RAISE EXCEPTION 'retained World tool lacks atomic quest receipt' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,NEW.world_id) c JOIN game_item_instances i ON i.item_instance_id=c.item
  WHERE i.item_instance_id=NEW.item_instance_id AND i.world_id=NEW.world_id AND i.lifecycle=1 AND i.quantity>=1
   AND i.definition_family='Item' AND (i.definition_production_key=p #>> '{callback,item_on_target,item}'
    OR i.definition_production_key IN (SELECT jsonb_array_elements_text(coalesce(p #> '{callback,item_on_target,alternatives}','[]'::JSONB))))
   AND to_jsonb(i)=p->'before' AND c.location=p->'location' AND c.kind=p->>'kind')
 THEN RAISE EXCEPTION 'retained World tool changed custody or attributes' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_world_tool_read_complete AFTER INSERT ON game_quest_world_tool_reads
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_world_tool_read_complete();
CREATE TRIGGER game_quest_world_tool_read_immutable BEFORE UPDATE OR DELETE ON game_quest_world_tool_reads
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_world_tool_read_no_truncate BEFORE TRUNCATE ON game_quest_world_tool_reads EXECUTE FUNCTION game_item_reject_truncate();
DO $migration$ BEGIN
 EXECUTE format('ALTER FUNCTION game_quest_world_tool_read_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON game_quest_world_tool_reads FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_world_tool_reads TO oteryn_game_runtime;
GRANT SELECT ON game_quest_world_tool_reads TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_quest_world_tool_read_complete() FROM PUBLIC;
