-- Candidate WORLD-HOUSE-INSTANCE-1: one durable Instance identity per actual
-- World-global allocated House. This does not allocate a House or assign a node.
CREATE TABLE game_world_house_instances (
    world_id UUID NOT NULL,
    house_key TEXT NOT NULL,
    house_revision TEXT NOT NULL CHECK(octet_length(house_revision) BETWEEN 1 AND 512),
    instance_id UUID NOT NULL CHECK(game_character_is_uuid_v7(instance_id)),
    compatible_content_digest BYTEA NOT NULL CHECK(octet_length(compatible_content_digest)=32),
    qualified_placement_digest BYTEA NOT NULL CHECK(octet_length(qualified_placement_digest)=32),
    binding_receipt_ref UUID NOT NULL CHECK(game_character_is_uuid_v7(binding_receipt_ref)),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(world_id,house_key),
    UNIQUE(world_id,instance_id),
    UNIQUE(binding_receipt_ref),
    FOREIGN KEY(world_id,house_key) REFERENCES game_house_ownership(world_id,house_key)
);
CREATE FUNCTION game_world_house_instance_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'World-House Instance binding is immutable; replacement requires explicit assignment/handoff owner' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_world_house_instance_immutable BEFORE UPDATE OR DELETE ON game_world_house_instances FOR EACH ROW EXECUTE FUNCTION game_world_house_instance_immutable();
CREATE FUNCTION game_world_house_instance_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW; END $$;
CREATE TRIGGER game_world_house_instance_stamp BEFORE INSERT ON game_world_house_instances FOR EACH ROW EXECUTE FUNCTION game_world_house_instance_stamp();
REVOKE ALL ON game_world_house_instances FROM PUBLIC;
GRANT SELECT ON game_world_house_instances TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_world_house_instances TO oteryn_game_control;
DO $$ DECLARE name TEXT; BEGIN
    FOREACH name IN ARRAY ARRAY['game_world_house_instance_immutable','game_world_house_instance_stamp'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp',name,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',name);
    END LOOP;
END $$;
