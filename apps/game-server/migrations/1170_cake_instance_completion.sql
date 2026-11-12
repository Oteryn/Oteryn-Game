-- Follow the season lane's 1160-1169 migrations. The immutable transform
-- record's mandatory deferred trigger still proves the entire final donation.
-- Each additional Item callback proves only its own entry, rather than running
-- the full 200-input proof again. No transaction memo can outlive later writes.
DO $migration$
DECLARE body TEXT; begin_at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Cake instance completion boundary'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 DECLARE r game_item_consume_transform_records%ROWTYPE;
         e JSONB; i game_item_instances%ROWTYPE;
 BEGIN
  SELECT * INTO r FROM game_item_consume_transform_records
   WHERE transaction_id=NEW.last_transaction_id
    AND recipe='oteryn:cake.baker-substances' AND created_xact_id=pg_current_xact_id();
  IF FOUND AND NEW.item_instance_id<>r.source_item_instance_id
   AND NEW.item_instance_id<>r.secondary_item_instance_id THEN
   -- Admission rejects duplicate inputs; the immutable record bounds this
   -- lookup to 199 entries and its CHECK already proves the donation shape.
   SELECT value INTO e FROM jsonb_array_elements(r.extra_inputs)
    WHERE (value->>'item')::uuid=NEW.item_instance_id;
   IF FOUND THEN
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id;
    IF i.item_instance_id IS NULL
     OR i.quantity<>(e->'before'->>'quantity')::bigint-(e->>'count')::integer
     OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
     OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
     OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
        ((e->'before')-ARRAY['quantity','lifecycle','last_transaction_id'])
     OR (i.quantity=0 AND
        (EXISTS(SELECT 1 FROM game_item_container_entries c WHERE c.item_instance_id=i.item_instance_id)
         OR EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.item_instance_id=i.item_instance_id)
         OR EXISTS(SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=i.item_instance_id)))
     OR (i.quantity>0 AND NOT game_cake_extra_custody(r,e))
    THEN RAISE EXCEPTION 'Cake donation lacks exact decrement and custody' USING ERRCODE='23514'; END IF;
    RETURN NULL;
   END IF;
  END IF;
 END;
$arm$ from begin_at for 5);
END $migration$;
