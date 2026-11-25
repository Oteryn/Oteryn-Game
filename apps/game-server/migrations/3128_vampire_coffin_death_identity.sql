-- Link recoverable use to its exact Count and the assignment that can still own it.
ALTER TABLE game_pending_vampire_coffin_payments
    ADD COLUMN death_id UUID,
    ADD COLUMN spawn_scope BYTEA,
    ADD COLUMN spawn_scope_generation NUMERIC(20,0),
    ADD COLUMN spawn_actor_id BIGINT,
    ADD COLUMN spawn_actor_generation NUMERIC(20,0),
    ADD CONSTRAINT game_vampire_coffin_spawn_complete CHECK (
        (spawn_scope IS NULL AND spawn_scope_generation IS NULL
            AND spawn_actor_id IS NULL AND spawn_actor_generation IS NULL)
        OR (death_id IS NOT NULL
            AND spawn_scope IS NOT NULL AND octet_length(spawn_scope) = 33
            AND spawn_scope_generation IS NOT NULL AND spawn_scope_generation > 0
            AND spawn_actor_id IS NOT NULL AND spawn_actor_id BETWEEN 0 AND 4294967295
            AND spawn_actor_generation IS NOT NULL AND spawn_actor_generation > 0)
    );

GRANT UPDATE (death_id, spawn_scope, spawn_scope_generation, spawn_actor_id, spawn_actor_generation)
    ON game_pending_vampire_coffin_payments TO oteryn_game_runtime;
