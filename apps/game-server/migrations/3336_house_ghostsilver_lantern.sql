-- Crystal 201189f8: data-global/scripts/quests/forgotten_knowledge/actions_lantern.lua:1-4.
-- Admit both lantern transitions on the existing House guard's function identity.
DO $ghostsilver_house_pair$
DECLARE
    body TEXT := pg_get_functiondef('game_item_action_house_pair(text,text)'::regprocedure);
    anchor CONSTANT TEXT := 'SELECT EXISTS(SELECT 1 FROM (VALUES';
BEGIN
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'House Item transformation pair layout changed';
    END IF;
    EXECUTE replace(body, anchor, $pairs$
 SELECT (p_before,p_after) IN (
    ('oteryn:item.tibia.i23734','oteryn:item.tibia.i23738'),
    ('oteryn:item.tibia.i23738','oteryn:item.tibia.i23734')
 ) OR EXISTS(SELECT 1 FROM (VALUES$pairs$);
END $ghostsilver_house_pair$;

-- Channel equipment uses Character custody. Both transitions keep the same
-- item, definition revision and Extra slot, with one ordinary USE receipt.
CREATE FUNCTION game_ghostsilver_toggle(p_key text) RETURNS text
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT CASE p_key WHEN 'oteryn:item.tibia.i23734' THEN 'oteryn:item.tibia.i23738'
                   WHEN 'oteryn:item.tibia.i23738' THEN 'oteryn:item.tibia.i23734' END
$$;

CREATE FUNCTION game_ghostsilver_equipped(r game_item_consume_transform_records) RETURNS boolean
LANGUAGE sql STABLE AS $$
 SELECT coalesce(r.source_kind=2 AND r.source_ordinal=10 AND NOT r.consume_source
    AND r.source_count=0 AND r.parent_item_instance_id IS NULL
    AND r.secondary_item_instance_id=r.source_item_instance_id AND r.before_secondary=r.before_item
    AND r.outputs='[]'::jsonb AND r.extra_inputs='[]'::jsonb AND r.content_digest IS NOT NULL
    AND r.before_item->>'quantity'='1' AND r.effects @> '{"kind":"carried_transform","require_head":false}'::jsonb
    AND (r.before_item->>'definition_production_key',r.recipe) IN (
       ('oteryn:item.tibia.i23734','oteryn:novelty-item.i23734.v1'),
       ('oteryn:item.tibia.i23738','oteryn:novelty-item.i23738.v1'))
    AND EXISTS (SELECT 1 FROM game_character_equipment_slots e
      WHERE e.slot=10 AND e.item_instance_id=r.source_item_instance_id AND e.world_id=r.world_id
        AND e.character_id=r.character_id AND to_jsonb(e)=r.source_location),false)
$$;

CREATE FUNCTION game_ghostsilver_equipped_apply(p_transaction uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE;
BEGIN
 SELECT * INTO STRICT r FROM game_item_consume_transform_records
  WHERE transaction_id=p_transaction AND created_xact_id=pg_current_xact_id();
 IF NOT game_ghostsilver_equipped(r) THEN
   RAISE EXCEPTION 'Lantern requires original equipped USE' USING ERRCODE='23514';
 END IF;
 UPDATE game_item_instances i SET definition_production_key=game_ghostsilver_toggle(i.definition_production_key),
    state_revision=i.state_revision+1,last_transaction_id=p_transaction
  WHERE i.item_instance_id=r.source_item_instance_id AND to_jsonb(i)=r.before_item;
 IF NOT FOUND THEN RAISE EXCEPTION 'Equipped lantern changed' USING ERRCODE='23514'; END IF;
END $$;

CREATE FUNCTION game_ghostsilver_equipped_complete(r game_item_consume_transform_records) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
 IF NOT game_ghostsilver_equipped(r) OR r.created_xact_id<>pg_current_xact_id()
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=r.source_item_instance_id
    AND i.last_transaction_id=r.transaction_id AND i.state_revision=(r.before_item->>'state_revision')::numeric+1
    AND i.definition_production_key=game_ghostsilver_toggle(r.before_item->>'definition_production_key')
    AND to_jsonb(i)-ARRAY['last_transaction_id','state_revision','definition_production_key']
      =r.before_item-ARRAY['last_transaction_id','state_revision','definition_production_key'])
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id
    AND item_instance_id=r.source_item_instance_id AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256
    AND created_xact_id=pg_current_xact_id() AND transaction_ordinal=1 AND transaction_count=1
    AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'Equipped lantern must toggle one instance in its original slot' USING ERRCODE='23514'; END IF;
END $$;

DO $equipped_lantern$
DECLARE body text; signature text; edit record;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_ghostsilver_toggle(text)',
     'game_ghostsilver_equipped(game_item_consume_transform_records)',
     'game_ghostsilver_equipped_apply(uuid)',
     'game_ghostsilver_equipped_complete(game_item_consume_transform_records)'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
   EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
 END LOOP;
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_source_custody';
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_source_custody;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_source_custody CHECK ('
   ||body||$shape$ OR coalesce(source_kind=2 AND source_ordinal=10 AND parent_item_instance_id IS NULL AND NOT consume_source
     AND effects @> '{"kind":"carried_transform","require_head":false}'::jsonb
     AND (before_item->>'definition_production_key',recipe) IN (
       ('oteryn:item.tibia.i23734','oteryn:novelty-item.i23734.v1'),
       ('oteryn:item.tibia.i23738','oteryn:novelty-item.i23738.v1')),false))$shape$;
 FOR edit IN SELECT * FROM (VALUES
  ('game_item_consume_transform_admit()', 'OR (NEW.source_kind=2 AND i.definition_production_key=',
    'OR game_ghostsilver_equipped(NEW) OR (NEW.source_kind=2 AND i.definition_production_key='),
  ('game_item_consume_transform_complete(uuid)', 'IF r.source_kind IN(3,4)',
   $complete$IF game_ghostsilver_equipped(r) THEN PERFORM game_ghostsilver_equipped_complete(r); RETURN; END IF;
    IF r.source_kind IN(3,4)$complete$),
  ('game_item_instance_guard()', 'WHERE r.source_kind=5 AND r.source_item_instance_id=OLD.item_instance_id AND r.before_item=',
   $guard$WHERE (r.source_kind=5 OR game_ghostsilver_equipped(r)) AND r.source_item_instance_id=OLD.item_instance_id AND r.before_item=$guard$),
  ('game_item_instance_guard()', 'AND NEW.definition_production_key=r.effects->''house_after''->>''key''',
   $key$AND NEW.definition_production_key=CASE WHEN r.source_kind=2 THEN game_ghostsilver_toggle(OLD.definition_production_key)
       ELSE r.effects->'house_after'->>'key' END$key$),
  ('game_item_instance_change_proven()', E'WHERE r.source_kind=5\n     AND r.source_item_instance_id=NEW.item_instance_id',
   $proof$WHERE (r.source_kind=5 OR game_ghostsilver_equipped(r))
     AND r.source_item_instance_id=NEW.item_instance_id$proof$),
  ('game_item_instance_change_proven()', 'PERFORM game_item_action_house_complete(NEW.last_transaction_id); RETURN NULL;',
   'PERFORM game_item_consume_transform_complete(NEW.last_transaction_id); RETURN NULL;'),
  ('game_item_action_house_custody(game_item_consume_transform_records)', 'AND c.slot=r.source_ordinal',
   $slot$AND (game_ghostsilver_toggle(r.before_item->>'definition_production_key') IS NULL OR c.slot=10)
    AND c.slot=r.source_ordinal$slot$)
 ) changes(signature,anchor,replacement) LOOP
   body:=pg_get_functiondef(edit.signature::regprocedure);
   IF (length(body)-length(replace(body,edit.anchor,'')))/length(edit.anchor)<>1 THEN
     RAISE EXCEPTION 'Equipped lantern guard layout changed in %',edit.signature;
   END IF;
   EXECUTE replace(body,edit.anchor,edit.replacement);
 END LOOP;
END $equipped_lantern$;
