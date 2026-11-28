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
 ) OR EXISTS(
  SELECT 1 FROM jsonb_to_recordset(NEW.rules) AS rule(guild UUID,rank SMALLINT,identity BIGINT,name TEXT,deny BOOLEAN)
  WHERE CASE WHEN rule.name IS NOT NULL THEN
     rule.name='' OR char_length(rule.name)>1999 OR rule.deny IS NULL
     OR rule.guild IS NOT NULL OR rule.rank IS NOT NULL OR rule.identity IS NOT NULL
   ELSE
     NOT EXISTS(SELECT 1 FROM game_guilds g WHERE g.guild_id=rule.guild AND g.world_id=NEW.world_id AND g.state IN(1,2))
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

-- Retry only the most recent star. One rule cannot cause exponential backtracking.
CREATE FUNCTION game_house_acl_name_matches(subject TEXT, pattern TEXT)
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

CREATE FUNCTION game_house_door_access(w UUID,h TEXT,actor UUID,door TEXT,digest BYTEA)
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
 EXECUTE format('ALTER FUNCTION game_house_door_access(UUID,TEXT,UUID,TEXT,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_acl_name_matches(TEXT,TEXT),
 game_house_door_access(UUID,TEXT,UUID,TEXT,BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_door_access(UUID,TEXT,UUID,TEXT,BYTEA) TO oteryn_game_runtime;
