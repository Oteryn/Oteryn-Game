-- W09 / Crystal 201189f8 data/scripts/actions/items/drome_cube.lua:1-39.
-- Reuse the item-use one-unit TRANSFORM and all of its custody/audit guards.
-- Owner ANSWER 6544857316: a full carried destination refuses before debit;
-- no ground fallback and no Cube cooldown. Concoction activation is separate.
-- Rollback is forward-only: disable the Cube dispatcher; retain receipts.
DO $$
DECLARE
 constraint_name TEXT;
 previous_rule TEXT;
BEGIN
 FOREACH constraint_name IN ARRAY ARRAY[
   'game_item_use_records_cause_kind_check','game_item_use_cause_shape'
 ] LOOP
   SELECT pg_get_expr(c.conbin,c.conrelid) INTO STRICT previous_rule
     FROM pg_constraint c
    WHERE c.conrelid='game_item_use_records'::regclass
      AND c.conname=constraint_name AND c.contype='c';
   EXECUTE format(
     'ALTER TABLE game_item_use_records DROP CONSTRAINT %I, ADD CONSTRAINT %I CHECK ((%s) OR (cause_kind=578 AND shape IN (3,4)))',
     constraint_name,constraint_name,previous_rule);
 END LOOP;
END $$;

CREATE TABLE game_drome_cube_uses (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_use_records(transaction_id)
   DEFERRABLE INITIALLY DEFERRED,
 wire_binding BYTEA NOT NULL CHECK(octet_length(wire_binding)=32),
 reward_index SMALLINT NOT NULL CHECK(reward_index BETWEEN 0 AND 19),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);

-- Both directions are checked at commit. Neither a bare burn nor a proof without
-- its item-use record can commit. The database independently recomputes the draw.
CREATE FUNCTION game_drome_cube_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER
SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE
 u RECORD;
 p RECORD;
 entropy BYTEA;
 draw SMALLINT;
 n INTEGER;
BEGIN
 SELECT * INTO u FROM game_item_use_records WHERE transaction_id=NEW.transaction_id;
 IF NOT FOUND THEN
   RAISE EXCEPTION 'drome cube use missing' USING ERRCODE='23514';
 END IF;
 SELECT * INTO p FROM game_drome_cube_uses WHERE transaction_id=NEW.transaction_id;
 IF NOT FOUND THEN
   RAISE EXCEPTION 'drome cube proof missing' USING ERRCODE='23514';
 END IF;
 entropy := sha256(convert_to('oteryn.drome-cube.reward.v1','UTF8')
                  || uuid_send(u.source_item_instance_id)
                  || int8send(u.source_quantity_before::BIGINT));
 draw := NULL;
 FOR n IN 0..31 LOOP
   IF get_byte(entropy,n)<240 THEN
     draw := get_byte(entropy,n)%20;
     EXIT;
   END IF;
 END LOOP;
 IF draw IS NULL OR p.reward_index IS DISTINCT FROM draw
    OR u.cause_kind IS DISTINCT FROM 578
    OR u.shape NOT IN (3,4)
    OR u.source_definition_family IS DISTINCT FROM 'Item'
    OR u.source_definition_production_key IS DISTINCT FROM 'oteryn:item.tibia.i36827'
    OR u.flask_definition_family IS DISTINCT FROM 'Item'
    OR u.flask_definition_production_key IS DISTINCT FROM
       ('oteryn:item.tibia.i' || (36723+draw)::TEXT)
    OR u.flask_quantity_after IS DISTINCT FROM COALESCE(u.flask_quantity_before,0)+1
    OR u.created_xact_id IS DISTINCT FROM p.created_xact_id
    OR p.created_xact_id IS DISTINCT FROM pg_current_xact_id()
 THEN
   RAISE EXCEPTION 'drome cube transform and proof differ' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;

CREATE CONSTRAINT TRIGGER game_drome_cube_proof_has_use
 AFTER INSERT ON game_drome_cube_uses DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_drome_cube_consistent();
CREATE CONSTRAINT TRIGGER game_drome_cube_use_has_proof
 AFTER INSERT ON game_item_use_records DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW WHEN(NEW.cause_kind=578)
 EXECUTE FUNCTION game_drome_cube_consistent();
CREATE TRIGGER game_drome_cube_uses_immutable
 BEFORE UPDATE OR DELETE ON game_drome_cube_uses
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_drome_cube_uses_no_truncate
 BEFORE TRUNCATE ON game_drome_cube_uses
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_drome_cube_uses_stamp
 BEFORE INSERT ON game_drome_cube_uses
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_drome_cube_consistent() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_drome_cube_uses FROM PUBLIC;
REVOKE ALL ON FUNCTION game_drome_cube_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_drome_cube_uses TO oteryn_game_runtime;
