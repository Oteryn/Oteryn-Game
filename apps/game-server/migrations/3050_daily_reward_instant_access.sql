CREATE TABLE public.game_character_daily_reward_access (
    character_id uuid PRIMARY KEY REFERENCES public.game_character_roots(character_id) ON DELETE CASCADE,
    tokens bigint NOT NULL CHECK (tokens BETWEEN 0 AND 4294967295)
);

REVOKE ALL ON public.game_character_daily_reward_access FROM PUBLIC;
GRANT SELECT ON public.game_character_daily_reward_access TO oteryn_game_runtime;
GRANT UPDATE (tokens) ON public.game_character_daily_reward_access TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON public.game_character_daily_reward_access TO oteryn_game_control;
