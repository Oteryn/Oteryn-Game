-- P01: serialize permanent Charm Expansion Store purchases before Platform debit.
CREATE TABLE game_charm_expansion_purchases (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    purchase_id UUID NOT NULL UNIQUE REFERENCES store_purchase_requests(purchase_request_id),
    world_id UUID NOT NULL,
    reserved_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp()
);

REVOKE ALL ON game_charm_expansion_purchases FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_charm_expansion_purchases TO oteryn_game_runtime;
GRANT SELECT ON game_charm_expansion_purchases TO oteryn_game_control;
