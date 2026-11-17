-- W19 source cosmetic component progress. Account grants remain in the existing owner.
CREATE TABLE game_character_item_cosmetic_progress (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 component_key TEXT NOT NULL CHECK(octet_length(component_key) BETWEEN 1 AND 128),
 contributions BIGINT NOT NULL CHECK(contributions BETWEEN 1 AND 4294967295),
 last_transaction_id UUID NOT NULL REFERENCES game_item_consume_transform_records(transaction_id)
   DEFERRABLE INITIALLY DEFERRED,
 PRIMARY KEY(character_id,component_key)
);

CREATE INDEX game_character_item_cosmetic_progress_receipt ON game_character_item_cosmetic_progress(last_transaction_id);

CREATE FUNCTION game_item_cosmetic_progress_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 IF TG_OP='INSERT' AND NEW.contributions=1 THEN RETURN NEW; END IF;
 IF TG_OP='UPDATE' AND (NEW.character_id,NEW.component_key)=(OLD.character_id,OLD.component_key)
    AND NEW.contributions=OLD.contributions+1 AND NEW.last_transaction_id<>OLD.last_transaction_id
 THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'cosmetic progress requires one source contribution' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_item_cosmetic_progress_guard BEFORE INSERT OR UPDATE OR DELETE
 ON game_character_item_cosmetic_progress FOR EACH ROW EXECUTE FUNCTION game_item_cosmetic_progress_guard();
CREATE TRIGGER game_item_cosmetic_progress_no_truncate BEFORE TRUNCATE
 ON game_character_item_cosmetic_progress EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_item_cosmetic_consistent() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; e JSONB; g JSONB; p JSONB; n BIGINT;
BEGIN
 IF TG_TABLE_NAME='game_item_consume_transform_records' THEN r:=NEW;
 ELSIF TG_TABLE_NAME='game_character_item_cosmetic_progress' THEN
   SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=NEW.last_transaction_id;
 ELSE
   SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=encode(NEW.source_event_id,'hex')::uuid;
 END IF;
 e:=r.effects->'cosmetic'; p:=e->'progress';
 IF TG_TABLE_NAME='game_character_item_cosmetic_progress' THEN
   -- Trigger row shapes differ: access progress-only fields in their own statement.
   -- SQL boolean short-circuiting does not prevent PL/pgSQL from resolving NEW fields.
   IF NEW.character_id IS DISTINCT FROM r.character_id
      OR NEW.component_key IS DISTINCT FROM p->>'key'
      OR NEW.contributions IS DISTINCT FROM (p->>'after')::bigint
      OR (SELECT count(*) FROM game_character_item_cosmetic_progress WHERE last_transaction_id=r.transaction_id)<>1
   THEN RAISE EXCEPTION 'cosmetic progress row differs from action' USING ERRCODE='23514'; END IF;
 END IF;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
    OR jsonb_typeof(e) IS DISTINCT FROM 'object'
    OR e->>'provenance' IS DISTINCT FROM 'crystal-item-cosmetics-201189f-v1'
    OR (e->>'consumed')::boolean IS DISTINCT FROM r.consume_source
    OR r.recipe IS DISTINCT FROM ('oteryn:item-cosmetic.i'||(e->>'source_item_id')||'.v1')
    OR r.before_item->>'definition_production_key' IS DISTINCT FROM ('oteryn:item.tibia.i'||(e->>'source_item_id'))
    OR jsonb_typeof(e->'grants') IS DISTINCT FROM 'array' OR jsonb_array_length(e->'grants')>2
    OR jsonb_array_length(r.outputs)<>0
    OR (NOT r.consume_source AND (jsonb_array_length(e->'grants')<>0 OR p IS DISTINCT FROM 'null'::jsonb))
 THEN RAISE EXCEPTION 'cosmetic effect lacks matching item action' USING ERRCODE='23514'; END IF;
 IF TG_TABLE_NAME='game_account_appearance_unlock_receipts' THEN
   IF NEW.granting_character_id<>r.character_id OR NEW.content_revision<>e->>'provenance'
      OR NEW.account_id<>(SELECT account_id FROM game_character_roots WHERE character_id=r.character_id)
      OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(e->'grants') a WHERE a->>'grant_key'=NEW.unlock_key)
   THEN RAISE EXCEPTION 'cosmetic grant differs from action' USING ERRCODE='23514'; END IF;
 END IF;
 FOR g IN SELECT value FROM jsonb_array_elements(e->'grants') LOOP
   IF NOT EXISTS(SELECT 1 FROM game_account_appearance_unlock_receipts a
      JOIN game_account_appearance_unlocks f USING(grant_occurrence_id,account_id,unlock_key)
      WHERE a.source_kind='item_action' AND a.source_event_id=uuid_send(r.transaction_id)
        AND a.unlock_key=g->>'grant_key' AND a.content_revision=e->>'provenance'
        AND a.granting_character_id=r.character_id
        AND a.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=r.character_id))
   THEN RAISE EXCEPTION 'cosmetic item spend lacks earned fact' USING ERRCODE='23514'; END IF;
 END LOOP;
 SELECT count(*) INTO n FROM game_account_appearance_unlock_receipts
   WHERE source_kind='item_action' AND source_event_id=uuid_send(r.transaction_id);
 IF n<>jsonb_array_length(e->'grants') THEN
   RAISE EXCEPTION 'cosmetic action has extra grants' USING ERRCODE='23514';
 END IF;
 IF p IS DISTINCT FROM 'null'::jsonb THEN
   IF NOT r.consume_source OR jsonb_typeof(p) IS DISTINCT FROM 'object'
      OR (p->>'after')::bigint IS DISTINCT FROM (p->>'before')::bigint+1
      OR NOT EXISTS(SELECT 1 FROM game_character_item_cosmetic_progress c
        WHERE c.character_id=r.character_id AND c.component_key=p->>'key'
          AND c.contributions=(p->>'after')::bigint AND c.last_transaction_id=r.transaction_id)
   THEN RAISE EXCEPTION 'cosmetic contribution lacks matching progress' USING ERRCODE='23514'; END IF;
 ELSIF TG_TABLE_NAME='game_character_item_cosmetic_progress' THEN
   RAISE EXCEPTION 'cosmetic progress lacks source contribution' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_item_cosmetic_receipt_consistent AFTER INSERT
 ON game_item_consume_transform_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
 WHEN(NEW.effects ? 'cosmetic') EXECUTE FUNCTION game_item_cosmetic_consistent();
CREATE CONSTRAINT TRIGGER game_item_cosmetic_progress_consistent AFTER INSERT OR UPDATE
 ON game_character_item_cosmetic_progress DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
 EXECUTE FUNCTION game_item_cosmetic_consistent();
CREATE CONSTRAINT TRIGGER game_item_cosmetic_grant_consistent AFTER INSERT
 ON game_account_appearance_unlock_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
 WHEN(NEW.source_kind='item_action') EXECUTE FUNCTION game_item_cosmetic_consistent();
REVOKE ALL ON game_character_item_cosmetic_progress FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_item_cosmetic_progress TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_item_cosmetic_progress_guard() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_cosmetic_consistent() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_cosmetic_progress_guard(),game_item_cosmetic_consistent() TO oteryn_game_runtime;



DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_item_cosmetic_progress_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_cosmetic_consistent() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
