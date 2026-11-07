-- DEATH-1 (`reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md`
-- §3.1, decision DEATH0-CHARACTER-DEATH-RECEIPT-V1): the grant 0016 missed.
--
-- 0016 revokes EXECUTE on `game_character_is_blessing_set(text[])` from PUBLIC
-- and grants it to no role. The function backs the `blessings_before` and
-- `blessings_after` CHECKs of `game_character_death_receipts`, and PostgreSQL
-- checks EXECUTE on functions a CHECK calls as the inserting role, so the
-- DEATH-1 writer's `oteryn_game_runtime` login cannot insert a death receipt.
-- This grants exactly that, like 0006 grants `game_character_is_uuid_v7(uuid)`.
--
-- No other 0016 gap: its trigger functions are not checked at fire time, they
-- call no revoked helper, and `oteryn_game_control` only reads the 0016 tables,
-- so it evaluates no CHECK there. 0016 itself is unchanged (applied migrations
-- are immutable).

GRANT EXECUTE ON FUNCTION game_character_is_blessing_set(text[]) TO oteryn_game_runtime;
