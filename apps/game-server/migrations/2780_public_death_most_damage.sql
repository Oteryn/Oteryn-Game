-- WU26 (capability 488): the most-damage dealer of a death, captured by the fenced death
-- transaction as a reference to one of its own historical killer rows, so the displayed
-- name is the name frozen at death. NULL is unknown: deaths captured before this
-- migration are never reinterpreted, and a death without damage evidence stays unknown.
ALTER TABLE game_public_death_classifications ADD COLUMN most_damage_ordinal SMALLINT
 CHECK(most_damage_ordinal BETWEEN 0 AND 80);
ALTER TABLE game_public_death_classifications ADD CONSTRAINT game_public_death_most_damage_killer
 FOREIGN KEY(death_occurrence_id,most_damage_ordinal)
 REFERENCES game_public_death_killers(death_occurrence_id,ordinal);
