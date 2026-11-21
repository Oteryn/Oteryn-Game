-- M37: receipt v1 kept native quest floors (-z); movement arrivals use legacy z.
-- Preserve immutable v1 receipts and normalize them at the arrival reader.
ALTER TABLE game_quest_npc_teleport_receipts
    ADD COLUMN runtime_coordinates boolean NOT NULL DEFAULT false;
ALTER TABLE game_quest_npc_teleport_receipts
    DROP CONSTRAINT game_quest_npc_teleport_receipts_tile_floor_check;
ALTER TABLE game_quest_npc_teleport_receipts
    ADD CONSTRAINT game_quest_npc_teleport_receipts_tile_floor_check CHECK (
        (runtime_coordinates AND tile_floor BETWEEN 0 AND 15)
        OR (NOT runtime_coordinates AND tile_floor BETWEEN -15 AND 0)
    );
