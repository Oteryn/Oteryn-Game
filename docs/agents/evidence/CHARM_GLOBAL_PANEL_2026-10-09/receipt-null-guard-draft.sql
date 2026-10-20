-- OFFLINE DRAFT: new migration; no repository migration number is allocated.
-- Base: Oteryn/Oteryn-Game@f21a45d124c0586df2f259b74a1923e410e99d74.
-- Keep the applied 0020 migration immutable. Its existing conditional CHECK
-- already validates the ranges and assign shape, but SQL UNKNOWN permits
-- missing unlock values. Add the missing presence requirement only.
-- ADD CONSTRAINT validates existing rows; invalid retained receipts fail this
-- migration closed. Do not use NOT VALID or rewrite immutable receipts here.
ALTER TABLE game_character_charm_receipts
    ADD CONSTRAINT game_character_charm_unlock_fields_present
    CHECK (command_kind <> 1
        OR (stage_before IS NOT NULL
            AND stage_after IS NOT NULL
            AND stage_cost IS NOT NULL));
