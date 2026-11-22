-- A rank is addressed by (guild_id, level). The identity distinguishes a rank
-- deleted and recreated at the same level, without binding access to its name.
ALTER TABLE game_guild_ranks ADD COLUMN house_acl_identity BIGINT GENERATED ALWAYS AS IDENTITY;

-- This is an access list, outside the guild-disband payout participant namespace.
CREATE TABLE game_house_acl_guild_ranks (
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 list_key TEXT NOT NULL CHECK(list_key IN('guest','subowner') OR list_key ~ '^door:[0-9]{1,5}:[0-9]{1,5}:-?[0-9]{1,2}$'),
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation>0),
 acl_revision NUMERIC(20,0) NOT NULL,
 previous_acl_revision NUMERIC(20,0) NOT NULL CHECK(previous_acl_revision>=0),
 connection_generation NUMERIC(20,0) NOT NULL CHECK(connection_generation>0),
 rules JSONB NOT NULL CHECK(jsonb_typeof(rules)='array' AND jsonb_array_length(rules)<=100),
 last_session_id UUID NOT NULL,
 last_command_id NUMERIC(20,0) NOT NULL,
 PRIMARY KEY(world_id,house_key,list_key),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties,
 FOREIGN KEY(last_session_id,last_command_id) REFERENCES game_house_management_receipts,
 CHECK(acl_revision=previous_acl_revision+1)
);
CREATE INDEX game_house_acl_guild_ranks_receipt ON game_house_acl_guild_ranks(last_session_id,last_command_id);
REVOKE ALL ON game_house_acl_guild_ranks FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_acl_guild_ranks TO oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_house_acl_session_current(s UUID, actor UUID, w UUID, generation NUMERIC)
RETURNS BOOLEAN LANGUAGE sql AS $$
 SELECT EXISTS(
  SELECT 1 FROM game_durability_reconnect_sessions sess
  JOIN game_durability_admission_character_guards cg ON cg.character_id=sess.character_id
  WHERE sess.game_session_id=s AND sess.character_id=actor AND sess.world_id=w
   AND sess.current_generation=generation AND sess.session_state=2
   AND cg.eligible AND cg.lease_generation=sess.character_lease_generation
   AND cg.holder_game_session_id=s AND cg.world_id=w
  FOR SHARE OF sess,cg
 )
$$;

CREATE FUNCTION game_house_acl_guild_ranks_proven() RETURNS TRIGGER LANGUAGE plpgsql AS $$
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
  SELECT 1 FROM jsonb_to_recordset(NEW.rules) AS rule(guild UUID,rank SMALLINT,identity BIGINT)
  WHERE NOT EXISTS(SELECT 1 FROM game_guilds g WHERE g.guild_id=rule.guild AND g.world_id=NEW.world_id AND g.state IN(1,2))
   OR (rule.rank IS NULL)<>(rule.identity IS NULL)
   OR (rule.rank IS NOT NULL AND NOT EXISTS(
    SELECT 1 FROM game_guild_ranks gr WHERE gr.guild_id=rule.guild AND gr.level=rule.rank AND gr.house_acl_identity=rule.identity
   ))
 ) THEN
  RAISE EXCEPTION 'House guild ACL requires a fenced advance and qualified selectors' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_acl_guild_ranks_proven AFTER INSERT OR UPDATE ON game_house_acl_guild_ranks
 FOR EACH ROW EXECUTE FUNCTION game_house_acl_guild_ranks_proven();

-- The runtime cannot write the table directly. This port owns both the existing
-- authorized revision advance and the guild selectors it publishes.
CREATE FUNCTION game_house_management_guild_acl_advance(s UUID,c NUMERIC,generation NUMERIC,rules JSONB)
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
  acl_revision,previous_acl_revision,connection_generation,rules,last_session_id,last_command_id)
 VALUES(r.world_id,r.house_key,r.list_key,p.ownership_generation,r.result_revision,p.acl_revision,generation,rules,s,c)
 ON CONFLICT(world_id,house_key,list_key) DO UPDATE SET
  ownership_generation=excluded.ownership_generation,acl_revision=excluded.acl_revision,
  previous_acl_revision=excluded.previous_acl_revision,connection_generation=excluded.connection_generation,
  rules=excluded.rules,last_session_id=excluded.last_session_id,last_command_id=excluded.last_command_id;
END $$;

CREATE FUNCTION game_house_acl_member_lists(p_world UUID,p_house TEXT,p_character UUID)
RETURNS TABLE(list_key TEXT) LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT e.list_key FROM game_house_acl_entries e
 WHERE e.world_id=p_world AND e.house_key=p_house AND e.character_id=p_character
 UNION
 SELECT a.list_key FROM game_house_acl_guild_ranks a
 JOIN game_house_properties p USING(world_id,house_key)
 JOIN game_guild_members m ON m.character_id=p_character
 JOIN game_guilds g ON g.guild_id=m.guild_id AND g.world_id=p_world AND g.state IN(1,2)
 JOIN game_guild_ranks gr ON gr.guild_id=m.guild_id AND gr.level=m.rank_level
 WHERE a.world_id=p_world AND a.house_key=p_house AND a.ownership_generation=p.ownership_generation
  AND EXISTS(
   SELECT 1 FROM jsonb_to_recordset(a.rules) AS rule(guild UUID,rank SMALLINT,identity BIGINT)
   WHERE rule.guild=m.guild_id AND (rule.rank IS NULL OR (rule.rank=gr.level AND rule.identity=gr.house_acl_identity))
  )
$$;

CREATE OR REPLACE FUNCTION game_house_access_grant(p_world UUID,p_house TEXT,p_character UUID)
RETURNS TEXT LANGUAGE sql STABLE AS $$
 SELECT CASE WHEN bool_or(list_key='subowner') THEN 'SUBOWNER'
             WHEN bool_or(list_key='guest') THEN 'GUEST' END
 FROM game_house_acl_member_lists(p_world,p_house,p_character)
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_acl_session_current(UUID,UUID,UUID,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_acl_guild_ranks_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_management_guild_acl_advance(UUID,NUMERIC,NUMERIC,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_acl_member_lists(UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_access_grant(UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_acl_session_current(UUID,UUID,UUID,NUMERIC),
 game_house_acl_guild_ranks_proven(),game_house_management_guild_acl_advance(UUID,NUMERIC,NUMERIC,JSONB),
 game_house_acl_member_lists(UUID,TEXT,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_management_guild_acl_advance(UUID,NUMERIC,NUMERIC,JSONB) TO oteryn_game_runtime;
