-- The real committed movement owner now selects canonical Cake sources and
-- re-proves Character recovery, session/lease, Channel ownership, independently
-- active Content and current map epoch before invoking this closed participant.
-- The receipt remains bound to exact actor, movement revision and reset epoch.
REVOKE ALL ON FUNCTION game_cake_errand_step(UUID,INTEGER,UUID,UUID,NUMERIC,NUMERIC,BYTEA,INTEGER) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_cake_errand_step(UUID,INTEGER,UUID,UUID,NUMERIC,NUMERIC,BYTEA,INTEGER) TO oteryn_game_runtime;
