-- Source-qualified: foreign source writes stay outside generic writable_tracks authority.
CREATE TABLE game_npc_source_quest_policies(
 program_sha256 BYTEA NOT NULL,
 source_key TEXT NOT NULL,
 foreign_tracks JSONB NOT NULL CHECK(jsonb_typeof(foreign_tracks)='object'),
 PRIMARY KEY(program_sha256,source_key),
 FOREIGN KEY(program_sha256,source_key) REFERENCES game_npc_source_policies(program_sha256,source_key)
);
REVOKE ALL ON game_npc_source_quest_policies FROM PUBLIC,oteryn_game_runtime;
GRANT SELECT ON game_npc_source_quest_policies TO oteryn_game_runtime;
CREATE TRIGGER game_npc_source_quest_policy_immutable BEFORE UPDATE OR DELETE ON game_npc_source_quest_policies
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_npc_source_quest_policy_no_truncate BEFORE TRUNCATE ON game_npc_source_quest_policies
 FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();

-- Called by the already authenticated Node arm, after that arm independently
-- checked the locked actual source observation. Each foreign write is covered
-- once by one exact Quest child; it never enters the generic parent row proof.
CREATE FUNCTION game_npc_quest_plan_valid(p_program BYTEA,p_source TEXT,p_character UUID,p_before JSONB,p_plan JSONB)
 RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE policy game_npc_source_quest_policies%ROWTYPE; plan_delta JSONB; rule JSONB; source_tracks JSONB;
BEGIN
 SELECT * INTO policy FROM game_npc_source_quest_policies WHERE program_sha256=p_program AND source_key=p_source;
 IF NOT FOUND THEN RETURN FALSE;END IF;
 source_tracks:=game_npc_quest_observed_tracks(p_character,p_source,p_before);
 IF jsonb_typeof(p_plan->'quest_changes') IS DISTINCT FROM 'array'
 OR jsonb_typeof(p_plan->'quest_children') IS DISTINCT FROM 'array'
 OR jsonb_typeof(p_plan->'quest_noops') IS DISTINCT FROM 'array'
 OR jsonb_array_length(p_plan->'quest_changes')<>(SELECT count(*) FROM jsonb_object_keys(policy.foreign_tracks))
 OR jsonb_array_length(p_plan->'quest_changes') NOT BETWEEN 1 AND 8
 OR jsonb_array_length(p_plan->'quest_children') NOT BETWEEN 0 AND 8
 OR jsonb_array_length(p_plan->'quest_noops')>8
 OR (SELECT count(DISTINCT value->>'key') FROM jsonb_array_elements(p_plan->'quest_noops'))<>jsonb_array_length(p_plan->'quest_noops')
 OR (SELECT count(DISTINCT value->>'key') FROM jsonb_array_elements(p_plan->'quest_changes'))<>jsonb_array_length(p_plan->'quest_changes')
 OR (SELECT count(DISTINCT value->>'quest') FROM jsonb_array_elements(p_plan->'quest_children'))<>jsonb_array_length(p_plan->'quest_children') THEN RETURN FALSE;END IF;
 FOR plan_delta IN SELECT value FROM jsonb_array_elements(p_plan->'quest_changes') LOOP
  rule:=policy.foreign_tracks->(plan_delta->>'key');
  IF rule IS NULL OR plan_delta->>'quest' IS DISTINCT FROM rule->>'quest'
   OR (plan_delta->>'before')::bigint IS DISTINCT FROM coalesce((source_tracks->>(plan_delta->>'key'))::bigint,-1)
   OR (plan_delta->>'after')::bigint IS DISTINCT FROM game_npc_source_expression(rule->'value',source_tracks,(p_before->>'now')::bigint)
   OR (SELECT count(*) FROM jsonb_array_elements(p_plan->'quest_children') child,
      LATERAL jsonb_array_elements(child->'changes') child_delta
      WHERE child->>'quest'=plan_delta->>'quest' AND child_delta->>'key'=plan_delta->>'key'
       AND child_delta->'source_before'=plan_delta->'before' AND child_delta->'after'=plan_delta->'after')
      +(SELECT count(*) FROM jsonb_array_elements(p_plan->'quest_noops') noop
       JOIN game_character_quest_tracks actual ON actual.character_id=p_character
        AND actual.track_key=noop->>'key' AND actual.quest_key=noop->>'quest'
        AND actual.value=(noop->>'value')::bigint
       WHERE noop->>'key'=plan_delta->>'key' AND noop->>'quest'=plan_delta->>'quest'
        AND noop->'value'=plan_delta->'before' AND noop->'value'=plan_delta->'after')<>1 THEN RETURN FALSE;END IF;
 END LOOP;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(p_plan->'quest_noops') noop
  WHERE NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p_plan->'quest_changes') source_delta
   JOIN game_character_quest_tracks actual ON actual.character_id=p_character
    AND actual.track_key=noop->>'key' AND actual.quest_key=noop->>'quest'
    AND actual.value=(noop->>'value')::bigint
   WHERE source_delta->>'key'=noop->>'key' AND source_delta->>'quest'=noop->>'quest'
    AND source_delta->'before'=noop->'value' AND source_delta->'after'=noop->'value')) THEN RETURN FALSE;END IF;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(p_plan->'quest_children') child,
  LATERAL jsonb_array_elements(child->'changes') child_delta
  WHERE child->'writes_state' IS DISTINCT FROM to_jsonb(child->>'quest'<>'oteryn:quest.npc_cosmetic_final_acquisition')
   OR child_delta->>'quest' IS DISTINCT FROM child->>'quest'
   OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p_plan->'quest_changes') source_delta
    WHERE source_delta->>'key'=child_delta->>'key' AND source_delta->>'quest'=child->>'quest'
     AND source_delta->'before'=child_delta->'source_before' AND source_delta->'after'=child_delta->'after')) THEN RETURN FALSE;END IF;
 RETURN TRUE;
END $$;
REVOKE ALL ON FUNCTION game_npc_quest_plan_valid(BYTEA,TEXT,UUID,JSONB,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_quest_plan_valid(BYTEA,TEXT,UUID,JSONB,JSONB) TO oteryn_game_runtime;
DO $path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_quest_plan_valid(bytea,text,uuid,jsonb,jsonb) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $path$;
