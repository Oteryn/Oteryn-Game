-- WCONTRIB: custody evidence joins the existing World contribution and Quest owners.
ALTER TABLE game_world_change_tallies
 DROP CONSTRAINT game_world_change_tallies_resource_check,
 ADD CONSTRAINT game_world_change_tallies_resource_check
 CHECK(resource IN ('coal','medicine','votes','corpses'));
ALTER TABLE game_world_change_contributions
 DROP CONSTRAINT game_world_change_contributions_resource_check,
 ADD CONSTRAINT game_world_change_contributions_resource_check
 CHECK(resource IN ('coal','medicine','votes','corpses')),
 ADD COLUMN created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id();

CREATE TABLE game_world_change_debits (
 world_id UUID NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,
 operation_id BYTEA NOT NULL CHECK(octet_length(operation_id)=16),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
 transition_key TEXT NOT NULL
   CHECK(transition_key ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(transition_key)<=128),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 amount BIGINT NOT NULL CHECK(amount BETWEEN 1 AND 4294967295),
 item_instance_id UUID REFERENCES game_item_instances(item_instance_id),
 before_item JSONB,
 before_timed JSONB,
 before_location JSONB,
 location_kind TEXT CHECK(location_kind IN ('container','nested','equipment')),
 transaction_id UUID CHECK(game_character_is_uuid_v7(transaction_id)),
 points_track TEXT CHECK(points_track ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(points_track)<=128),
 before_points BIGINT,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(world_id,reset_epoch,operation_id),
 UNIQUE(character_id,game_session_id,cause_ordinal),
 FOREIGN KEY(world_id,reset_epoch,operation_id)
   REFERENCES game_world_change_contributions(world_id,reset_epoch,operation_id)
   DEFERRABLE INITIALLY DEFERRED,
 CHECK(CASE WHEN item_instance_id IS NOT NULL THEN
   before_item IS NOT NULL AND before_location IS NOT NULL AND location_kind IS NOT NULL
   AND transaction_id IS NOT NULL AND points_track IS NULL AND before_points IS NULL
 ELSE before_item IS NULL AND before_timed IS NULL AND before_location IS NULL AND location_kind IS NULL
   AND transaction_id IS NULL AND points_track IS NOT NULL
   AND before_points IS NOT NULL AND before_points>=amount END)
);
CREATE INDEX game_world_change_debits_character ON game_world_change_debits(character_id);
CREATE INDEX game_world_change_debits_item ON game_world_change_debits(item_instance_id);

CREATE FUNCTION game_world_change_debit_after(r game_world_change_debits, i game_item_instances)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT i.item_instance_id=r.item_instance_id AND i.last_transaction_id=r.transaction_id
   AND i.quantity=(r.before_item->>'quantity')::bigint-r.amount
   AND i.lifecycle=CASE WHEN i.quantity=0 THEN 2 ELSE 1 END
   AND (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id'])=
       (r.before_item-ARRAY['quantity','lifecycle','last_transaction_id'])
$$;

CREATE FUNCTION game_world_change_debit_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; points BIGINT; location JSONB; timed JSONB;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 IF NEW.item_instance_id IS NULL THEN
   SELECT value INTO points FROM game_character_quest_tracks
    WHERE character_id=NEW.character_id AND track_key=NEW.points_track FOR UPDATE;
   IF NOT FOUND OR points IS DISTINCT FROM NEW.before_points OR points<NEW.amount THEN
     RAISE EXCEPTION 'world contribution lacks owned points' USING ERRCODE='23514';
   END IF;
   RETURN NEW;
 END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 CASE NEW.location_kind
   WHEN 'container' THEN
     SELECT to_jsonb(l) INTO location FROM game_item_container_entries l
      WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
   WHEN 'nested' THEN
     SELECT to_jsonb(l) INTO location FROM game_item_nested_entries l
      WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
   WHEN 'equipment' THEN
     SELECT to_jsonb(l) INTO location FROM game_character_equipment_slots l
      WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
   ELSE RAISE EXCEPTION 'unsupported world contribution custody' USING ERRCODE='23514';
 END CASE;
 SELECT to_jsonb(t) INTO timed FROM game_item_timed_states t
  WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 -- Only the donor's final portable Swamp Troll corpse is an admitted timed input.
 -- Preserve its timer snapshot on the retired Item; the timed owner refuses lifecycle2.
 IF timed IS DISTINCT FROM NEW.before_timed
   OR (i.definition_production_key='oteryn:item.tibia.i4178' AND timed IS NULL)
   OR (timed IS NOT NULL AND (
     i.definition_production_key<>'oteryn:item.tibia.i4178' OR i.quantity<>1 OR NEW.amount<>1
     OR timed->>'charges' IS NOT NULL
     OR NOT coalesce((timed->>'remaining_ms')::bigint BETWEEN 1 AND 60000,false)
     OR ((timed->>'deadline_at') IS NOT NULL
       AND (timed->>'deadline_at')::bigint<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint)))
 THEN RAISE EXCEPTION 'world contribution timed custody is not a live portable corpse' USING ERRCODE='23514'; END IF;
 IF i.item_instance_id IS NULL OR i.world_id<>NEW.world_id
   OR location IS DISTINCT FROM NEW.before_location
   OR to_jsonb(i) IS DISTINCT FROM NEW.before_item OR i.lifecycle<>1
   OR i.quantity<NEW.amount OR i.definition_family<>'Item'
   OR NOT EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,NEW.world_id) c
     WHERE c.item=i.item_instance_id AND c.location=NEW.before_location AND c.kind=NEW.location_kind)
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'world contribution lacks plain carried custody' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_world_change_debit_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE;
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id()
   OR NOT EXISTS(SELECT 1 FROM game_world_change_contributions c
     WHERE (c.world_id,c.reset_epoch,c.operation_id)=(NEW.world_id,NEW.reset_epoch,NEW.operation_id)
       AND c.amount=NEW.amount AND c.created_xact_id=pg_current_xact_id()
       AND (c.resource='votes')=(NEW.item_instance_id IS NULL)) THEN
   RAISE EXCEPTION 'world contribution and debit must commit together' USING ERRCODE='23514';
 END IF;
 IF NEW.item_instance_id IS NULL THEN
   IF NOT EXISTS(SELECT 1 FROM game_character_quest_tracks t
     JOIN game_character_quest_receipts r ON r.character_id=t.character_id
       AND r.committed_character_revision=t.committed_character_revision
     WHERE t.character_id=NEW.character_id AND t.track_key=NEW.points_track
       AND t.value=NEW.before_points-NEW.amount AND r.created_xact_id=pg_current_xact_id()
       AND r.cause_id=NEW.game_session_id AND r.cause_kind IN ('command','use')
       AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
       AND r.values_before[array_position(r.track_keys,NEW.points_track)]=NEW.before_points
       AND r.values_after[array_position(r.track_keys,NEW.points_track)]=t.value) THEN
     RAISE EXCEPTION 'world vote requires the same atomic Quest point debit' USING ERRCODE='23514';
   END IF;
   RETURN NULL;
 END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id;
 IF NOT coalesce(game_world_change_debit_after(NEW,i),false)
   OR NEW.before_timed IS DISTINCT FROM
      (SELECT to_jsonb(t) FROM game_item_timed_states t WHERE t.item_instance_id=NEW.item_instance_id)
   OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,NEW.world_id) c
     WHERE c.item=i.item_instance_id))
   OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,NEW.world_id) c
     WHERE c.item=i.item_instance_id AND c.location=NEW.before_location AND c.kind=NEW.location_kind)) THEN
   RAISE EXCEPTION 'incomplete world contribution custody debit' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;

-- Extend each existing Item proof chain without weakening its other owner arms.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
   'game_item_container_entry_removal_proven','game_item_nested_entry_proven','game_equipment_slot_proven'] LOOP
   body:=pg_get_functiondef((n||'()')::regprocedure);
   begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
   IF begin_at=0 THEN RAISE EXCEPTION 'unexpected world contribution proof body: %',n; END IF;
   IF n='game_item_instance_guard' THEN
     arm:=$arm$
       IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_world_change_debits r
         WHERE r.item_instance_id=OLD.item_instance_id AND r.before_item=to_jsonb(OLD)
           AND r.created_xact_id=pg_current_xact_id() AND game_world_change_debit_after(r,NEW))
       THEN RETURN NEW; END IF;
$arm$;
   ELSIF n='game_item_instance_change_proven' THEN
     arm:=$arm$
       IF EXISTS(SELECT 1 FROM game_world_change_debits r
         WHERE r.item_instance_id=NEW.item_instance_id AND r.created_xact_id=pg_current_xact_id()
           AND game_world_change_debit_after(r,NEW)) THEN RETURN NULL; END IF;
$arm$;
   ELSE
     arm:=format($arm$
       IF TG_OP='DELETE' AND TG_TABLE_NAME=%L AND EXISTS(
         SELECT 1 FROM game_world_change_debits r JOIN game_item_instances i USING(item_instance_id)
         WHERE r.location_kind=%L AND r.before_location=to_jsonb(OLD)
           AND r.item_instance_id=OLD.item_instance_id AND r.created_xact_id=pg_current_xact_id()
           AND i.quantity=0 AND i.lifecycle=2 AND game_world_change_debit_after(r,i)) THEN
         IF TG_WHEN='BEFORE' THEN RETURN OLD; END IF;
         RETURN NULL;
       END IF;
$arm$,CASE n WHEN 'game_item_container_entry_removal_proven' THEN 'game_item_container_entries'
       WHEN 'game_item_nested_entry_proven' THEN 'game_item_nested_entries' ELSE 'game_character_equipment_slots' END,
       CASE n WHEN 'game_item_container_entry_removal_proven' THEN 'container'
       WHEN 'game_item_nested_entry_proven' THEN 'nested' ELSE 'equipment' END);
   END IF;
   EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
   EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $migration$;

CREATE TRIGGER game_world_change_debit_admit BEFORE INSERT ON game_world_change_debits
 FOR EACH ROW EXECUTE FUNCTION game_world_change_debit_admit();
CREATE CONSTRAINT TRIGGER game_world_change_debit_complete AFTER INSERT ON game_world_change_debits
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_world_change_debit_complete();
CREATE TRIGGER game_world_change_debit_immutable BEFORE UPDATE OR DELETE ON game_world_change_debits
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE TRIGGER game_world_change_debit_no_truncate BEFORE TRUNCATE ON game_world_change_debits
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_world_change_debit_after(game_world_change_debits,game_item_instances)',
   'game_world_change_debit_admit()','game_world_change_debit_complete()'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON game_world_change_debits FROM PUBLIC;
REVOKE ALL ON FUNCTION game_world_change_debit_after(game_world_change_debits,game_item_instances),
 game_world_change_debit_admit(),game_world_change_debit_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_world_change_debits TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_debits TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_world_change_debit_after(game_world_change_debits,game_item_instances)
 TO oteryn_game_runtime;
