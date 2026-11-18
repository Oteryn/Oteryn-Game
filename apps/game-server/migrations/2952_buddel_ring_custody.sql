-- Use the carried-input proof for Buddel, with the travel command as its owner.
-- Quest-owned inputs retain their original non-null cause and composite FK.
ALTER TABLE game_quest_callback_consumptions
 ALTER COLUMN transition_key DROP NOT NULL,
 ADD COLUMN travel_occurrence UUID UNIQUE REFERENCES game_newhaven_travel_receipts(occurrence_id)
   DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN travel_equipment_revision NUMERIC(20,0),
 ADD CONSTRAINT game_carried_input_owner CHECK ((
   (travel_occurrence IS NULL AND transition_key IS NOT NULL AND travel_equipment_revision IS NULL)
   OR (travel_occurrence IS NOT NULL AND transition_key IS NULL AND quantity=1 AND replacement IS NULL
     AND location_kind IN ('container','nested','equipment')
     AND before_item->>'definition_production_key'='oteryn:item.tibia.i3097'
     AND (before_item->>'quantity')::bigint=1
     AND ((location_kind='equipment')=(travel_equipment_revision IS NOT NULL)))) IS TRUE);

DO $migration$
DECLARE body TEXT; start_at INT;
 timed_item TEXT := '        OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)';
BEGIN
 -- A carried dwarven ring can have a time checkpoint. Keep that immutable
 -- history, just as expiry does, and forbid an outstanding tile deadline.
 body:=pg_get_functiondef('game_quest_callback_admit()'::regprocedure);
 timed_item:=E'\n'||timed_item;
 IF (length(body)-length(replace(body,timed_item,'')))/length(timed_item)<>1 THEN
   RAISE EXCEPTION 'missing or ambiguous carried input timed guard';
 END IF;
 EXECUTE replace(body,timed_item,$arm$
        OR EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id
            AND (NEW.travel_occurrence IS NULL OR t.deadline_at IS NOT NULL))$arm$);

 body:=pg_get_functiondef('game_quest_callback_complete()'::regprocedure);
 start_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF start_at=0 THEN RAISE EXCEPTION 'missing carried input completion guard'; END IF;
 EXECUTE overlay(body placing $arm$BEGIN
 IF NEW.travel_occurrence IS NOT NULL THEN
   SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id;
   IF NEW.created_xact_id<>pg_current_xact_id()
      OR NOT coalesce(game_quest_callback_after(NEW,i),false)
      OR EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,i.world_id,i.item_instance_id))
      OR NOT EXISTS(
          SELECT 1 FROM game_newhaven_travel_receipts r
          JOIN game_character_pending_arrivals a USING(character_id,occurrence_id)
          WHERE r.occurrence_id=NEW.travel_occurrence AND r.character_id=NEW.character_id
            AND r.game_session_id=NEW.cause_id AND r.command_id=NEW.cause_ordinal
            AND encode(r.request_binding,'hex')=convert_from(NEW.proof,'UTF8')::jsonb->>'travel_binding'
            AND r.created_xact_id=pg_current_xact_id() AND a.world_id=i.world_id
            AND r.route IN ('oteryn:service.travel.buddel/camp_ring','oteryn:service.travel.buddel/helheim_ring',
                            'oteryn:service.travel.buddel/okolnir_ring','oteryn:service.travel.buddel/tyrsung_ring')
            AND r.fare_gold_units=0 AND r.base_fare_gold_units=0)
      OR (NEW.location_kind='equipment' AND NOT EXISTS(
          SELECT 1 FROM game_character_equipment_state e WHERE e.character_id=NEW.character_id
            AND e.last_transaction_id=NEW.transaction_id AND e.revision=NEW.travel_equipment_revision+1))
   THEN RAISE EXCEPTION 'Buddel ring handover requires its matching arrival and custody update' USING ERRCODE='23514'; END IF;
   RETURN NULL;
 END IF;
$arm$ from start_at for 5);

 body:=pg_get_functiondef('game_equipment_state_proven()'::regprocedure);
 start_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF start_at=0 THEN RAISE EXCEPTION 'missing equipment successor guard'; END IF;
 EXECUTE overlay(body placing $arm$BEGIN
 IF NEW.combat_mode IS NOT DISTINCT FROM OLD.combat_mode AND EXISTS(
     SELECT 1 FROM game_quest_callback_consumptions c
     WHERE c.travel_occurrence IS NOT NULL AND c.location_kind='equipment'
       AND c.character_id=NEW.character_id AND c.transaction_id=NEW.last_transaction_id
       AND c.travel_equipment_revision=OLD.revision AND NEW.revision=OLD.revision+1
       AND c.created_xact_id=pg_current_xact_id())
 THEN RETURN NULL; END IF;
$arm$ from start_at for 5);

 body:=pg_get_functiondef('game_npc_travel_receipt_guard()'::regprocedure);
 start_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF start_at=0 THEN RAISE EXCEPTION 'missing NPC travel receipt guard'; END IF;
 EXECUTE overlay(body placing $arm$BEGIN
 IF NEW.route IN ('oteryn:service.travel.buddel/camp_ring','oteryn:service.travel.buddel/helheim_ring',
                  'oteryn:service.travel.buddel/okolnir_ring','oteryn:service.travel.buddel/tyrsung_ring')
    AND NOT EXISTS(SELECT 1 FROM game_quest_callback_consumptions c
                   WHERE c.travel_occurrence=NEW.occurrence_id AND c.created_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'Buddel ring travel requires a matching handover' USING ERRCODE='23514'; END IF;
$arm$ from start_at for 5);
END $migration$;
