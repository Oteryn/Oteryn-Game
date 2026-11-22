-- Guild selectors share the canonical House ACL revision and owner receipt.
-- Ownership generation prevents a selector from following a property to its next owner.
CREATE TABLE game_house_guild_acl (
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 list_key TEXT NOT NULL CHECK(list_key IN('guest','subowner')),
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation>0),
 rules JSONB NOT NULL CHECK(jsonb_typeof(rules)='array' AND jsonb_array_length(rules)<=100),
 last_session_id UUID NOT NULL,
 last_command_id NUMERIC(20,0) NOT NULL,
 PRIMARY KEY(world_id,house_key,list_key),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties,
 FOREIGN KEY(last_session_id,last_command_id) REFERENCES game_house_management_receipts DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_house_guild_acl_receipt ON game_house_guild_acl(last_session_id,last_command_id);
REVOKE ALL ON game_house_guild_acl FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_house_guild_acl TO oteryn_game_runtime;
GRANT SELECT ON game_house_guild_acl TO oteryn_game_control;

CREATE FUNCTION game_house_guild_acl_proven() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND (NEW.world_id,NEW.house_key,NEW.list_key)
     IS DISTINCT FROM (OLD.world_id,OLD.house_key,OLD.list_key) THEN
  RAISE EXCEPTION 'House guild ACL principal is immutable' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS(
  SELECT 1 FROM game_house_management_receipts r JOIN game_house_properties p USING(world_id,house_key)
  WHERE r.game_session_id=NEW.last_session_id AND r.command_id=NEW.last_command_id
   AND r.world_id=NEW.world_id AND r.house_key=NEW.house_key AND r.list_key=NEW.list_key
   AND r.action_kind=1 AND r.created_xact_id=pg_current_xact_id() AND r.result_revision=p.acl_revision
   AND p.ownership_generation=NEW.ownership_generation
 ) OR EXISTS(
  SELECT 1 FROM jsonb_to_recordset(NEW.rules) AS rule(guild UUID,rank SMALLINT,name TEXT)
  WHERE NOT EXISTS(SELECT 1 FROM game_guilds g WHERE g.guild_id=rule.guild AND g.world_id=NEW.world_id AND g.state IN(1,2))
   OR (rule.rank IS NULL)<>(rule.name IS NULL)
   OR (rule.rank IS NOT NULL AND NOT EXISTS(
    SELECT 1 FROM game_guild_ranks gr WHERE gr.guild_id=rule.guild AND gr.level=rule.rank AND gr.name_key=rule.name
   ))
 ) THEN
  RAISE EXCEPTION 'House guild ACL requires current owner receipt and qualified selectors' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_guild_acl_proven AFTER INSERT OR UPDATE ON game_house_guild_acl
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_guild_acl_proven();

CREATE OR REPLACE FUNCTION game_house_access_grant(p_world UUID,p_house TEXT,p_character UUID)
RETURNS TEXT LANGUAGE sql STABLE AS $$
 WITH granted AS (
  SELECT list_key FROM game_house_acl_entries
   WHERE world_id=p_world AND house_key=p_house AND character_id=p_character
  UNION ALL
  SELECT a.list_key FROM game_house_guild_acl a
   JOIN game_house_properties p USING(world_id,house_key)
   JOIN game_guild_members m ON m.character_id=p_character
   JOIN game_guilds g ON g.guild_id=m.guild_id AND g.world_id=p_world AND g.state IN(1,2)
   JOIN game_guild_ranks gr ON gr.guild_id=m.guild_id AND gr.level=m.rank_level
   WHERE a.world_id=p_world AND a.house_key=p_house AND a.ownership_generation=p.ownership_generation
    AND EXISTS(
     SELECT 1 FROM jsonb_to_recordset(a.rules) AS rule(guild UUID,rank SMALLINT,name TEXT)
     WHERE rule.guild=m.guild_id AND (rule.rank IS NULL OR (rule.rank=gr.level AND rule.name=gr.name_key))
    )
 )
 SELECT CASE WHEN bool_or(list_key='subowner') THEN 'SUBOWNER'
             WHEN bool_or(list_key='guest') THEN 'GUEST' END FROM granted
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_guild_acl_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_access_grant(UUID,TEXT,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_guild_acl_proven(),game_house_access_grant(UUID,TEXT,UUID) FROM PUBLIC;
