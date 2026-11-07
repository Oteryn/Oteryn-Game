-- Conversation lifetimes are transient; their authored delay is part of replayed responses.
ALTER TABLE game_quest_npc_program_receipts
 ADD COLUMN end_delay_ms INTEGER NOT NULL DEFAULT 0
 CHECK(end_delay_ms BETWEEN 0 AND 60000 AND (end_conversation OR end_delay_ms=0));
