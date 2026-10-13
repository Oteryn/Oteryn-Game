-- CHAR-VITALS-1: the saved health and mana of a Character, restored on its next admission
-- (owner decision 2026-10-13; `reviews/OTERYN_GAME_CHAR_VITALS0_DURABLE_VITALS_DECISION_2026-10-13.md`,
-- which amends SPELL-D2 and CHAR-POSITION-0 §3.1).
--
-- Scope of this migration only:
--   * two nullable columns on `game_character_last_positions`. The vitals are part of the same
--     Character runtime-state projection as the cell: no CharacterRevision, no cause, no receipt.
--     They are written by the same fenced upsert as the cell, under the same order key, so the
--     existing order trigger (an UPDATE must strictly raise the key and keep the Character and
--     its World), the World guard, the no-TRUNCATE trigger and the grants of migration 0080
--     cover them unchanged;
--   * both columns are NULL or both are set. NULL means "start at the maxima" (SPELL-D2): a
--     respawn placement, an actor saved at its maxima, and every row written before this
--     migration;
--   * a saved actor is alive: health is 1..2^28-1 and mana 0..2^28-1 (`MAX_VITAL_POOL`, the
--     SPELL-D8 wire bound). The next admission caps each at the actor's current maximum.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the two
-- columns and their check. Readers of a row without them start every actor at its maxima, the
-- behaviour before this migration.

ALTER TABLE game_character_last_positions
    ADD COLUMN health INTEGER NULL,
    ADD COLUMN mana INTEGER NULL,
    ADD CONSTRAINT game_character_last_positions_vitals CHECK (
        (health IS NULL AND mana IS NULL)
        OR (health BETWEEN 1 AND 268435455 AND mana BETWEEN 0 AND 268435455)
    );
