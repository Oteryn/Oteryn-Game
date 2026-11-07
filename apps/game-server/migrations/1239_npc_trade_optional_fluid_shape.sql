-- Untagged ordinary trades keep their original BUY/SELL conservation rules.
-- Only a non-NULL fluid subtype requires one filled-container purchase.
ALTER TABLE game_npc_trades
    DROP CONSTRAINT game_npc_trades_check1,
    ADD CONSTRAINT game_npc_trades_fluid_shape CHECK (
        fluid_subtype IS NULL OR
        (fluid_subtype BETWEEN 1 AND 255 AND side=1 AND quantity=1 AND item_units=1));
