-- Old receipts lack a committed final-blow kind and credit role.
ALTER TABLE game_public_death_classifications
    ADD COLUMN final_blow_kind SMALLINT CHECK (final_blow_kind BETWEEN 1 AND 3);

CREATE TABLE game_character_player_kill_credits (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    death_occurrence_id UUID NOT NULL REFERENCES game_character_death_receipts(death_occurrence_id),
    committed_at BIGINT NOT NULL,
    final_blow BOOLEAN NOT NULL,
    PRIMARY KEY (character_id, death_occurrence_id)
);
CREATE INDEX game_character_player_kill_credits_recent
    ON game_character_player_kill_credits(character_id, committed_at DESC, death_occurrence_id DESC);
CREATE UNIQUE INDEX game_character_player_kill_credits_occurrence_final
    ON game_character_player_kill_credits(death_occurrence_id) WHERE final_blow;
CREATE INDEX game_character_player_kill_credits_death
    ON game_character_player_kill_credits(death_occurrence_id);
REVOKE ALL ON game_character_player_kill_credits FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_player_kill_credits TO oteryn_game_runtime;
GRANT SELECT ON game_character_player_kill_credits TO oteryn_game_control;
