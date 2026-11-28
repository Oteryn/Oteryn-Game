-- Rainbow Shield (TibiaWiki Fandom archive): four variants, 15 minutes each.
-- Legacy receipts predate decay in outputs; use the same clock as a new craft.
CREATE FUNCTION game_rainbow_shield_duration(p_key TEXT,p_revision TEXT) RETURNS BIGINT
LANGUAGE sql IMMUTABLE AS $$
 SELECT CASE WHEN p_revision='definition-r1' AND p_key IN
  ('oteryn:item.tibia.i8078','oteryn:item.tibia.i8079','oteryn:item.tibia.i8080','oteryn:item.tibia.i8081')
  THEN 900000::bigint END
$$;

-- Live enchanted gem uses reuse the atomic craft receipt and its completion proof.
-- Admission applies only to new receipts. Existing receipts retain their replay semantics.
CREATE OR REPLACE FUNCTION game_item_enchanting_result(s INTEGER,t INTEGER) RETURNS INTEGER
LANGUAGE sql IMMUTABLE SECURITY DEFINER AS $$
 SELECT CASE
 WHEN t=8077 THEN (ARRAY[8078,8079,8081,8080])[array_position(ARRAY[676,675,677,678],s)]
 WHEN s=676 AND t=9020 THEN 9019
 END
$$;

CREATE OR REPLACE FUNCTION game_item_enchanting_admit(r game_item_craft_records) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d JSONB; i game_item_instances%ROWTYPE; o JSONB; s INTEGER; t INTEGER;
BEGIN
 IF r.recipe<>'oteryn:enchanting.weapon'
  OR NOT r.consume_source OR jsonb_typeof(r.enchanting->'debits') IS DISTINCT FROM 'array'
  OR jsonb_array_length(r.enchanting->'debits')<>2
  OR jsonb_array_length(r.outputs)<>1
  OR (r.enchanting->'debits'->0->'before') IS DISTINCT FROM r.before_item
  OR (r.before_item->>'item_instance_id')::uuid IS DISTINCT FROM r.source_item_instance_id
  OR (r.enchanting->'debits'->0->>'parent')::uuid IS DISTINCT FROM r.parent_item_instance_id
  OR (r.enchanting->'debits'->0->>'ordinal')::numeric IS DISTINCT FROM r.source_ordinal
 THEN RAISE EXCEPTION 'invalid enchanted gem proof' USING ERRCODE='23514'; END IF;
 FOR d IN SELECT value FROM jsonb_array_elements(r.enchanting->'debits') LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(d->'before'->>'item_instance_id')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM d->'before'
   OR i.world_id<>r.world_id OR i.definition_family<>'Item' OR i.lifecycle<>1
   OR (d->>'count')::bigint NOT BETWEEN 1 AND i.quantity
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   -- Expiry keeps a spent timed row on the re-enchantable base item.
   OR EXISTS(SELECT 1 FROM game_item_timed_states ts WHERE ts.item_instance_id=i.item_instance_id
     AND (i.definition_production_key NOT IN ('oteryn:item.tibia.i8077','oteryn:item.tibia.i9020')
      OR ts.charges IS NOT NULL OR ts.remaining_ms IS NOT NULL OR ts.deadline_at IS NOT NULL))
   OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
      ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
      WHERE e.item_instance_id=i.item_instance_id AND e.world_id=r.world_id AND b.world_id=r.world_id
       AND e.character_id=r.character_id AND e.parent_item_instance_id=(d->>'parent')::uuid
       AND e.placement_ordinal=(d->>'ordinal')::numeric)
  THEN RAISE EXCEPTION 'enchanting requires plain carried inputs' USING ERRCODE='23514'; END IF;
 END LOOP;
 s:=substring(r.before_item->>'definition_production_key' FROM '^oteryn:item.tibia.i([0-9]+)$')::integer;
 t:=substring(r.enchanting->'debits'->1->'before'->>'definition_production_key' FROM '^oteryn:item.tibia.i([0-9]+)$')::integer;
 o:=r.outputs->0;
 IF (r.enchanting->'debits'->0->>'count')::bigint IS DISTINCT FROM 1
  OR (r.enchanting->'debits'->1->>'count')::bigint IS DISTINCT FROM 1
  OR (r.enchanting->'debits'->1->'before'->>'quantity')::bigint IS DISTINCT FROM 1
  OR (r.enchanting->'debits'->1->'before'->>'item_instance_id')::uuid=r.source_item_instance_id
  OR (o->>'count')::bigint IS DISTINCT FROM 1 OR o->>'charges' IS NOT NULL
  OR r.enchanting->'cost' IS DISTINCT FROM 'null'::jsonb
 THEN RAISE EXCEPTION 'invalid enchanted gem debits' USING ERRCODE='23514'; END IF;
 IF s IS NULL OR t IS NULL OR game_item_enchanting_result(s,t) IS NULL
  OR o->>'key' IS DISTINCT FROM 'oteryn:item.tibia.i'||game_item_enchanting_result(s,t)
  OR NOT game_character_is_uuid_v7((o->>'item')::uuid)
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(o->>'item')::uuid)
  OR coalesce(o->>'revision','')='' OR (o->>'ordinal')::numeric<=0
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id AND placement_ordinal=(o->>'ordinal')::numeric)
 THEN RAISE EXCEPTION 'invalid declared enchanting output' USING ERRCODE='23514'; END IF;
 IF t=8077 THEN
  IF (r.enchanting->>'content_digest' ~ '^[0-9a-f]{64}$') IS NOT TRUE
   OR (o->'decay'->>'duration_millis')::bigint IS DISTINCT FROM game_rainbow_shield_duration(o->>'key',o->>'revision')
   OR game_rainbow_shield_duration(o->>'key',o->>'revision') IS NULL
   OR o->'decay'->'target' IS DISTINCT FROM jsonb_build_object('family','Item',
      'key','oteryn:item.tibia.i8077','revision',r.enchanting->'debits'->1->'before'->>'definition_revision_ref')
  THEN RAISE EXCEPTION 'invalid rainbow shield deadline proof' USING ERRCODE='23514'; END IF;
 ELSIF o->'decay' IS NOT NULL AND o->'decay'<>'null'::jsonb THEN
  RAISE EXCEPTION 'unexpected enchanted gem deadline' USING ERRCODE='23514';
 END IF;
END $$;

DO $migration$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_enchanting_result(integer,integer)','game_item_enchanting_admit(game_item_craft_records)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
 END LOOP;
END $migration$;

-- Reuse the absolute deadline owner for all four shield outputs, with the craft receipt
-- as same-transaction source. The completion proof also requires the schedule to exist.
DO $migration$
DECLARE body TEXT; arm TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing original temporal mint proof'; END IF;
 arm:=$arm$
  IF EXISTS(SELECT 1 FROM game_item_craft_records c JOIN game_item_instances i
      ON i.item_instance_id=(c.outputs->0->>'item')::uuid
    WHERE c.transaction_id=NEW.source_transaction_id AND c.recipe='oteryn:enchanting.weapon'
      AND c.enchanting->'debits'->1->'before'->>'definition_production_key'='oteryn:item.tibia.i8077'
      AND NEW.created_xact_id=pg_current_xact_id()
      AND i.item_instance_id=NEW.item_instance_id AND i.minted_transaction_id=c.transaction_id
      AND i.world_id=c.world_id AND i.lifecycle=1 AND i.quantity=1 AND NEW.source_ordinal=1
      AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
        =(NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
      AND (c.outputs->0->>'key',c.outputs->0->>'revision')=(NEW.definition_production_key,NEW.definition_revision)
      AND c.enchanting->>'content_digest'=encode(NEW.content_digest,'hex')
      AND NEW.expires_at_unix_ms=c.occurred_at+NEW.duration_millis
      AND (
       (c.created_xact_id=NEW.created_xact_id AND
        c.outputs->0->'decay'=jsonb_build_object('duration_millis',NEW.duration_millis,
         'target',jsonb_build_object('family',NEW.target_family,'key',NEW.target_production_key,'revision',NEW.target_revision)))
       OR (c.created_xact_id<>NEW.created_xact_id
        AND c.outputs->0->>'decay' IS NULL
        AND NEW.duration_millis=game_rainbow_shield_duration(NEW.definition_production_key,NEW.definition_revision)
        AND (NEW.target_family,NEW.target_production_key,NEW.target_revision)
         =('Item','oteryn:item.tibia.i8077',c.enchanting->'debits'->1->'before'->>'definition_revision_ref'))))
  THEN RETURN NULL; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

 body:=pg_get_functiondef('game_item_enchanting_complete(game_item_craft_records)'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing original enchanting completion proof'; END IF;
 arm:=$arm$
  IF r.recipe='oteryn:enchanting.weapon'
   AND r.enchanting->'debits'->1->'before'->>'definition_production_key'='oteryn:item.tibia.i8077'
   AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules s
    WHERE s.item_instance_id=(r.outputs->0->>'item')::uuid AND s.source_transaction_id=r.transaction_id
      AND s.created_xact_id=r.created_xact_id)
  THEN RAISE EXCEPTION 'enchanted rainbow shield requires its original deadline' USING ERRCODE='23514'; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

-- Preserve the original mint clock, including overdue shields. This entry
-- point belongs to the migration owner and is also exercised by the upgrade test.
CREATE FUNCTION game_rainbow_shield_backfill() RETURNS VOID LANGUAGE sql AS $$
 INSERT INTO game_spell_item_temporal_schedules (
  item_instance_id,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,
  definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest)
 SELECT i.item_instance_id,c.transaction_id,1,d.duration,c.occurred_at+d.duration,
  i.definition_family,i.definition_production_key,i.definition_revision_ref,
  'Item','oteryn:item.tibia.i8077',c.enchanting->'debits'->1->'before'->>'definition_revision_ref',
  decode(c.enchanting->>'content_digest','hex')
 FROM game_item_craft_records c
 JOIN game_item_instances i ON i.minted_transaction_id=c.transaction_id
 CROSS JOIN LATERAL (SELECT game_rainbow_shield_duration(i.definition_production_key,i.definition_revision_ref) duration) d
 WHERE c.recipe='oteryn:enchanting.weapon' AND c.outputs->0->>'decay' IS NULL
  AND d.duration IS NOT NULL
  AND c.enchanting->'debits'->1->'before'->>'definition_production_key'='oteryn:item.tibia.i8077'
  AND c.enchanting->>'content_digest' ~ '^[0-9a-f]{64}$'
  AND i.item_instance_id=(c.outputs->0->>'item')::uuid AND i.world_id=c.world_id
  AND (i.definition_production_key,i.definition_revision_ref)=(c.outputs->0->>'key',c.outputs->0->>'revision')
  AND i.definition_family='Item' AND i.lifecycle=1 AND i.quantity=1
 ON CONFLICT (item_instance_id) DO NOTHING
$$;
REVOKE ALL ON FUNCTION game_rainbow_shield_backfill() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

-- Only the proven craft's own nonstackable shield moves with its deadline.
CREATE FUNCTION game_rainbow_shield_moves(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE STRICT AS $$
 SELECT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p
  JOIN game_item_craft_records c ON c.transaction_id=p.source_transaction_id
  WHERE p.item_instance_id=p_item AND p.target_production_key='oteryn:item.tibia.i8077'
   AND c.recipe='oteryn:enchanting.weapon' AND (c.outputs->0->>'item')::uuid=p_item
   AND c.enchanting->'debits'->1->'before'->>'definition_production_key'=p.target_production_key)
$$;

DO $migration$
DECLARE patch RECORD; body TEXT; original TEXT; ground_anchor TEXT;
BEGIN
 FOR patch IN SELECT * FROM (VALUES
  ('game_item_loop_line_admit()', 'NEW.item_instance_id', ' = '),
  ('game_house_container_line_admit(game_item_loop_move_lines)', 'l.item_instance_id', '='),
  ('game_market_write(uuid,uuid,uuid,numeric,numeric,numeric,uuid,smallint,uuid,text,text,smallint,integer,bigint,boolean,integer,bytea,bytea)',
   'item.item_instance_id', '=')
 ) p(signature, operand, eq)
 LOOP
  original:='EXISTS (SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id'||patch.eq||patch.operand||')';
  IF patch.eq='=' THEN original:=replace(original,'EXISTS (','EXISTS('); END IF;
  body:=pg_get_functiondef(patch.signature::regprocedure);
  IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
   RAISE EXCEPTION 'missing exact shield movement guard in %',patch.signature;
  END IF;
  EXECUTE replace(body,original,'('||original||' AND NOT game_rainbow_shield_moves('||patch.operand||'))');
 END LOOP;

 -- A resumed Ground owner uses the same assignment and exact stack proof
 -- already used for other movable action-minted deadlines.
 body:=pg_get_functiondef('game_event_item_timer_ground_current(uuid,uuid,uuid,numeric,bigint)'::regprocedure);
 ground_anchor:='AND (EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules';
 IF strpos(body,ground_anchor)=0 THEN RAISE EXCEPTION 'missing Ground timer movement proof'; END IF;
 EXECUTE replace(body,ground_anchor,
  'AND (game_rainbow_shield_moves(p_item) OR EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules');
END $migration$;

-- The expiry receipt and audit retain the original schedule. The reverted
-- Item has no active deadline. All other schedules are still immutable.
ALTER TABLE game_spell_item_temporal_receipts
 DROP CONSTRAINT game_spell_item_temporal_receipts_item_instance_id_fkey,
 ADD FOREIGN KEY(item_instance_id) REFERENCES game_item_instances(item_instance_id);

CREATE FUNCTION game_spell_item_temporal_schedule_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' AND game_rainbow_shield_moves(OLD.item_instance_id)
   AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r
     JOIN game_spell_item_temporal_audit a USING(transaction_id,event_id)
     JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
     WHERE r.item_instance_id=OLD.item_instance_id AND r.created_xact_id=pg_current_xact_id()
      AND a.created_xact_id=r.created_xact_id AND i.last_transaction_id=r.transaction_id
      AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=r.state_revision_before+1
      AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
       =(OLD.target_family,OLD.target_production_key,OLD.target_revision)
      AND convert_from(a.envelope,'UTF8')::jsonb=jsonb_build_object('receipt',to_jsonb(r),'schedule',to_jsonb(OLD)))
 THEN RETURN OLD; END IF;
 RAISE EXCEPTION 'Item temporal schedule is immutable until proven shield expiry' USING ERRCODE='23514';
END $$;

DROP TRIGGER game_spell_item_temporal_schedules_immutable ON game_spell_item_temporal_schedules;
CREATE TRIGGER game_spell_item_temporal_schedules_immutable
 BEFORE UPDATE OR DELETE ON game_spell_item_temporal_schedules FOR EACH ROW
 EXECUTE FUNCTION game_spell_item_temporal_schedule_guard();

CREATE FUNCTION game_rainbow_shield_clear_deadline() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF game_rainbow_shield_moves(NEW.item_instance_id) THEN
  DELETE FROM game_spell_item_temporal_schedules WHERE item_instance_id=NEW.item_instance_id;
 END IF;
 RETURN NULL;
END $$;

-- PostgreSQL fires same-event constraint triggers in name order. The normal
-- receipt proof runs before this cleanup; its immutable audit retains the clock.
CREATE CONSTRAINT TRIGGER z_rainbow_shield_clear_deadline
 AFTER INSERT ON game_spell_item_temporal_receipts DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_rainbow_shield_clear_deadline();

DO $migration$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_rainbow_shield_duration(text,text)','game_rainbow_shield_backfill()',
  'game_rainbow_shield_moves(uuid)','game_spell_item_temporal_schedule_guard()','game_rainbow_shield_clear_deadline()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
 END LOOP;
END $migration$;
REVOKE ALL ON FUNCTION game_rainbow_shield_duration(text,text),game_rainbow_shield_moves(uuid),
 game_spell_item_temporal_schedule_guard(),game_rainbow_shield_clear_deadline() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_rainbow_shield_duration(text,text),game_rainbow_shield_moves(uuid) TO oteryn_game_runtime;

-- Use the existing unattended Item pass for shields in depot, Inbox, House
-- or offline custody. Escrow is intentionally outside the custody selector:
-- an open offer keeps its exact quoted item until settlement or withdrawal.
DO $migration$
DECLARE patch RECORD; body TEXT; anchor TEXT; successor TEXT; first_at INT; last_at INT;
BEGIN
 FOR patch IN SELECT * FROM (VALUES
  ('game_temple_scroll_capture(game_item_instances,uuid)', 'o.item_instance_id'),
  ('game_temple_scroll_receipt_proven(game_spell_item_temporal_receipts)', 'r.item_instance_id')
 ) p(signature, item)
 LOOP
  body:=pg_get_functiondef(patch.signature::regprocedure);
  -- 3131 adds the ice sample to the scroll's original terminal-only predicate.
  anchor:=$old$p.definition_production_key NOT IN ('oteryn:item.tibia.i25718','oteryn:item.tibia.i4837') OR p.target_family IS NOT NULL$old$;
  IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing unattended Item proof in %',patch.signature; END IF;
  body:=replace(body,anchor,'(('||anchor||') AND NOT game_rainbow_shield_moves('||patch.item||'))');
  IF patch.item='r.item_instance_id' THEN
   anchor:='OR i.lifecycle<>2 OR i.quantity<>0';
   IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing unattended Item successor'; END IF;
   successor:=$proof$
   OR (p.target_family IS NULL AND (i.lifecycle<>2 OR i.quantity<>0))
   OR (p.target_family IS NOT NULL AND (i.lifecycle<>1 OR i.quantity<>r.quantity_before
     OR (i.definition_family,i.definition_production_key,i.definition_revision_ref)
       IS DISTINCT FROM (p.target_family,p.target_production_key,p.target_revision)
     OR NOT EXISTS(SELECT 1 FROM public.game_temple_scroll_custody(i.item_instance_id) k
       WHERE k.kind=r.custody_kind AND k.character_id IS NOT DISTINCT FROM r.character_id
        AND k.parent IS NOT DISTINCT FROM r.source_parent
        AND k.ordinal IS NOT DISTINCT FROM r.source_ordinal
        AND k.slot IS NOT DISTINCT FROM r.source_slot
        AND k.equipment_revision IS NOT DISTINCT FROM r.equipment_revision_before+1
        AND (k.channel_id IS NULL OR k.channel_id=r.channel_id))))
$proof$;
   body:=replace(body,anchor,successor);
   anchor:='OR EXISTS(SELECT 1 FROM public.game_item_ground_locations WHERE item_instance_id=r.item_instance_id)';
   IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing unattended Item retirement custody proof'; END IF;
   body:=replace(body,anchor,'OR (p.target_family IS NULL AND (EXISTS'||substr(anchor,length('OR EXISTS')+1));
   anchor:='OR EXISTS(SELECT 1 FROM public.game_bazaar_item_locations WHERE item_instance_id=r.item_instance_id) THEN';
   IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing final unattended Item custody proof'; END IF;
   body:=replace(body,anchor,replace(anchor,') THEN','))) THEN'));
  END IF;
  EXECUTE body;
 END LOOP;

 body:=pg_get_functiondef('game_temple_scroll_expire_due(uuid,uuid,numeric,integer)'::regprocedure);
 anchor:=$old$p.definition_production_key IN ('oteryn:item.tibia.i25718','oteryn:item.tibia.i4837') AND p.target_family IS NULL$old$;
 IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing unattended Item timer selection'; END IF;
 body:=replace(body,anchor,'(('||anchor||') OR game_rainbow_shield_moves(p.item_instance_id))');
 anchor:='k.slot,k.equipment_revision';
 IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing unattended Item custody selection'; END IF;
 body:=replace(body,anchor,anchor||',p.target_family,p.target_production_key,p.target_revision');
 first_at:=strpos(body,'   UPDATE public.game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=tx');
 last_at:=strpos(body,'   SELECT convert_to(jsonb_build_object(');
 IF first_at=0 OR last_at<=first_at THEN RAISE EXCEPTION 'missing unattended Item retirement writer'; END IF;
 EXECUTE overlay(body placing $writer$
   IF s.target_family IS NOT NULL THEN
    UPDATE public.game_item_instances SET definition_family=s.target_family,
     definition_production_key=s.target_production_key,definition_revision_ref=s.target_revision,last_transaction_id=tx
     WHERE item_instance_id=s.item_instance_id AND state_revision=s.state_revision AND lifecycle=1;
    IF NOT FOUND THEN RAISE EXCEPTION 'shield timer compare-and-change' USING ERRCODE='23514'; END IF;
    IF s.kind=3 THEN
     UPDATE public.game_character_equipment_state SET revision=revision+1,last_transaction_id=tx
      WHERE character_id=s.character_id AND revision=s.equipment_revision;
     IF NOT FOUND THEN RAISE EXCEPTION 'shield timer equipment epoch' USING ERRCODE='23514'; END IF;
    END IF;
   ELSE
$writer$||substr(body,first_at,last_at-first_at)||E'   END IF;\n' FROM first_at FOR last_at-first_at);
END $migration$;

-- Queue the historical schedule proofs after installing all table triggers.
SELECT game_rainbow_shield_backfill();
