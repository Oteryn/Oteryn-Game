-- P01C: permanent Charm Expansion entitlement owned by the purchasing Character.
CREATE TABLE game_character_charm_expansions (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp()
);

CREATE TRIGGER game_character_charm_expansions_no_update_or_delete
    BEFORE UPDATE OR DELETE ON game_character_charm_expansions
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_charm_expansions_no_truncate
    BEFORE TRUNCATE ON game_character_charm_expansions
    EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON game_character_charm_expansions FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_charm_expansions TO oteryn_game_runtime;
GRANT SELECT ON game_character_charm_expansions TO oteryn_game_control;
