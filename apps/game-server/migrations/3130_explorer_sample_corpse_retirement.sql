-- Explorer collection retires the exact sampled body and its native loot in the
-- same transaction as the carried tool BURN, sample MINT, and quest receipt.
CREATE TABLE game_quest_sample_retirements (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances,
 corpse_item UUID NOT NULL REFERENCES game_item_instances,
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 before_item JSONB NOT NULL,
 before_custody JSONB NOT NULL,
 character_id UUID NOT NULL,
 cause_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL,
 transition_key TEXT NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
  REFERENCES game_quest_corpse_samples(character_id,cause_id,cause_ordinal,transition_key)
);
CREATE INDEX game_quest_sample_retirements_sample
 ON game_quest_sample_retirements(character_id,cause_id,cause_ordinal,transition_key);
CREATE INDEX game_quest_sample_retirements_corpse ON game_quest_sample_retirements(corpse_item);
CREATE TRIGGER game_quest_sample_retirements_immutable BEFORE UPDATE OR DELETE ON game_quest_sample_retirements
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_sample_retirements_no_truncate BEFORE TRUNCATE ON game_quest_sample_retirements
 EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_quest_sample_retirements FROM PUBLIC;
GRANT SELECT ON game_quest_sample_retirements TO oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_quest_sample_retirement_proven(p_item UUID,p_custody JSONB)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_quest_sample_retirements r JOIN game_item_instances i USING(item_instance_id)
 WHERE r.item_instance_id=p_item AND r.created_xact_id=pg_current_xact_id()
   AND (p_custody IS NULL OR r.before_custody=p_custody)
   AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
   AND i.state_revision=(r.before_item->>'state_revision')::numeric+1
   AND (to_jsonb(i)-ARRAY['lifecycle','quantity','last_transaction_id','state_revision'])
       =(r.before_item-ARRAY['lifecycle','quantity','last_transaction_id','state_revision'])
   AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=p_item)
   AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)
   AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=p_item OR parent_item_instance_id=p_item))
$$;

CREATE FUNCTION game_quest_sample_is_retired(p_sample game_quest_corpse_samples)
RETURNS BOOLEAN LANGUAGE sql STABLE STRICT AS $$
 SELECT EXISTS(SELECT 1 FROM game_quest_sample_retirements r WHERE r.item_instance_id=p_sample.corpse_item
   AND r.corpse_item=p_sample.corpse_item AND r.before_item=p_sample.before_item
   AND (r.character_id,r.cause_id,r.cause_ordinal,r.transition_key)
      =(p_sample.character_id,p_sample.cause_id,p_sample.cause_ordinal,p_sample.transition_key)
   AND game_quest_sample_retirement_proven(r.item_instance_id,p_sample.before_ground))
$$;

CREATE FUNCTION game_quest_retire_sample(p_sample game_quest_corpse_samples)
RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE member RECORD; retired UUID; retired_count INTEGER:=0;
BEGIN
 IF game_quest_sample_is_retired(p_sample) THEN
  RETURN;
 END IF;
 -- The Ground owner lock was taken before reading the original sample. All
 -- ordinary corpse loot custody changes share it. Traverse only its own tree.
 FOR member IN
  WITH RECURSIVE tree(id,kind,depth,path,custody) AS (
   SELECT p_sample.corpse_item,0,0,ARRAY[p_sample.corpse_item],p_sample.before_ground
   UNION ALL SELECT edge.id,edge.kind,t.depth+1,t.path||edge.id,edge.custody
   FROM tree t CROSS JOIN LATERAL (
    SELECT e.item_instance_id AS id,1 AS kind,to_jsonb(e) AS custody
    FROM game_item_corpse_container_entries e WHERE t.depth=0 AND e.parent_item_instance_id=t.id
    UNION ALL
    SELECT e.item_instance_id,2,to_jsonb(e)
    FROM game_item_nested_entries e WHERE e.parent_item_instance_id=t.id
   ) edge WHERE t.depth<9 AND NOT edge.id=ANY(t.path)
  )
  SELECT t.*,to_jsonb(i) AS item,i.lifecycle,i.world_id
  FROM (SELECT * FROM tree LIMIT 4097) t JOIN game_item_instances i ON i.item_instance_id=t.id
  ORDER BY t.depth DESC,t.id FOR UPDATE OF i
 LOOP
  retired_count:=retired_count+1;
  IF retired_count>4096 OR member.depth>8 OR member.lifecycle<>1 OR member.world_id<>p_sample.world_id
     OR (member.kind=0 AND member.item IS DISTINCT FROM p_sample.before_item)
     OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=member.id)
     OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=member.id)
  THEN RAISE EXCEPTION 'sampled corpse has changed custody' USING ERRCODE='23514'; END IF;
  retired:=game_character_uuid_v7();
  INSERT INTO game_quest_sample_retirements(item_instance_id,corpse_item,transaction_id,before_item,before_custody,
   character_id,cause_id,cause_ordinal,transition_key)
   VALUES(member.id,p_sample.corpse_item,retired,member.item,member.custody,
    p_sample.character_id,p_sample.cause_id,p_sample.cause_ordinal,p_sample.transition_key);
  UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=retired WHERE item_instance_id=member.id;
  CASE member.kind
   WHEN 0 THEN DELETE FROM game_item_ground_locations WHERE item_instance_id=member.id;
   WHEN 1 THEN DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=member.id;
   WHEN 2 THEN DELETE FROM game_item_nested_entries WHERE item_instance_id=member.id;
  END CASE;
  IF NOT FOUND THEN RAISE EXCEPTION 'sampled corpse member lost custody' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF NOT game_quest_sample_is_retired(p_sample) THEN
  RAISE EXCEPTION 'sampled corpse was not retired' USING ERRCODE='23514';
 END IF;
END $$;

DO $migration$
DECLARE body TEXT; guard TEXT; arm TEXT; start_at INTEGER; anchor TEXT;
BEGIN
 FOR guard IN SELECT unnest(ARRAY['game_item_instance_guard','game_item_instance_change_proven',
     'game_item_ground_removal_proven','game_item_corpse_entry_removal_proven','game_item_nested_entry_proven']) LOOP
  body:=pg_get_functiondef((guard||'()')::regprocedure);
  start_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF start_at=0 THEN RAISE EXCEPTION 'missing corpse sample predecessor: %',guard; END IF;
  arm:=CASE guard
   WHEN 'game_item_instance_guard' THEN $guard$
    IF TG_OP='UPDATE' AND NEW.lifecycle=2 AND NEW.quantity=0
     AND EXISTS(SELECT 1 FROM game_quest_sample_retirements r
      WHERE r.item_instance_id=OLD.item_instance_id AND r.before_item=to_jsonb(OLD)
       AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id())
     AND (to_jsonb(NEW)-ARRAY['lifecycle','quantity','last_transaction_id','state_revision'])
         =(to_jsonb(OLD)-ARRAY['lifecycle','quantity','last_transaction_id','state_revision'])
    THEN NEW.state_revision:=OLD.state_revision+1; RETURN NEW; END IF;
$guard$
   WHEN 'game_item_instance_change_proven' THEN $guard$
    IF TG_OP='UPDATE' AND NEW.lifecycle=2 AND NEW.quantity=0
     AND EXISTS(SELECT 1 FROM game_quest_sample_retirements r
      WHERE r.item_instance_id=OLD.item_instance_id AND r.transaction_id=NEW.last_transaction_id
       AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id())
     AND game_quest_sample_retirement_proven(NEW.item_instance_id,NULL)
    THEN RETURN NULL; END IF;
$guard$
   ELSE $guard$
    IF TG_OP='DELETE' AND game_quest_sample_retirement_proven(OLD.item_instance_id,to_jsonb(OLD))
    THEN RETURN NULL; END IF;
$guard$ END;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from start_at for 5);
 END LOOP;

 body:=pg_get_functiondef('game_quest_corpse_sample_complete()'::regprocedure);
 anchor:='sample_row RECORD;';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'missing sampled corpse row declaration';
 END IF;
 body:=replace(body,anchor,'sample_row game_quest_corpse_samples%ROWTYPE;');
 anchor:='AND game_quest_corpse_adjacent(g.spatial_position,sample_row.actor_position))';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'missing fresh sampled corpse predicate';
 END IF;
 body:=replace(body,anchor,anchor||'
   AND NOT game_quest_sample_is_retired(sample_row)');
 anchor:=E' RETURN NULL;\nEND';
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'missing sampled corpse completion arm';
 END IF;
 EXECUTE replace(body,anchor,$guard$
 IF source_row.plan->>'quest'='oteryn:quest.the_explorer_society_quest'
  AND source_row.transition_key IN (
   'oteryn:quest-reward/explorer-society/prepare-butterfly',
   'oteryn:quest-reward/explorer-society/collect/butterfly-purple',
   'oteryn:quest-reward/explorer-society/collect/butterfly-blue',
   'oteryn:quest-reward/explorer-society/collect/butterfly-red',
   'oteryn:quest-reward/explorer-society/collect/ectoplasm')
 THEN PERFORM game_quest_retire_sample(sample_row); END IF;
 RETURN NULL;
END$guard$);
 EXECUTE format('ALTER FUNCTION game_quest_sample_is_retired(game_quest_corpse_samples) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_quest_retire_sample(game_quest_corpse_samples) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_quest_sample_retirement_proven(UUID,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
ALTER FUNCTION game_quest_corpse_sample_complete() SECURITY DEFINER;
REVOKE ALL ON FUNCTION game_quest_retire_sample(game_quest_corpse_samples),
 game_quest_sample_is_retired(game_quest_corpse_samples),game_quest_sample_retirement_proven(UUID,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_sample_retirement_proven(UUID,JSONB) TO oteryn_game_runtime;
