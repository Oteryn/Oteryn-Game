-- M38: bounded latest-completed lookup for an explicit inbound route group.
-- The existing receipt owner stamps created_xact_id; no duplicate return state is stored.
CREATE INDEX game_npc_travel_completed_origin
 ON game_newhaven_travel_receipts(character_id, route, created_xact_id DESC)
 WHERE arrived_at IS NOT NULL;
