-- NPC-TRADE-3: full-charge purchases use Item's existing implicit full state.
-- A used/timed instance remains ineligible under the original trade guard.
ALTER TABLE game_npc_trades ADD COLUMN full_charges INTEGER
 CHECK (full_charges BETWEEN 1 AND 65535);
ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trade_subtype_exclusive
 CHECK (full_charges IS NULL OR (fluid_subtype IS NULL AND (sell_all OR item_units=1)));
