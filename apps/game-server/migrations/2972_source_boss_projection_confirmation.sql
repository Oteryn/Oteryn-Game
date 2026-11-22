-- Existing published admissions retain their cooldown. New publications keep
-- recovery authority through installation confirmation. Only a completed
-- native publication can seal the admission and end recovery eligibility.
ALTER TABLE game_source_boss_admissions
    ADD COLUMN projection_pending BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN installation_confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD CHECK (NOT projection_pending OR state=2),
    ADD CHECK (NOT installation_confirmed OR state=2);

DROP INDEX game_source_boss_unpublished;
CREATE INDEX game_source_boss_pending_projection
    ON game_source_boss_admissions(scope_key,scope_generation,admission)
    WHERE state=1 OR (state=2 AND projection_pending);

GRANT UPDATE (projection_pending, installation_confirmed)
    ON game_source_boss_admissions TO oteryn_game_runtime;
