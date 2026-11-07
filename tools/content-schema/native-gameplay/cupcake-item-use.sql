
-- BEGIN SRV7 source Cupcake USE on the mature item-use owner (cause579).
DO $migration$
DECLARE name TEXT; rule TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_use_records_cause_kind_check','game_item_use_cause_shape','game_item_use_house_source'] LOOP
  SELECT pg_get_expr(conbin,conrelid) INTO STRICT rule FROM pg_constraint WHERE conrelid='game_item_use_records'::regclass AND conname=name AND contype='c';
  EXECUTE format('ALTER TABLE game_item_use_records DROP CONSTRAINT %I, ADD CONSTRAINT %I CHECK ((%s) OR(cause_kind=579 AND shape=1 AND(source_house_key IS NULL OR game_house_scope_key_valid(source_house_key))))',name,name,rule);
 END LOOP;
END $migration$;
CREATE TABLE game_seasonal_cupcake_uses(
 transaction_id UUID PRIMARY KEY,
 source_kind SMALLINT NOT NULL CHECK(source_kind IN(1,2)),
 use_transaction_id UUID REFERENCES game_item_use_records(transaction_id),
 transform_transaction_id UUID REFERENCES game_item_consume_transform_records(transaction_id),
 CHECK((source_kind=1 AND use_transaction_id IS NOT NULL AND use_transaction_id=transaction_id AND transform_transaction_id IS NULL) OR(source_kind=2 AND transform_transaction_id IS NOT NULL AND transform_transaction_id=transaction_id AND use_transaction_id IS NULL)),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
 wire_binding BYTEA NOT NULL CHECK(octet_length(wire_binding)=32),
 item_id BIGINT NOT NULL CHECK(item_id IN(28484,28485,28486)),used_at TIMESTAMPTZ NOT NULL,
 conditions_sha256 BYTEA NOT NULL CHECK(octet_length(conditions_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),UNIQUE(game_session_id,command_id)
);
CREATE INDEX game_seasonal_cupcake_exhaustion ON game_seasonal_cupcake_uses(character_id,item_id,used_at DESC);
CREATE TRIGGER game_seasonal_cupcake_stamp BEFORE INSERT ON game_seasonal_cupcake_uses FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_seasonal_cupcake_immutable BEFORE UPDATE OR DELETE ON game_seasonal_cupcake_uses FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_seasonal_cupcake_no_truncate BEFORE TRUNCATE ON game_seasonal_cupcake_uses FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE FUNCTION game_seasonal_cupcake_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:cupcake:'||encode(uuid_send(NEW.character_id),'hex')||':'||NEW.item_id::text,0));
 NEW.used_at:=public.game_daily_effect_now();
 IF EXISTS(SELECT 1 FROM game_seasonal_cupcake_uses r WHERE r.character_id=NEW.character_id AND r.item_id=NEW.item_id AND r.used_at+interval '600 seconds'>NEW.used_at) THEN
  RAISE EXCEPTION 'Cupcake source exhaustion remains active' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_seasonal_cupcake_admit BEFORE INSERT ON game_seasonal_cupcake_uses FOR EACH ROW EXECUTE FUNCTION game_seasonal_cupcake_admit();
CREATE FUNCTION game_seasonal_cupcake_proven() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_seasonal_cupcake_uses%ROWTYPE; u game_item_use_records%ROWTYPE; t game_item_consume_transform_records%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_seasonal_cupcake_uses WHERE transaction_id=NEW.transaction_id;
 IF c.transaction_id IS NULL OR c.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'Cupcake source receipt must belong to the same transaction' USING ERRCODE='23514'; END IF;
 IF c.item_id=28486 AND NOT EXISTS(
  SELECT 1 FROM game_character_condition_snapshots s CROSS JOIN LATERAL jsonb_array_elements(s.snapshot->'conditions') condition
  WHERE s.character_id=c.character_id AND s.xmin=pg_current_xact_id()::xid
   AND condition->'definition'->>'key'='item.seasonal.cupcake.lemon'
   AND condition->'definition'->'values'='{"SourceAttributes":{"modifiers":{"melee":null,"distance":{"Add":10},"magic_points":null,"shielding":null,"fishing":null},"duration_ms":3600000}}'::jsonb
   AND jsonb_typeof(condition->'ends_at')='number' AND(condition->>'ends_at')::numeric BETWEEN 1 AND 3600000000) THEN
  RAISE EXCEPTION 'Lemon cupcake requires its exact one-hour distance+10 native condition' USING ERRCODE='23514'; END IF;
 IF c.item_id=28486 AND EXISTS(SELECT 1 FROM game_hireling_gourmet_state g WHERE g.character_id=c.character_id AND(g.attributes_item IS NOT NULL OR g.attributes_ms<>0)) THEN
  RAISE EXCEPTION 'Lemon FORCEUPDATE must retire the old gourmet attribute projection' USING ERRCODE='23514'; END IF;
 IF c.source_kind=2 THEN
  SELECT * INTO t FROM game_item_consume_transform_records WHERE transaction_id=c.transaction_id;
  IF t.transaction_id IS NULL OR t.created_xact_id<>pg_current_xact_id() OR t.character_id<>c.character_id OR t.game_session_id<>c.game_session_id OR t.command_id<>c.command_id
   OR t.source_kind NOT IN(3,5) OR NOT t.consume_source OR t.source_count<>1 OR t.outputs<>'[]'::jsonb
   OR t.before_item->>'definition_production_key'<>'oteryn:item.tibia.i'||c.item_id::text
   OR t.recipe<>'oteryn:seasonal.cupcake.'||c.item_id::text
   OR NOT EXISTS(SELECT 1 FROM game_character_condition_snapshots s WHERE s.character_id=c.character_id AND s.xmin=pg_current_xact_id()::xid AND sha256(convert_to(s.snapshot::text,'UTF8'))=c.conditions_sha256)
   THEN RAISE EXCEPTION 'Cupcake requires its exact native Ground/House burn and condition successor' USING ERRCODE='23514'; END IF;
  RETURN NULL;
 END IF;
 SELECT * INTO u FROM game_item_use_records WHERE transaction_id=NEW.transaction_id;
 IF u.transaction_id IS NULL OR u.created_xact_id<>pg_current_xact_id()
 OR u.cause_kind<>579 OR u.shape<>1 OR u.character_id<>c.character_id OR u.game_session_id<>c.game_session_id OR u.command_id<>c.command_id
 OR u.source_definition_family<>'Item' OR u.source_definition_production_key<>'oteryn:item.tibia.i'||c.item_id::text
 OR u.source_quantity_after<>u.source_quantity_before-1
 OR NOT EXISTS(SELECT 1 FROM game_character_condition_snapshots s WHERE s.character_id=c.character_id AND s.xmin=pg_current_xact_id()::xid
  AND sha256(convert_to(s.snapshot::text,'UTF8'))=c.conditions_sha256)
 OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.game_session_id=u.game_session_id AND s.character_id=u.character_id
  AND s.world_id=u.world_id AND s.current_generation=u.expected_session_generation AND s.scope_ownership_generation=u.runtime_scope_ownership_generation
  AND s.session_state IN(1,2) AND((u.source_house_key IS NULL AND s.runtime_scope_kind=1 AND s.runtime_scope_channel_id=u.channel_id AND s.runtime_scope_instance_id IS NULL)
   OR(u.source_house_key IS NOT NULL AND s.runtime_scope_kind=2 AND s.runtime_scope_house_key=u.source_house_key AND s.runtime_scope_channel_id IS NULL
    AND s.runtime_scope_instance_id=game_house_scope_instance_id(u.world_id,u.source_house_key) AND s.origin_channel_id=u.channel_id))) THEN
  RAISE EXCEPTION 'Cupcake requires its same-transaction native USE, condition snapshot and independent current scope' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_seasonal_cupcake_proven AFTER INSERT ON game_seasonal_cupcake_uses DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_seasonal_cupcake_proven();
CREATE CONSTRAINT TRIGGER game_seasonal_cupcake_native_proven AFTER INSERT ON game_item_use_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause_kind=579) EXECUTE FUNCTION game_seasonal_cupcake_proven();
CREATE CONSTRAINT TRIGGER game_seasonal_cupcake_transform_proven AFTER INSERT ON game_item_consume_transform_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.recipe IN('oteryn:seasonal.cupcake.28484','oteryn:seasonal.cupcake.28485','oteryn:seasonal.cupcake.28486')) EXECUTE FUNCTION game_seasonal_cupcake_proven();
REVOKE ALL ON game_seasonal_cupcake_uses FROM PUBLIC;
GRANT SELECT,INSERT ON game_seasonal_cupcake_uses TO oteryn_game_runtime;
GRANT SELECT ON game_seasonal_cupcake_uses TO oteryn_game_control;
DO $migration$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_seasonal_cupcake_admit()','game_seasonal_cupcake_proven()'] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
 END LOOP;
END $migration$;
-- END SRV7 source Cupcake USE.
