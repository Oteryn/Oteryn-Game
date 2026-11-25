-- Cause 470 from Inbox custody can commit only with a genuine Hireling lamp placement.
-- Keep the Store claim proof lookup bounded by the exact purchased item.
CREATE INDEX game_store_claim_lamp_use
    ON game_item_use_records(source_item_instance_id, occurred_at, transaction_id)
    WHERE source_kind=2 AND cause_kind=470;
