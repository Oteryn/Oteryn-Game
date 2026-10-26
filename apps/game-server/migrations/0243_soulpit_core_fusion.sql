-- SOULPIT: extend CRAFT-1's existing receipt and proof guards to a second input.
-- Existing one-input recipes retain source_units=1 and no secondary snapshot.
ALTER TABLE game_item_craft_records
 ADD COLUMN source_units SMALLINT NOT NULL DEFAULT 1,
 ADD COLUMN secondary_item_instance_id UUID REFERENCES game_item_instances,
 ADD COLUMN secondary_parent_item_instance_id UUID REFERENCES game_item_instances,
 ADD COLUMN secondary_ordinal NUMERIC(20,0),
 ADD COLUMN secondary_before JSONB,
 ADD CONSTRAINT game_item_craft_input_shape CHECK (
   (recipe NOT IN ('oteryn:soulpit.fuse','oteryn:soulpit.prism','oteryn:soulpit.prism-ominous','oteryn:soulpit.exalted') AND source_units=1
     AND secondary_item_instance_id IS NULL AND secondary_parent_item_instance_id IS NULL
     AND secondary_ordinal IS NULL AND secondary_before IS NULL)
   OR (recipe IN ('oteryn:soulpit.fuse','oteryn:soulpit.prism','oteryn:soulpit.prism-ominous','oteryn:soulpit.exalted')
     AND jsonb_array_length(outputs)=(CASE WHEN recipe='oteryn:soulpit.prism-ominous' THEN 2 ELSE 1 END)
     AND (outputs->0->>'count') IS NOT NULL AND (outputs->0->>'count')::int=1
     AND (recipe<>'oteryn:soulpit.prism-ominous' OR ((outputs->1->>'count') IS NOT NULL AND (outputs->1->>'count')::int=1)) AND (
       (source_units=2 AND recipe='oteryn:soulpit.fuse' AND secondary_item_instance_id IS NULL
         AND secondary_parent_item_instance_id IS NULL AND secondary_ordinal IS NULL AND secondary_before IS NULL)
       OR (source_units=1 AND secondary_item_instance_id IS NOT NULL
         AND secondary_item_instance_id<>source_item_instance_id
         AND secondary_parent_item_instance_id IS NOT NULL AND secondary_parent_item_instance_id=parent_item_instance_id
         AND secondary_ordinal IS NOT NULL AND secondary_ordinal>0
         AND secondary_before IS NOT NULL AND jsonb_typeof(secondary_before)='object'))));

CREATE FUNCTION game_item_craft_secondary_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE;
BEGIN
 IF NEW.secondary_item_instance_id IS NULL THEN RETURN NEW; END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.secondary_item_instance_id FOR UPDATE;
 IF i.item_instance_id IS NULL OR to_jsonb(i)<>NEW.secondary_before OR i.lifecycle<>1
    OR i.quantity<1 OR i.world_id<>NEW.world_id OR i.definition_family<>'Item'
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
                    ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
                   WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id
                     AND e.world_id=NEW.world_id AND b.world_id=NEW.world_id
                     AND e.parent_item_instance_id=NEW.secondary_parent_item_instance_id
                     AND e.placement_ordinal=NEW.secondary_ordinal)
 THEN RAISE EXCEPTION 'fusion requires unchanged plain carried target' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_item_craft_secondary_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_item_craft_records%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_craft_records WHERE transaction_id=p_transaction;
 IF r.secondary_item_instance_id IS NULL THEN RETURN; END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.secondary_item_instance_id;
 IF i.item_instance_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
    OR i.quantity<>(r.secondary_before->>'quantity')::bigint-1
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
    OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       (r.secondary_before-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
    OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries
         WHERE item_instance_id=i.item_instance_id AND character_id=r.character_id
           AND parent_item_instance_id=r.secondary_parent_item_instance_id AND placement_ordinal=r.secondary_ordinal))
 THEN RAISE EXCEPTION 'incomplete atomic fusion target' USING ERRCODE='23514'; END IF;
END $$;

-- Modify only the exact CRAFT-1 proof clauses; preserve every other owner's guard.
DO $migration$
DECLARE body TEXT; changed TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
 IF strpos(body,'OR i.quantity<1')=0 OR strpos(body,'i.quantity=1 AND o=NEW.outputs->0')=0
 THEN RAISE EXCEPTION 'missing exact craft admission clauses'; END IF;
 changed:=replace(replace(body,'OR i.quantity<1','OR i.quantity<NEW.source_units'),
                  'i.quantity=1 AND o=NEW.outputs->0','i.quantity=NEW.source_units AND o=NEW.outputs->0');
 IF changed=body THEN RAISE EXCEPTION 'missing craft admission clauses'; END IF;
 EXECUTE changed;
 body:=pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
 IF strpos(body,$old$(r.before_item->>'quantity')::bigint-1$old$)=0
    OR strpos(body,'last_transaction_id=r.transaction_id)<>1')=0
    OR strpos(body,'FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP')=0
 THEN RAISE EXCEPTION 'missing exact craft completion clauses'; END IF;
 changed:=replace(body,$old$(r.before_item->>'quantity')::bigint-1$old$,$new$(r.before_item->>'quantity')::bigint-r.source_units$new$);
 changed:=replace(changed,'last_transaction_id=r.transaction_id)<>1',
   'last_transaction_id=r.transaction_id)<>(CASE WHEN r.secondary_item_instance_id IS NULL THEN 1 ELSE 2 END)');
 changed:=replace(changed,'FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP',
   'PERFORM game_item_craft_secondary_complete(p_transaction); FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP');
 IF changed=body THEN RAISE EXCEPTION 'missing craft completion clauses'; END IF;
 EXECUTE changed;
 body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 changed:=replace(body,E'r.transaction_id=NEW.last_transaction_id AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN\n     IF NEW.quantity<>OLD.quantity-1',
   E'r.transaction_id=NEW.last_transaction_id AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN\n     IF NEW.quantity<>OLD.quantity-(SELECT source_units FROM game_item_craft_records WHERE transaction_id=NEW.last_transaction_id)');
 IF changed=body THEN RAISE EXCEPTION 'missing craft decrement clause'; END IF;
 EXECUTE changed;
END $migration$;

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_container_entry_removal_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected proof body: %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_craft_records r WHERE r.secondary_item_instance_id=OLD.item_instance_id
      AND r.transaction_id=NEW.last_transaction_id AND r.secondary_before=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.quantity<>OLD.quantity-1 OR NEW.lifecycle<>(CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END)
       OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
          (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
     THEN RAISE EXCEPTION 'fusion decrements exactly one target unit' USING ERRCODE='23514'; END IF;
     RETURN NEW;
    END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_craft_records r WHERE r.secondary_item_instance_id=NEW.item_instance_id
      AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_item_craft_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_craft_records r JOIN game_item_instances i
       ON i.item_instance_id=r.secondary_item_instance_id WHERE r.secondary_item_instance_id=OLD.item_instance_id
       AND r.character_id=OLD.character_id AND r.secondary_parent_item_instance_id=OLD.parent_item_instance_id
       AND r.secondary_ordinal=OLD.placement_ordinal AND r.world_id=OLD.world_id
       AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id
       AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
CREATE TRIGGER game_item_craft_secondary_admit BEFORE INSERT ON game_item_craft_records
 FOR EACH ROW EXECUTE FUNCTION game_item_craft_secondary_admit();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_item_craft_secondary_admit() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_craft_secondary_complete(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_item_craft_secondary_admit(),game_item_craft_secondary_complete(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_craft_secondary_complete(UUID) TO oteryn_game_runtime;

-- Server entropy stays inside the item owner transaction. Only the committed outcome is retained.
CREATE FUNCTION game_soulpit_entropy() RETURNS bytea LANGUAGE SQL VOLATILE AS $$
 SELECT sha256(uuid_send(game_character_uuid_v7()))
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_soulpit_entropy() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_soulpit_entropy() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_soulpit_entropy() TO oteryn_game_runtime;
