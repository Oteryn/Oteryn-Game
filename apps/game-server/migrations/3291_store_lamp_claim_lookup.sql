-- Cause 470 from Inbox custody can commit only with a genuine Hireling lamp placement.
-- Keep the Store claim proof lookup bounded by the exact purchased item.
CREATE INDEX game_store_claim_lamp_use
    ON game_item_use_records(source_item_instance_id, occurred_at, transaction_id)
    WHERE source_kind=2 AND cause_kind=470;

-- Retain the decision generation in the same transaction as the lamp burn.
-- Placements committed before this column existed keep their historical fallback.
ALTER TABLE game_hireling_lamp_transitions ADD COLUMN placement_recovery_generation NUMERIC(20,0)
    CHECK (kind=1 AND placement_recovery_generation BETWEEN 1 AND 18446744073709551615
           OR placement_recovery_generation IS NULL);
