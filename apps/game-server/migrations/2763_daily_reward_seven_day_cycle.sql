-- WU26-07 seven-day daily cycle (owner decision 2026-11-11; protocol-oteryn
-- daily-reward-v2.md). The claim writer now stores lane day 1 after a cycle day 7
-- claim, where the DAILY30 lane stored 8. The projection guard admits that wrap
-- for a reward version 2 claim on day 7 and keeps every previous rule, so a
-- state written before this migration (lane day 8 after day 7, 1 after day 30,
-- the day after otherwise) still proves against its claim. Claims, their lines
-- and the stored lane days are not changed.
--
-- Rollback: applied migrations are immutable. A successor restores the previous
-- predicate only after the claim writer stops storing the seven-day wrap.
DO $migration$
DECLARE
 body TEXT;
 needle CONSTANT TEXT:='OR state.lane_day <> (CASE WHEN claim.lane_day = CASE WHEN claim.reward_version=2 THEN 30 ELSE 7 END THEN 1 ELSE claim.lane_day + 1 END) THEN';
BEGIN
 body:=pg_get_functiondef('game_daily_reward_projection_guard()'::regprocedure);
 IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
  RAISE EXCEPTION 'unexpected daily reward projection lane rule';
 END IF;
 EXECUTE replace(body,needle,
  'OR (state.lane_day <> (CASE WHEN claim.lane_day = CASE WHEN claim.reward_version=2 THEN 30 ELSE 7 END THEN 1 ELSE claim.lane_day + 1 END)'
  ||' AND NOT (claim.reward_version=2 AND claim.lane_day=7 AND state.lane_day=1)) THEN');
END $migration$;
