-- SOULPIT-1: normalized Character species completions, consumed by the existing XP owner.
-- BOSSENC-1 owns the completion proof and eventual fenced writer. Until its shared activity
-- completion contract lands, no runtime or control role may insert a mastery or award titles.
CREATE TABLE game_character_animus_masteries (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 creature_key TEXT NOT NULL CHECK (creature_key ~ '^oteryn:creature\.[a-z0-9_]+$' AND octet_length(creature_key)<=256),
 completion_occurrence_id UUID NOT NULL CHECK (game_character_is_uuid_v7(completion_occurrence_id)),
 completed_character_revision NUMERIC(20,0) NOT NULL CHECK (completed_character_revision BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(character_id,creature_key),
 UNIQUE(character_id,completion_occurrence_id)
);
CREATE TRIGGER game_animus_masteries_immutable BEFORE UPDATE OR DELETE ON game_character_animus_masteries
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_animus_masteries_no_truncate BEFORE TRUNCATE ON game_character_animus_masteries
 EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_character_animus_masteries FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_character_animus_masteries TO oteryn_game_runtime,oteryn_game_control;
ALTER TABLE game_party_kill_xp_members
 ADD COLUMN animus_basis_points SMALLINT NOT NULL DEFAULT 0
 CHECK (animus_basis_points=0 OR (animus_basis_points BETWEEN 200 AND 400 AND animus_basis_points % 10=0));
