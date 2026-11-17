-- Compose the QMECH retirement admission arms into game_item_instance_change_proven()
-- after the accepted MAX501 proofs (2837), which pin its accepted source.
-- Each arm is the exact one its owner migration (2401, 2405, 2605, 2707) used to
-- insert; they are applied in that order, so the latest owner is checked first.
DO $migration$
DECLARE body TEXT; arm TEXT; arms TEXT:=''; at INTEGER; anchor TEXT; exclusions TEXT;
BEGIN
 -- owner migration 2401.
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing Item change proof predecessor for 2401'; END IF;
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_arrival_receipts r WHERE r.transaction_id=NEW.last_transaction_id
       AND r.created_xact_id=pg_current_xact_id() AND r.corpse_before->'item'=to_jsonb(OLD)) THEN
       PERFORM game_elemental_arrival_complete(NEW.last_transaction_id); RETURN NULL;
     END IF;
     $arm$;
 arms:=arm||arms;
 -- owner migration 2405.
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing Item change proof predecessor for 2405'; END IF;
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_loot_retire_receipts r WHERE r.transaction_id=NEW.last_transaction_id
       AND r.created_xact_id=pg_current_xact_id() AND r.item_before=to_jsonb(OLD)) THEN
       PERFORM game_elemental_loot_retire_complete(NEW.item_instance_id); RETURN NULL;
     END IF;
     $arm$;
 arms:=arm||arms;
 -- owner migration 2605.
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing Item change proof predecessor for 2605'; END IF;
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_altar_receipts r,LATERAL jsonb_array_elements(r.inputs) v
       WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND v->'item'=to_jsonb(OLD)) THEN
       PERFORM game_elemental_altar_complete(NEW.last_transaction_id); RETURN NULL;
     END IF;
     $arm$;
 arms:=arm||arms;
 -- owner migration 2707.
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing Item change proof predecessor for 2707'; END IF;
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_dreamer_ring_offerings r WHERE r.transaction_id=NEW.last_transaction_id
     AND r.item_instance_id=OLD.item_instance_id AND r.created_xact_id=pg_current_xact_id() AND r.before_item=to_jsonb(OLD)) THEN
     PERFORM game_dreamer_ring_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
 arms:=arm||arms;
 -- Keep main's complete MAX501 terminal proof first. Add these owners to
 -- its existing mixed-owner exclusion, so a QMECH receipt still selects the
 -- exact before-image/completion arms below. Ordinary reset children must not
 -- pay four extra SPI before-image probes ahead of their bounded cohort proof.
 anchor:=$anchor$    UNION ALL SELECT 1 FROM game_player_death_item_records r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()$anchor$;
 exclusions:=$exclude$
    UNION ALL SELECT 1 FROM game_elemental_arrival_receipts r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_elemental_loot_retire_receipts r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_elemental_altar_receipts r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
    UNION ALL SELECT 1 FROM game_dreamer_ring_offerings r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()$exclude$;
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected MAX501 mixed-owner exclusion';
 END IF;
 body:=replace(body,anchor,anchor||exclusions);
 anchor:=E'  IF EXISTS(SELECT 1 FROM game_item_imbuement_records r\n    WHERE r.action=3';
 at:=strpos(body,anchor);
 IF at=0 OR (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected Item proof following MAX501';
 END IF;
 EXECUTE overlay(body placing arms FROM at FOR 0);
END $migration$;

-- Each cached positive branch is already inside the same membership test.
-- `cached` is a local record, unchanged by either snapshot/fence branch. Do not
-- scan the up-to-501 UUID array twice for each of the terminal custody proofs.
-- Keep the outer membership, cache-owner, snapshot and write-fence predicates.
DO $membership$
DECLARE body TEXT; redundant TEXT:='IF p_item=ANY(cached.proven_items) THEN RETURN TRUE; END IF;';
BEGIN
 body:=pg_get_functiondef('game_world_reset_item_proven(uuid)'::regprocedure);
 IF (length(body)-length(replace(body,redundant,'')))/length(redundant)<>2 THEN
  RAISE EXCEPTION 'unexpected MAX501 cached membership branches';
 END IF;
 EXECUTE replace(body,redundant,'RETURN TRUE;');
END $membership$;

-- Compose the 2500 live-creature output arm with main's 1552 bounded carried
-- exchange. 2500 re-created game_quest_carried_use_complete() from its 1005
-- shape, which dropped the 1552 exchange inputs; both arms are restored here.
DO $migration$
DECLARE body TEXT; original TEXT:=$old$    IF jsonb_array_length(NEW.plan->'take')<>1
        OR NEW.plan #>> '{take,0,item}' IS DISTINCT FROM NEW.plan->>'carried_item'
        OR NEW.plan #>> '{take,0,count}' IS DISTINCT FROM '1'
        OR NOT game_quest_carried_output_valid(NEW.plan)
        OR NEW.created_xact_id<>pg_current_xact_id() THEN$old$;
BEGIN
    body:=pg_get_functiondef('game_quest_carried_use_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed carried quest output proof';
    END IF;
    EXECUTE replace(body,original,$new$    IF jsonb_typeof(NEW.plan->'take') IS DISTINCT FROM 'array'
        OR jsonb_array_length(NEW.plan->'take') NOT BETWEEN 1 AND 8
        OR (SELECT count(*) FROM jsonb_array_elements(NEW.plan->'take') e
            WHERE e->>'item'=NEW.plan->>'carried_item' AND e->>'count'='1')<>1
        OR (SELECT count(DISTINCT e->>'item') FROM jsonb_array_elements(NEW.plan->'take') e)
            <>jsonb_array_length(NEW.plan->'take')
        OR EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') e
            WHERE CASE WHEN e->>'count' ~ '^[0-9]{1,3}$'
                THEN (e->>'count')::integer NOT BETWEEN 1 AND 100 ELSE true END)
        OR (CASE WHEN NEW.plan->'carried_exchange'='true'::jsonb THEN
            jsonb_typeof(NEW.plan->'give') IS DISTINCT FROM 'object'
        ELSE jsonb_array_length(NEW.plan->'take')<>1
            OR NOT game_quest_carried_output_valid(NEW.plan) END)
        OR NEW.created_xact_id<>pg_current_xact_id() THEN$new$);
    EXECUTE format('ALTER FUNCTION game_quest_carried_use_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
