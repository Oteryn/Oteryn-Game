-- DRAFT ONLY: append after the exact source Quest child table/helpers.
-- Preserve the latest installed ordinary guard bodies, including other owners'
-- closed arms. These alternatives cover only current-XID admitted NPC children.
DO $quest_child_guards$
DECLARE signature TEXT; installed_definition TEXT; insertion TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY[
  'game_character_quest_track_guard()',
  'game_character_quest_state_guard()'] LOOP
  installed_definition:=pg_get_functiondef(signature::regprocedure);
  IF position('game_npc_quest_track_proven' IN installed_definition)>0
   OR position('game_npc_quest_state_proven' IN installed_definition)>0 THEN
   RAISE EXCEPTION 'NPC Quest child guard already installed: %',signature;
  END IF;
  IF signature='game_character_quest_track_guard()' THEN
   insertion:=$arm$
    IF TG_OP='INSERT' THEN
     IF game_npc_quest_track_proven(NEW.character_id,NEW.track_key,NEW.quest_key,
       NEW.committed_character_revision,0,NEW.value,TRUE) THEN RETURN NULL; END IF;
    ELSIF TG_OP='UPDATE' THEN
     IF game_npc_quest_track_proven(NEW.character_id,NEW.track_key,NEW.quest_key,
       NEW.committed_character_revision,OLD.value,NEW.value,FALSE) THEN RETURN NULL; END IF;
    END IF;
$arm$;
  ELSE
   insertion:=$arm$
    IF TG_OP='INSERT' THEN
     IF game_npc_quest_state_proven(NEW.character_id,NEW.quest_key,
       NEW.committed_character_revision,NEW.pinned_content_revision,NEW.definition_hash,
       NEW.started_character_revision,NEW.completed_character_revision,TRUE,NULL)
     THEN RETURN NULL; END IF;
    ELSIF TG_OP='UPDATE' THEN
     IF game_npc_quest_state_proven(NEW.character_id,NEW.quest_key,
       NEW.committed_character_revision,NEW.pinned_content_revision,NEW.definition_hash,
       NEW.started_character_revision,NEW.completed_character_revision,FALSE,
       OLD.completed_character_revision) THEN RETURN NULL; END IF;
    END IF;
$arm$;
  END IF;
  IF position(E'\nBEGIN\n' IN installed_definition)=0 THEN
   RAISE EXCEPTION 'Unexpected ordinary Quest guard shape: %',signature;
  END IF;
  installed_definition:=overlay(installed_definition placing E'\nBEGIN\n'||insertion
    from position(E'\nBEGIN\n' IN installed_definition) for length(E'\nBEGIN\n'));
  EXECUTE installed_definition;
 END LOOP;
END $quest_child_guards$;
-- BEFORE row guards are intentionally untouched: revision must increase,
-- identity/pin/start/completion remain immutable according to existing policy.
