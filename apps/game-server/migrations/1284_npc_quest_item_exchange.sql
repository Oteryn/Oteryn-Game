-- Ordered NPC recipes reuse the carried quest input owner. Its new arm is
-- restricted to an NPC response and quest receipt created in this transaction;
-- the existing world USE/STEP and custody proof arms remain unchanged.
DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_quest_callback_complete()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing carried quest completion guard'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF NEW.location_kind IN ('container','nested') AND EXISTS(
    SELECT 1 FROM game_character_quest_receipts r
    JOIN game_quest_npc_program_receipts n ON n.character_id=r.character_id
      AND n.game_session_id=r.cause_id AND n.command_id=r.cause_ordinal
      AND n.transition_key=r.transition_key
    WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.cause_id
      AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
      AND r.cause_kind='command' AND r.transition_key LIKE 'oteryn:quest-npc/%'
      AND r.created_xact_id=pg_current_xact_id()
  ) THEN
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id;
    IF NEW.created_xact_id<>pg_current_xact_id()
      OR NOT coalesce(game_quest_callback_after(NEW,i),false)
      OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,i.world_id) c WHERE c.item=i.item_instance_id))
      OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,i.world_id) c
        WHERE c.item=i.item_instance_id AND c.location=NEW.before_location AND c.kind=NEW.location_kind))
    THEN RAISE EXCEPTION 'incomplete atomic NPC quest input' USING ERRCODE='23514'; END IF;
    RETURN NULL;
  END IF;
$arm$ from begin_at for 5);
END $migration$;
