-- Link a recoverable coffin use to the exact Count's existing boss death receipt.
ALTER TABLE game_pending_vampire_coffin_payments ADD COLUMN death_id UUID;

GRANT UPDATE (death_id) ON game_pending_vampire_coffin_payments TO oteryn_game_runtime;
