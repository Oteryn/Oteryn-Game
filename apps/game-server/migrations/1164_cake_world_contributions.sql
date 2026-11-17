-- W05d stage-one goals: Crystal a_piece_of_cake_config.lua:97-104,374-383;
-- extraction source movements_extraction_spot.lua:26-49. This cohort does not
-- admit bridge Item48227 or invent stage two while its identity is unavailable.
CREATE TABLE game_cake_world_progress (
 world_id UUID NOT NULL,
 event_year INTEGER NOT NULL CHECK(event_year BETWEEN 2000 AND 9999),
 substances SMALLINT NOT NULL DEFAULT 0 CHECK(substances BETWEEN 0 AND 200),
 golems SMALLINT NOT NULL DEFAULT 0 CHECK(golems BETWEEN 0 AND 200),
 PRIMARY KEY(world_id,event_year)
);

CREATE TABLE game_cake_lure_receipts (
 firing_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 wave SMALLINT NOT NULL,
 entry SMALLINT NOT NULL,
 creature_index SMALLINT NOT NULL,
 world_id UUID NOT NULL,
 event_year INTEGER NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,
 x INTEGER NOT NULL,
 y INTEGER NOT NULL,
 z INTEGER NOT NULL CHECK(z=7),
 PRIMARY KEY(firing_id,channel_id,wave,entry,creature_index),
 FOREIGN KEY(firing_id,channel_id,wave,entry,creature_index)
  REFERENCES game_world_raid_creatures(firing_id,channel_id,wave,entry,creature_index),
 FOREIGN KEY(world_id,event_year) REFERENCES game_cake_world_progress(world_id,event_year),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_epochs(world_id,reset_epoch),
 -- Crystal startup/tables/tile.lua:1111-1153 contains these Item8886 tiles.
 CHECK((x=32346 AND y IN(31822,31823)) OR (x=32350 AND y IN(32275,32276))
    OR (x=33251 AND y IN(31802,31803)) OR (x=33191 AND y IN(32397,32398))
    OR (x=32377 AND y IN(32801,32802)))
);

CREATE FUNCTION game_cake_lure_immutable() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 RAISE EXCEPTION 'Cake lure receipts are immutable' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_cake_lure_no_rewrite BEFORE UPDATE OR DELETE
 ON game_cake_lure_receipts FOR EACH ROW EXECUTE FUNCTION game_cake_lure_immutable();

CREATE FUNCTION game_cake_lure_admit() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_world_raid_firings f
  JOIN game_world_raid_creatures c USING(firing_id)
  JOIN game_world_raid_creature_placements p USING(firing_id,channel_id,wave,entry,creature_index)
  JOIN game_world_reset_active a ON a.world_id=f.world_id AND a.reset_epoch=NEW.reset_epoch
  JOIN game_world_reset_epochs r ON r.world_id=a.world_id AND r.reset_epoch=a.reset_epoch
  JOIN game_world_event_epochs e ON e.world_id=a.world_id AND e.reset_epoch=a.reset_epoch
  JOIN game_cake_world_progress q ON q.world_id=f.world_id AND q.event_year=NEW.event_year
  WHERE f.firing_id=NEW.firing_id AND f.world_id=NEW.world_id
   AND f.raid_key='oteryn:raid.piece_of_cake' AND c.channel_id=NEW.channel_id
   AND c.wave=NEW.wave AND c.entry=NEW.entry AND c.creature_index=NEW.creature_index
   AND c.creature_key='oteryn:creature.cake_golem' AND c.state=2 AND p.placed
   AND extract(year FROM r.activated_at AT TIME ZONE 'Europe/Berlin')=NEW.event_year
   AND e.seasonal_calendar_revision=1 AND 'piece_of_cake'=ANY(e.seasonal_event_keys)
   AND q.golems<200)
 THEN RAISE EXCEPTION 'Cake extraction requires its placed source raid and current annual occurrence'
  USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_cake_lure_admission BEFORE INSERT ON game_cake_lure_receipts
 FOR EACH ROW EXECUTE FUNCTION game_cake_lure_admit();

CREATE FUNCTION game_cake_progress_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  IF NEW.substances<>0 OR NEW.golems<>0 THEN
   RAISE EXCEPTION 'Cake occurrence begins with zero contributions' USING ERRCODE='23514';
  END IF;
 ELSIF (NEW.world_id,NEW.event_year) IS DISTINCT FROM (OLD.world_id,OLD.event_year)
  OR NEW.substances<>OLD.substances OR NEW.golems NOT BETWEEN OLD.golems AND OLD.golems+1
  OR NEW.golems<>(SELECT count(*) FROM game_cake_lure_receipts r
   WHERE r.world_id=NEW.world_id AND r.event_year=NEW.event_year)
 THEN RAISE EXCEPTION 'Cake progress requires exact immutable contributions' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_cake_progress_admission BEFORE INSERT OR UPDATE ON game_cake_world_progress
 FOR EACH ROW EXECUTE FUNCTION game_cake_progress_guard();

CREATE TABLE game_cake_donation_receipts (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_consume_transform_records,
 world_id UUID NOT NULL,
 event_year INTEGER NOT NULL,
 before_count SMALLINT NOT NULL CHECK(before_count BETWEEN 0 AND 199),
 after_count SMALLINT NOT NULL CHECK(after_count BETWEEN 1 AND 200 AND after_count>before_count),
 FOREIGN KEY(world_id,event_year) REFERENCES game_cake_world_progress(world_id,event_year)
);
CREATE TRIGGER game_cake_donation_no_rewrite BEFORE UPDATE OR DELETE ON game_cake_donation_receipts
 FOR EACH ROW EXECUTE FUNCTION game_cake_lure_immutable();

CREATE FUNCTION game_cake_donation_shape(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(r.recipe='oteryn:cake.baker-substances' AND r.consume_source
  AND r.source_kind IN(1,2,4) AND r.source_count BETWEEN 1 AND 100
  AND r.before_item->>'definition_family'='Item'
  AND r.before_item->>'definition_production_key'='oteryn:item.tibia.i12143'
  AND r.before_item->>'definition_revision_ref'='definition-r1'
  AND r.outputs='[]'::jsonb AND r.secondary_item_instance_id=r.source_item_instance_id
  AND r.before_secondary=r.before_item AND r.effects->>'kind'='cake_substances'
  AND r.effects->>'npc_key' IN('oteryn:npc.benny_the_baker','oteryn:npc.biff_the_baker',
   'oteryn:npc.brian_the_baker','oteryn:npc.balduin_the_baker','oteryn:npc.billy_the_baker')
  AND jsonb_typeof(r.extra_inputs)='array' AND jsonb_array_length(r.extra_inputs)<=199
  AND octet_length(r.extra_inputs::text)<=1048576
  AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(r.extra_inputs) e
   WHERE (e->>'count')::integer NOT BETWEEN 1 AND 100 OR (e->>'source_kind')::integer NOT IN(1,2,4)
    OR e->'before'->>'definition_production_key' IS DISTINCT FROM 'oteryn:item.tibia.i12143'
    OR e->'before'->>'definition_revision_ref' IS DISTINCT FROM 'definition-r1')
  AND r.source_count+(SELECT coalesce(sum((e->>'count')::integer),0) FROM jsonb_array_elements(r.extra_inputs) e)
   =(r.effects->>'accepted')::integer
  AND (r.effects->>'accepted')::integer BETWEEN 1 AND 200,false)
$$;

-- Existing limits remain byte-for-byte branches for every other recipe.
DO $migration$
DECLARE n TEXT; prior TEXT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_action_extra_input_bound','game_item_action_count_shape',
  'game_item_consume_transform_parent_capacity_check'] LOOP
  SELECT pg_get_expr(conbin,conrelid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_item_consume_transform_records'::regclass AND conname=n;
  EXECUTE format('ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT %I',n);
  EXECUTE format('ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT %I CHECK ((%s) OR game_cake_donation_shape(game_item_consume_transform_records))',n,prior);
 END LOOP;
END $migration$;

CREATE FUNCTION game_cake_extra_custody(r game_item_consume_transform_records,e JSONB)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT CASE WHEN e->>'source_kind'='2' THEN EXISTS(
  SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=(e->>'item')::uuid
   AND s.character_id=r.character_id AND s.world_id=r.world_id AND s.slot=(e->>'ordinal')::smallint
   AND (e->>'parent')::uuid=r.character_id)
 ELSE game_item_action_extra_custody(r,e) END
$$;

CREATE FUNCTION game_cake_donation_extra_admit(r game_item_consume_transform_records) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE e JSONB; i game_item_instances%ROWTYPE;
BEGIN
 IF NOT game_cake_donation_shape(r) OR
  (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(r.extra_inputs))<>jsonb_array_length(r.extra_inputs)
 THEN RAISE EXCEPTION 'Cake donation requires exact bounded inputs' USING ERRCODE='23514'; END IF;
 FOR e IN SELECT value FROM jsonb_array_elements(r.extra_inputs) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(e->>'item')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR i.item_instance_id IN(r.source_item_instance_id,r.secondary_item_instance_id)
   OR to_jsonb(i) IS DISTINCT FROM e->'before' OR i.lifecycle<>1 OR i.quantity<(e->>'count')::integer
   OR i.world_id<>r.world_id OR i.definition_family<>'Item'
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR NOT game_cake_extra_custody(r,e)
  THEN RAISE EXCEPTION 'Cake donation lacks plain carried predecessor' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

CREATE FUNCTION game_cake_donation_extra_complete(r game_item_consume_transform_records) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE e JSONB; i game_item_instances%ROWTYPE;
BEGIN
 FOR e IN SELECT value FROM jsonb_array_elements(r.extra_inputs) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(e->>'item')::uuid;
  IF i.item_instance_id IS NULL OR i.quantity<>(e->'before'->>'quantity')::bigint-(e->>'count')::integer
   OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END) OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
   OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
      ((e->'before')-ARRAY['quantity','lifecycle','last_transaction_id'])
   OR (i.quantity=0 AND (EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
                     OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)
                     OR EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=i.item_instance_id)))
   OR (i.quantity>0 AND NOT game_cake_extra_custody(r,e))
  THEN RAISE EXCEPTION 'Cake donation lacks exact decrement and custody' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_action_extra_admit','game_item_action_extra_complete'] LOOP
  body:=pg_get_functiondef((n||'(game_item_consume_transform_records)')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing Cake additional-input extension boundary'; END IF;
  arm:='IF p_record.recipe=''oteryn:cake.baker-substances'' THEN PERFORM game_cake_donation_extra_'
   ||CASE WHEN n='game_item_action_extra_admit' THEN 'admit' ELSE 'complete' END||'(p_record); RETURN; END IF;';
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
 body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Cake instance guard extension boundary'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r
  LEFT JOIN LATERAL jsonb_array_elements(r.extra_inputs) e ON (e->>'item')::uuid=OLD.item_instance_id
  WHERE r.recipe='oteryn:cake.baker-substances' AND r.created_xact_id=pg_current_xact_id()
   AND r.transaction_id=NEW.last_transaction_id AND game_cake_donation_shape(r)
   AND ((r.source_item_instance_id=OLD.item_instance_id AND r.before_item=to_jsonb(OLD)
         AND NEW.quantity=OLD.quantity-r.source_count)
     OR (e->'before'=to_jsonb(OLD) AND NEW.quantity=OLD.quantity-(e->>'count')::integer))) THEN
  IF NEW.lifecycle<>(CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END)
   OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
      (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
  THEN RAISE EXCEPTION 'Cake donation changes only its exact carried quantity' USING ERRCODE='23514'; END IF;
  RETURN NEW;
 END IF;
$arm$ from begin_at for 5);
END $migration$;

CREATE FUNCTION game_cake_donation_record() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE q game_cake_world_progress%ROWTYPE; available BIGINT; accepted INTEGER;
BEGIN
 IF NEW.recipe<>'oteryn:cake.baker-substances' THEN RETURN NEW; END IF;
 IF NOT game_cake_donation_shape(NEW) THEN
  RAISE EXCEPTION 'Cake donation requires its exact source recipe' USING ERRCODE='23514'; END IF;
 SELECT * INTO q FROM game_cake_world_progress
  WHERE world_id=NEW.world_id AND event_year=(NEW.effects->>'year')::integer FOR UPDATE;
 IF q.world_id IS NULL OR q.substances>=200 OR NOT EXISTS(
  SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs r USING(world_id,reset_epoch)
  JOIN game_world_event_epochs e USING(world_id,reset_epoch)
  WHERE a.world_id=NEW.world_id AND a.reset_epoch=(NEW.effects->>'reset_epoch')::numeric
   AND extract(year FROM r.activated_at AT TIME ZONE 'Europe/Berlin')=q.event_year
   AND e.seasonal_calendar_revision=1 AND 'piece_of_cake'=ANY(e.seasonal_event_keys))
 THEN RAISE EXCEPTION 'Cake donation requires the current annual stage one' USING ERRCODE='23514'; END IF;
 IF ((NEW.source_kind=2 OR EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.extra_inputs) e WHERE e->>'source_kind'='2'))
    IS DISTINCT FROM (coalesce(NEW.effects->'equipment_before','null'::jsonb)<>'null'::jsonb))
  OR (coalesce(NEW.effects->'equipment_before','null'::jsonb)<>'null'::jsonb AND
   NEW.effects->'equipment_before' IS DISTINCT FROM (SELECT to_jsonb(s) FROM game_character_equipment_state s WHERE s.character_id=NEW.character_id))
 THEN RAISE EXCEPTION 'Cake donation requires its exact current equipment predecessor' USING ERRCODE='23514'; END IF;
 -- The source NPC removes min(all carried units, remaining shared goal), not
 -- merely the clicked stack. Count the same current custody graph before burns.
 SELECT coalesce(sum(i.quantity),0) INTO available FROM game_item_instances i
 LEFT JOIN game_item_container_entries c ON c.item_instance_id=i.item_instance_id AND c.character_id=NEW.character_id
 LEFT JOIN game_item_container_slots b ON b.character_id=c.character_id AND b.item_instance_id=c.parent_item_instance_id
 LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id
 LEFT JOIN LATERAL game_item_loop_owner(n.parent_item_instance_id) owner ON n.item_instance_id IS NOT NULL
 LEFT JOIN game_character_equipment_slots h ON h.item_instance_id=i.item_instance_id AND h.character_id=NEW.character_id AND h.world_id=i.world_id
 WHERE i.world_id=NEW.world_id AND i.lifecycle=1 AND i.definition_family='Item'
  AND i.definition_production_key='oteryn:item.tibia.i12143' AND i.definition_revision_ref='definition-r1'
  AND ((c.world_id=i.world_id AND b.world_id=i.world_id)
   OR (n.world_id=i.world_id AND owner.owner_character=NEW.character_id AND owner.depth BETWEEN 0 AND 7) OR h.item_instance_id IS NOT NULL);
 accepted:=(NEW.effects->>'accepted')::integer;
 IF NEW.created_xact_id<>pg_current_xact_id() OR accepted<>least(available,200-q.substances)
  OR (NEW.effects->>'before')::integer<>q.substances
  OR (NEW.effects->>'after')::integer<>q.substances+accepted
 THEN RAISE EXCEPTION 'Cake donation must consume the exact capped carried total' USING ERRCODE='23514'; END IF;
 INSERT INTO game_cake_donation_receipts(transaction_id,world_id,event_year,before_count,after_count)
 VALUES(NEW.transaction_id,NEW.world_id,q.event_year,q.substances,q.substances+accepted);
 UPDATE game_cake_world_progress SET substances=substances+accepted
 WHERE world_id=q.world_id AND event_year=q.event_year;
 RETURN NEW;
END $$;
CREATE TRIGGER game_cake_donation_record AFTER INSERT ON game_item_consume_transform_records
 FOR EACH ROW EXECUTE FUNCTION game_cake_donation_record();

CREATE OR REPLACE FUNCTION game_cake_progress_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  IF NEW.substances<>0 OR NEW.golems<>0 THEN
   RAISE EXCEPTION 'Cake occurrence begins with zero contributions' USING ERRCODE='23514';
  END IF;
 ELSIF (NEW.world_id,NEW.event_year) IS DISTINCT FROM (OLD.world_id,OLD.event_year)
  OR NEW.substances<OLD.substances OR NEW.golems NOT BETWEEN OLD.golems AND OLD.golems+1
  OR NEW.substances<>(SELECT coalesce(sum(after_count-before_count),0) FROM game_cake_donation_receipts r
   WHERE r.world_id=NEW.world_id AND r.event_year=NEW.event_year)
  OR NEW.golems<>(SELECT count(*) FROM game_cake_lure_receipts r
   WHERE r.world_id=NEW.world_id AND r.event_year=NEW.event_year)
 THEN RAISE EXCEPTION 'Cake progress requires exact immutable contributions' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_cake_donation_complete() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.recipe='oteryn:cake.baker-substances' AND NOT EXISTS(
  SELECT 1 FROM game_cake_donation_receipts r WHERE r.transaction_id=NEW.transaction_id
   AND r.world_id=NEW.world_id AND r.event_year=(NEW.effects->>'year')::integer
   AND r.before_count=(NEW.effects->>'before')::integer AND r.after_count=(NEW.effects->>'after')::integer)
 THEN RAISE EXCEPTION 'Cake item receipt requires its atomic world contribution' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cake_donation_complete AFTER INSERT ON game_item_consume_transform_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cake_donation_complete();

DO $privileges$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_cake_lure_immutable()','game_cake_lure_admit()',
  'game_cake_progress_guard()','game_cake_extra_custody(game_item_consume_transform_records,jsonb)','game_cake_donation_shape(game_item_consume_transform_records)',
  'game_cake_donation_extra_admit(game_item_consume_transform_records)',
  'game_cake_donation_extra_complete(game_item_consume_transform_records)',
  'game_cake_donation_record()','game_cake_donation_complete()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
 END LOOP;
END $privileges$;
GRANT SELECT,INSERT,UPDATE ON game_cake_world_progress TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_cake_lure_receipts,game_cake_donation_receipts TO oteryn_game_runtime;

CREATE FUNCTION game_cake_donation_receipt_admit() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_consume_transform_records r
  WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id()
   AND r.world_id=NEW.world_id AND game_cake_donation_shape(r)
   AND (r.effects->>'year')::integer=NEW.event_year
   AND (r.effects->>'before')::integer=NEW.before_count
   AND (r.effects->>'after')::integer=NEW.after_count)
 THEN RAISE EXCEPTION 'Cake donation contribution requires its same-transaction item receipt' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_cake_donation_receipt_admission BEFORE INSERT ON game_cake_donation_receipts
 FOR EACH ROW EXECUTE FUNCTION game_cake_donation_receipt_admit();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_cake_donation_receipt_admit() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_cake_donation_receipt_admit() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_cake_donation_receipt_admit() TO oteryn_game_runtime;

-- A carried hand stack is part of Player:getItemCount/removeItem as well. Only
-- this source recipe can consume existing equipment custody kind2; the party
-- hat's unchanged, effect-only head contract remains the original branch.
DO $migration$
DECLARE body TEXT; old TEXT; replacement TEXT; prior TEXT;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT prior FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_source_custody';
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_source_custody;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_source_custody CHECK (('
  ||prior||') OR game_cake_donation_shape(game_item_consume_transform_records))';
 body:=pg_get_functiondef('game_item_action_custody_current(game_item_consume_transform_records)'::regprocedure);
 old:='ELSE FALSE END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'missing Cake equipment custody extension boundary'; END IF;
 EXECUTE replace(body,old,$arm$WHEN 2 THEN game_cake_donation_shape(r) AND EXISTS(
  SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=r.source_item_instance_id
   AND s.character_id=r.character_id AND s.world_id=r.world_id AND s.slot=r.source_ordinal
   AND r.parent_item_instance_id IS NULL AND to_jsonb(s)=r.source_location)
 ELSE FALSE END$arm$);
 body:=pg_get_functiondef('game_item_consume_transform_admit()'::regprocedure);
 old:='IF NEW.source_kind IN(3,4) THEN';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'missing Cake equipped source admission boundary'; END IF;
 EXECUTE replace(body,old,'IF NEW.source_kind IN(3,4) OR (NEW.source_kind=2 AND NEW.recipe=''oteryn:cake.baker-substances'') THEN');
 body:=pg_get_functiondef('game_item_consume_transform_complete(uuid)'::regprocedure);
 old:='IF r.source_kind IN(3,4) THEN';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'missing Cake equipped source completion boundary'; END IF;
 EXECUTE replace(body,old,'IF r.source_kind IN(3,4) OR (r.source_kind=2 AND r.recipe=''oteryn:cake.baker-substances'') THEN');
END $migration$;

CREATE FUNCTION game_cake_equipment_complete(r game_item_consume_transform_records) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE e JSONB; s game_character_equipment_state%ROWTYPE;
BEGIN
 IF r.source_kind=2 AND (r.before_item->>'quantity')::bigint=r.source_count
  AND EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=r.source_item_instance_id)
 THEN RAISE EXCEPTION 'Cake donation must retire its consumed equipment slot' USING ERRCODE='23514'; END IF;
 IF coalesce(r.effects->'equipment_before','null'::jsonb)='null'::jsonb THEN RETURN; END IF;
 SELECT * INTO s FROM game_character_equipment_state WHERE character_id=r.character_id;
 IF s.character_id IS NULL OR s.revision<>(r.effects->'equipment_before'->>'revision')::numeric+1
  OR s.last_transaction_id IS DISTINCT FROM r.transaction_id
  OR (to_jsonb(s)-ARRAY['revision','last_transaction_id']) IS DISTINCT FROM
     ((r.effects->'equipment_before')-ARRAY['revision','last_transaction_id'])
 THEN RAISE EXCEPTION 'Cake donation requires its atomic equipment revision' USING ERRCODE='23514'; END IF;
END $$;
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_cake_donation_complete()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF NEW.recipe='oteryn:cake.baker-substances' THEN PERFORM game_cake_equipment_complete(NEW); END IF;
$arm$ from begin_at for 5);
 FOREACH n IN ARRAY ARRAY['game_equipment_slot_proven','game_equipment_state_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing Cake equipment proof extension: %',n; END IF;
  IF n='game_equipment_slot_proven' THEN arm:=$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r JOIN game_item_instances i ON i.item_instance_id=OLD.item_instance_id
    WHERE r.recipe='oteryn:cake.baker-substances' AND r.created_xact_id=pg_current_xact_id()
     AND r.character_id=OLD.character_id AND r.world_id=OLD.world_id AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id
     AND ((r.source_kind=2 AND r.source_item_instance_id=OLD.item_instance_id AND r.source_location=to_jsonb(OLD))
      OR EXISTS(SELECT 1 FROM jsonb_array_elements(r.extra_inputs) e WHERE e->>'source_kind'='2'
        AND (e->>'item')::uuid=OLD.item_instance_id AND (e->>'ordinal')::smallint=OLD.slot))) THEN
    RETURN NULL;
   END IF;
$arm$;
  ELSE arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r WHERE r.recipe='oteryn:cake.baker-substances'
    AND r.created_xact_id=pg_current_xact_id() AND r.transaction_id=NEW.last_transaction_id
    AND r.character_id=NEW.character_id AND r.effects->'equipment_before'=to_jsonb(OLD)) THEN
    PERFORM game_cake_equipment_complete(r) FROM game_item_consume_transform_records r WHERE r.transaction_id=NEW.last_transaction_id;
    RETURN NULL;
   END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_cake_equipment_complete(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_cake_equipment_complete(game_item_consume_transform_records) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_cake_equipment_complete(game_item_consume_transform_records) TO oteryn_game_runtime;

-- Crystal biff_the_baker.lua:69-100 Player:getItemCount/removeItem includes
-- carried hand stacks. An equipment source has no parent Container, so its
-- actual parent capacity is0; all existing recipes retain1..500 unchanged.
DO $migration$
DECLARE prior TEXT;body TEXT;old TEXT;replacement TEXT;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT prior FROM pg_constraint
  WHERE conrelid='game_item_consume_transform_records'::regclass
   AND conname='game_item_consume_transform_records_parent_capacity_check';
 ALTER TABLE game_item_consume_transform_records
  DROP CONSTRAINT game_item_consume_transform_records_parent_capacity_check;
 EXECUTE format('ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_consume_transform_records_parent_capacity_check CHECK ((%s) OR (recipe=''oteryn:cake.baker-substances'' AND source_kind=2 AND parent_item_instance_id IS NULL AND parent_capacity=0 AND game_cake_donation_shape(game_item_consume_transform_records)))',prior);

 -- Preserve the complete existing direct-carried source custody/audit proof.
 -- Only the new exact source recipe gains its already-validated source_count;
 -- all legacy branches keep the original one-unit/dispenser semantics.
 body:=pg_get_functiondef('game_item_consume_transform_complete(uuid)'::regprocedure);
 old:='CASE WHEN r.recipe=''oteryn:dispenser.fill'' THEN r.source_count ELSE 1 END';
 replacement:='CASE WHEN r.recipe IN(''oteryn:dispenser.fill'',''oteryn:cake.baker-substances'') THEN r.source_count ELSE 1 END';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
  RAISE EXCEPTION 'missing exact Cake direct-source count completion boundary';
 END IF;
 EXECUTE replace(body,old,replacement);
END $migration$;

-- An equipment root has no parent Container. Do not permit a fabricated
-- positive capacity merely because that value satisfies the legacy scalar bound.
ALTER TABLE game_item_consume_transform_records
 ADD CONSTRAINT game_cake_equipment_parent_truth CHECK(
  recipe<>'oteryn:cake.baker-substances' OR source_kind<>2 OR
  (parent_item_instance_id IS NULL AND parent_capacity=0));

-- Player:getItemCount/removeItem (Crystal biff_the_baker.lua:69-100) may
-- consume200 one-unit carried inputs. Every deferred Item mutation still
-- revalidates the complete final set, using one relational proof instead of
-- issuing an individual PL/pgSQL SELECT for each input on every callback.
-- There is no memoization, timeout increase or omitted custody/before-image check.
CREATE OR REPLACE FUNCTION game_cake_donation_extra_complete(r game_item_consume_transform_records)
 RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
 IF NOT game_cake_donation_shape(r) OR EXISTS(
  SELECT 1 FROM jsonb_array_elements(r.extra_inputs) e
  LEFT JOIN game_item_instances i ON i.item_instance_id=(e->>'item')::uuid
  WHERE i.item_instance_id IS NULL
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
 ) THEN RAISE EXCEPTION 'Cake donation lacks exact decrement and custody' USING ERRCODE='23514'; END IF;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_cake_donation_extra_complete(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

-- Crystal biff_the_baker.lua:69-100 removes the accepted substance amount,
-- including an entire nested stack. The existing additional-input removal
-- proof admits single units; extend only this exact recipe to its complete
-- before quantity, preserving every existing custody/transaction predicate.
DO $migration$
DECLARE body TEXT;old TEXT;replacement TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 old:='WHERE (e->>''source_kind'')::integer=4 AND (e->>''count'')::integer=1';
 replacement:='WHERE (e->>''source_kind'')::integer=4 AND
  ((e->>''count'')::integer=1 OR (r.recipe=''oteryn:cake.baker-substances''
   AND game_cake_donation_shape(r)
   AND (e->>''count'')::bigint=(e->''before''->>''quantity'')::bigint))';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
  RAISE EXCEPTION 'missing exact Cake nested extra stack-retirement boundary';
 END IF;
 EXECUTE replace(body,old,replacement);
END $migration$;
