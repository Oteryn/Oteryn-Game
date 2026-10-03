-- Candidate ACTOR_SPELL_PARAMETERS_V2 private result outbox. No protocol IDs.
-- An immutable result is replay data, never current actor or House authority.
ALTER TABLE game_house_editors ADD COLUMN created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id();
CREATE FUNCTION game_house_editor_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW; END $$;
CREATE TRIGGER game_house_editor_stamp BEFORE INSERT ON game_house_editors FOR EACH ROW EXECUTE FUNCTION game_house_editor_stamp();
CREATE TABLE game_spell_parameter_results (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
    catalog_digest BYTEA NOT NULL CHECK(octet_length(catalog_digest)=32),
    spell_key TEXT NOT NULL CHECK(octet_length(spell_key) BETWEEN 1 AND 512),
    spell_revision TEXT NOT NULL CHECK(octet_length(spell_revision) BETWEEN 1 AND 512),
    intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 192),
    intent_digest BYTEA NOT NULL CHECK(intent_digest=sha256(intent)),
    result BYTEA NOT NULL CHECK(octet_length(result) BETWEEN 1 AND 8192),
    result_digest BYTEA NOT NULL CHECK(result_digest=sha256(result)),
    cast_succeeded BOOLEAN NOT NULL,
    cost_transaction_id UUID REFERENCES game_spell_item_receipts(transaction_id),
    editor_id UUID REFERENCES game_house_editors(editor_id),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(game_session_id,command_id),
    CHECK(NOT cast_succeeded OR cost_transaction_id IS NOT NULL),
    CHECK(editor_id IS NULL OR cast_succeeded)
);
CREATE FUNCTION game_spell_parameter_result_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW; END $$;
CREATE TRIGGER game_spell_parameter_result_stamp BEFORE INSERT ON game_spell_parameter_results FOR EACH ROW EXECUTE FUNCTION game_spell_parameter_result_stamp();
CREATE FUNCTION game_spell_parameter_result_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM game_character_roots c WHERE c.character_id=NEW.character_id AND c.world_id=NEW.world_id AND c.lifecycle=1) THEN
        RAISE EXCEPTION 'parameter result Character/World mismatch' USING ERRCODE='23514';
    END IF;
    IF NEW.cost_transaction_id IS NOT NULL AND NOT EXISTS(
        SELECT 1 FROM game_spell_item_receipts r WHERE r.transaction_id=NEW.cost_transaction_id
        AND r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
        AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id
        AND r.ownership_generation=NEW.scope_generation AND r.catalog_digest=NEW.catalog_digest
        AND r.spell_production_key=NEW.spell_key AND r.spell_revision=NEW.spell_revision
        AND r.created_xact_id=NEW.created_xact_id AND NEW.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'parameter success requires same-transaction exact source cost' USING ERRCODE='23514';
    END IF;
    IF NEW.editor_id IS NOT NULL AND NOT EXISTS(
        SELECT 1 FROM game_house_editors e WHERE e.editor_id=NEW.editor_id
        AND e.game_session_id=NEW.game_session_id AND e.opening_command_id=NEW.command_id
        AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id
        AND e.scope_generation=NEW.scope_generation AND e.content_digest=NEW.catalog_digest
        AND e.created_xact_id=NEW.created_xact_id AND NEW.created_xact_id=pg_current_xact_id()
        AND NOT e.consumed AND e.expires_at>clock_timestamp()) THEN
        RAISE EXCEPTION 'parameter result requires exact live editor token' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_parameter_result_proven AFTER INSERT ON game_spell_parameter_results DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_parameter_result_proven();
CREATE FUNCTION game_spell_parameter_result_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'parameter result is immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_spell_parameter_result_immutable BEFORE UPDATE OR DELETE ON game_spell_parameter_results FOR EACH ROW EXECUTE FUNCTION game_spell_parameter_result_immutable();
REVOKE ALL ON game_spell_parameter_results FROM PUBLIC;
GRANT SELECT,INSERT ON game_spell_parameter_results TO oteryn_game_runtime;
DO $$ DECLARE name TEXT; BEGIN
    FOREACH name IN ARRAY ARRAY['game_house_editor_stamp','game_spell_parameter_result_stamp','game_spell_parameter_result_proven','game_spell_parameter_result_immutable'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp',name,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',name);
    END LOOP;
END $$;
