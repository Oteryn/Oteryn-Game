-- Explicit source-field World config. Missing policy remains unavailable;
-- admission readiness's policy token never implies a guessed PvP mode.
CREATE TABLE game_control_field_policy_grants (
 control_role TEXT NOT NULL CHECK(octet_length(control_role) BETWEEN 1 AND 63),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 world_policy_revision TEXT NOT NULL CHECK(octet_length(world_policy_revision) BETWEEN 1 AND 4096),
 PRIMARY KEY(control_role,world_id,world_policy_revision)
);
CREATE TABLE game_spell_field_world_policies (
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 world_policy_revision TEXT NOT NULL CHECK(octet_length(world_policy_revision) BETWEEN 1 AND 4096),
 world_type SMALLINT NOT NULL CHECK(world_type IN(1,2,3)),
 protection_level BIGINT NOT NULL CHECK(protection_level BETWEEN 0 AND 4294967295),
 in_fight_ms BIGINT NOT NULL CHECK(in_fight_ms BETWEEN 0 AND 4294967295),
 source_pin TEXT NOT NULL CHECK(source_pin='99902524e052f37574194466c2949c576e4ab269'),
 control_role TEXT NOT NULL,
 decision_identity TEXT NOT NULL CHECK(octet_length(decision_identity) BETWEEN 1 AND 4096),
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(world_id,world_policy_revision)
);
CREATE FUNCTION game_spell_field_policy_control_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 IF TG_OP <> 'INSERT' THEN
  RAISE EXCEPTION 'field World policy is immutable; publish a new policy revision' USING ERRCODE='42501';
 END IF;
 IF NEW.control_role<>session_user OR NOT pg_has_role(session_user,'oteryn_game_control','MEMBER')
    OR NOT EXISTS(SELECT 1 FROM game_control_field_policy_grants g WHERE g.control_role=session_user AND g.world_id=NEW.world_id AND g.world_policy_revision=NEW.world_policy_revision) THEN
  RAISE EXCEPTION 'field World policy is not granted to this control principal' USING ERRCODE='42501';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_spell_field_policy_control_guard BEFORE INSERT OR UPDATE OR DELETE ON game_spell_field_world_policies
FOR EACH ROW EXECUTE FUNCTION game_spell_field_policy_control_guard();
REVOKE ALL ON TABLE game_control_field_policy_grants,game_spell_field_world_policies FROM PUBLIC;
GRANT SELECT ON game_spell_field_world_policies TO oteryn_game_runtime,oteryn_game_control;
GRANT INSERT ON game_spell_field_world_policies TO oteryn_game_control;
-- Only the database/control bootstrap owner can issue exact World/revision
-- grants. This migration deliberately creates no grant or configured policy.
