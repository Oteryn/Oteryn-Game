ALTER TABLE game_house_acl_guild_ranks ADD COLUMN IF NOT EXISTS raw_lines TEXT[],
 ADD COLUMN IF NOT EXISTS ordered_rules BOOLEAN NOT NULL DEFAULT false;

-- Keep the existing receipt and tenure proof for the ordered selector array.
-- Old guild-only entries have no deny field and remain grants.
CREATE OR REPLACE FUNCTION game_house_acl_guild_ranks_proven() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND ((NEW.world_id,NEW.house_key,NEW.list_key)
     IS DISTINCT FROM (OLD.world_id,OLD.house_key,OLD.list_key)
     OR NEW.acl_revision<=OLD.acl_revision) THEN
  RAISE EXCEPTION 'House guild ACL requires a newer revision on the same list' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS(
  SELECT 1 FROM game_house_management_receipts r JOIN game_house_properties p USING(world_id,house_key)
  WHERE r.game_session_id=NEW.last_session_id AND r.command_id=NEW.last_command_id
   AND r.world_id=NEW.world_id AND r.house_key=NEW.house_key AND r.list_key=NEW.list_key
   AND r.action_kind=1 AND r.created_xact_id=pg_current_xact_id()
   AND r.result_revision=NEW.acl_revision AND p.acl_revision=NEW.acl_revision
   AND p.ownership_generation=NEW.ownership_generation
   AND game_house_acl_session_current(r.game_session_id,r.character_id,r.world_id,NEW.connection_generation)
 ) OR NEW.raw_lines IS NULL OR cardinality(NEW.raw_lines)>100
 OR EXISTS (
  SELECT 1 FROM unnest(NEW.raw_lines) AS line
  WHERE line IS NULL OR (NEW.ordered_rules AND (strpos(line,chr(13))>0 OR strpos(line,chr(10))>0))
 )
 OR (SELECT coalesce(sum(char_length(line)),0) FROM unnest(NEW.raw_lines) AS line)
      + greatest(cardinality(NEW.raw_lines)-1,0)>1999
 OR (SELECT coalesce(sum(char_length(rule.name)),0) + greatest(count(rule.name)-1,0)
     FROM jsonb_to_recordset(NEW.rules) AS rule(name TEXT))>1999
 OR EXISTS(
  SELECT 1 FROM jsonb_to_recordset(NEW.rules) AS rule(guild UUID,rank SMALLINT,identity BIGINT,name TEXT,deny BOOLEAN)
  WHERE CASE WHEN rule.name IS NOT NULL THEN
     NOT NEW.ordered_rules OR rule.name='' OR strpos(rule.name,chr(13))>0 OR strpos(rule.name,chr(10))>0 OR rule.deny IS NULL
     OR rule.guild IS NOT NULL OR rule.rank IS NOT NULL OR rule.identity IS NOT NULL
   ELSE
     (NOT NEW.ordered_rules AND coalesce(rule.deny,false))
     OR NOT EXISTS(SELECT 1 FROM game_guilds g WHERE g.guild_id=rule.guild AND g.world_id=NEW.world_id AND g.state IN(1,2))
     OR (rule.rank IS NULL)<>(rule.identity IS NULL)
     OR (rule.rank IS NOT NULL AND NOT EXISTS(
      SELECT 1 FROM game_guild_ranks gr WHERE gr.guild_id=rule.guild AND gr.level=rule.rank AND gr.house_acl_identity=rule.identity
     ))
   END
 ) THEN
  RAISE EXCEPTION 'House guild ACL requires a fenced advance and qualified selectors' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;

CREATE OR REPLACE FUNCTION game_house_management_guild_acl_advance(s UUID,c NUMERIC,generation NUMERIC,rules JSONB)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_management_receipts%ROWTYPE; p game_house_properties%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_house_management_receipts WHERE game_session_id=s AND command_id=c;
 IF NOT FOUND OR NOT game_house_acl_session_current(s,r.character_id,r.world_id,generation) THEN
  RAISE EXCEPTION 'House guild ACL needs its original current session' USING ERRCODE='23514';
 END IF;
 SELECT * INTO p FROM game_house_properties WHERE world_id=r.world_id AND house_key=r.house_key FOR UPDATE;
 PERFORM game_house_management_acl_advance(s,c);
 INSERT INTO game_house_acl_guild_ranks(world_id,house_key,list_key,ownership_generation,
  acl_revision,previous_acl_revision,connection_generation,rules,raw_lines,ordered_rules,last_session_id,last_command_id)
 VALUES(r.world_id,r.house_key,r.list_key,p.ownership_generation,r.result_revision,p.acl_revision,generation,rules->'rules',
  CASE WHEN jsonb_typeof(rules->'lines')='array' THEN
   ARRAY(SELECT line FROM jsonb_array_elements_text(rules->'lines') WITH ORDINALITY AS lines(line,n) ORDER BY n)
  END,coalesce((rules->>'ordered')::boolean,false),s,c)
 ON CONFLICT(world_id,house_key,list_key) DO UPDATE SET
  ownership_generation=excluded.ownership_generation,acl_revision=excluded.acl_revision,
  previous_acl_revision=excluded.previous_acl_revision,connection_generation=excluded.connection_generation,
  rules=excluded.rules,raw_lines=excluded.raw_lines,ordered_rules=excluded.ordered_rules,last_session_id=excluded.last_session_id,last_command_id=excluded.last_command_id;
END $$;

-- Retry only the most recent star. One rule cannot cause exponential backtracking.
CREATE OR REPLACE FUNCTION game_house_acl_name_matches(subject TEXT, pattern TEXT)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE STRICT SET search_path=pg_catalog,pg_temp AS $$
DECLARE
 subject_at INTEGER := 1;
 pattern_at INTEGER := 1;
 star_at INTEGER := 0;
 consumed_through INTEGER := 0;
 token TEXT;
BEGIN
 subject := lower(subject COLLATE "C");
 pattern := lower(pattern COLLATE "C");
 WHILE subject_at <= char_length(subject) LOOP
  token := substr(pattern,pattern_at,1);
  IF token='*' THEN
   star_at := pattern_at;
   consumed_through := subject_at;
   pattern_at := pattern_at+1;
  ELSIF token='?' OR token=substr(subject,subject_at,1) THEN
   subject_at := subject_at+1;
   pattern_at := pattern_at+1;
  ELSIF star_at>0 THEN
   consumed_through := consumed_through+1;
   subject_at := consumed_through;
   pattern_at := star_at+1;
  ELSE
   RETURN false;
  END IF;
 END LOOP;
 RETURN ltrim(substr(pattern,pattern_at),'*')='';
END $$;

CREATE OR REPLACE FUNCTION game_house_acl_member_lists(p_world UUID,p_house TEXT,p_character UUID)
RETURNS TABLE(list_key TEXT) LANGUAGE sql STABLE SECURITY DEFINER AS $$
 WITH actor_name AS (
  SELECT c.name
  FROM game_character_roots c
  WHERE c.character_id=p_character AND c.world_id=p_world AND c.lifecycle=1
 ), membership AS (
  SELECT m.guild_id,gr.level,gr.house_acl_identity
  FROM game_guild_members m
  JOIN game_guilds g ON g.guild_id=m.guild_id AND g.world_id=p_world AND g.state IN(1,2)
  JOIN game_guild_ranks gr ON gr.guild_id=m.guild_id AND gr.level=m.rank_level
  WHERE m.character_id=p_character
 ), decisions AS (
  SELECT a.list_key,first_match.deny
  FROM game_house_acl_guild_ranks a
  JOIN game_house_properties p USING(world_id,house_key)
  CROSS JOIN actor_name c
  CROSS JOIN LATERAL (
   SELECT coalesce(rule.deny,false) AS deny
   FROM ROWS FROM (jsonb_to_recordset(a.rules) AS (
    guild UUID,rank SMALLINT,identity BIGINT,name TEXT,deny BOOLEAN
   )) WITH ORDINALITY AS rule(guild,rank,identity,name,deny,position)
   WHERE CASE WHEN rule.name IS NOT NULL THEN
     game_house_acl_name_matches(c.name COLLATE "C",rule.name)
    ELSE EXISTS(
     SELECT 1 FROM membership m
     WHERE rule.guild=m.guild_id AND (rule.rank IS NULL OR
           (rule.rank=m.level AND rule.identity=m.house_acl_identity))
    )
   END
   ORDER BY rule.position
   LIMIT 1
  ) first_match
  WHERE a.world_id=p_world AND a.house_key=p_house AND a.ownership_generation=p.ownership_generation
 )
 SELECT d.list_key FROM decisions d WHERE NOT d.deny
 UNION
 SELECT e.list_key FROM game_house_acl_entries e
 WHERE e.world_id=p_world AND e.house_key=p_house AND e.character_id=p_character
   AND NOT EXISTS(SELECT 1 FROM decisions d WHERE d.list_key=e.list_key)
$$;

-- Older rows have qualified identities only. Render the live names, and keep
-- stale identities inert when no current selector could represent them.
CREATE OR REPLACE FUNCTION game_house_acl_text(w UUID,h TEXT,list TEXT)
RETURNS TABLE(lines TEXT[], ordered BOOLEAN) LANGUAGE sql STABLE AS $$
 SELECT coalesce(a.raw_lines, ARRAY(
  SELECT CASE
   WHEN g.name IS NULL OR (rule.rank IS NOT NULL AND gr.name IS NULL)
    THEN '# Unavailable guild selector'
   ELSE coalesce(gr.name,'') || '@' || g.name
   END
  FROM ROWS FROM (jsonb_to_recordset(a.rules) AS (guild UUID,rank SMALLINT,identity BIGINT))
    WITH ORDINALITY AS rule(guild,rank,identity,position)
  LEFT JOIN game_guilds g ON g.guild_id=rule.guild AND g.world_id=w AND g.state IN(1,2)
  LEFT JOIN game_guild_ranks gr ON gr.guild_id=g.guild_id AND gr.level=rule.rank AND gr.house_acl_identity=rule.identity
  ORDER BY rule.position
 )), a.ordered_rules
 FROM game_house_acl_guild_ranks a
 JOIN game_house_properties p USING(world_id,house_key)
 WHERE a.world_id=w AND a.house_key=h AND a.list_key=list AND a.ownership_generation=p.ownership_generation
$$;

CREATE OR REPLACE FUNCTION game_house_door_access(w UUID,h TEXT,actor UUID,door TEXT,digest BYTEA)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS (
  SELECT 1 FROM game_house_properties p
  WHERE p.world_id=w AND p.house_key=h AND p.state IN(3,4)
   AND p.content_digest=digest AND NOT game_house_access_fence(w,h)
   AND door LIKE 'door:%'
   AND (game_house_effective_owner(w,h)=actor OR EXISTS (
    SELECT 1 FROM game_house_acl_member_lists(w,h,actor) a WHERE a.list_key=door
   ))
 )
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_acl_guild_ranks_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_acl_member_lists(UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_management_guild_acl_advance(UUID,NUMERIC,NUMERIC,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_acl_text(UUID,TEXT,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_door_access(UUID,TEXT,UUID,TEXT,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_acl_name_matches(TEXT,TEXT),
 game_house_acl_text(UUID,TEXT,TEXT), game_house_door_access(UUID,TEXT,UUID,TEXT,BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_acl_text(UUID,TEXT,TEXT), game_house_door_access(UUID,TEXT,UUID,TEXT,BYTEA) TO oteryn_game_runtime;
