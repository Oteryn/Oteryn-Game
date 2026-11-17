-- T087: optional named spell modifiers, atomically sampled with existing combat facts.
-- NULL identifies an older observation that did not capture these owners.
ALTER TABLE game_character_cyclopedia_combat
 ADD COLUMN offence_augments BYTEA
 CHECK (offence_augments IS NULL OR octet_length(offence_augments) BETWEEN 5 AND 262144),
 ADD COLUMN offence_augments_revision BIGINT CHECK (offence_augments_revision > 0);
