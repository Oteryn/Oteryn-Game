-- PREM-WIRE-1 (PREMIUM-ACTIVATION-0 §1.2, `OTERYN_GAME_PREMIUM_ACTIVATION0_GAMEPLAY_SWITCH_OVER_DECISION_2026-10-04.md`):
-- the durable Premium activation latch, written by `durability::premium_activation_latch`.
--
--   * one row at most (`id = 1`). The row means Premium has been delivered on this deployment,
--     whatever the current configuration says: the activation record's id, its switch-over
--     instant `S` and when the latch was written, in Unix microseconds;
--   * written only by `INSERT ... (id = 1) ON CONFLICT (id) DO NOTHING RETURNING`, so two nodes
--     with different configured activations cannot both latch. Never updated, deleted or
--     truncated;
--   * the latch insert and every pre-delivery read are fenced by one transaction-scoped advisory
--     lock (`PREMIUM_ACTIVATION_LOCK_KEY`): the insert takes it exclusively, a pre-delivery read
--     shared.
-- No existing table changes.
--
-- Rollback: applied migrations are immutable. Nothing can unlatch Premium on purpose: a deployment
-- that drops the table must also stop granting Premium.

CREATE TABLE game_premium_activation_latch (
    id SMALLINT PRIMARY KEY CHECK (id = 1),
    activation_id TEXT NOT NULL CHECK (activation_id <> ''),
    switch_over_us BIGINT NOT NULL,
    latched_at_us BIGINT NOT NULL
);

CREATE TRIGGER game_premium_activation_latch_immutable BEFORE UPDATE OR DELETE
    ON game_premium_activation_latch FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_premium_activation_latch_no_truncate BEFORE TRUNCATE
    ON game_premium_activation_latch EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON game_premium_activation_latch FROM PUBLIC;
-- Runtime: the gate reads the row and latches it once; it never updates or deletes.
GRANT SELECT, INSERT ON game_premium_activation_latch TO oteryn_game_runtime;
GRANT SELECT ON game_premium_activation_latch TO oteryn_game_control;
