-- Coin cosmetics follow the account. Token cosmetics retain character/world custody.
CREATE INDEX store_delivery_account_cosmetic ON store_delivery_receipts(account_id, entitlement_key)
    WHERE entitlement_key LIKE 'store-cosmetic:%';
CREATE INDEX store_pending_cosmetic_offer ON game_character_store_cosmetics(offer_key, purchase_id)
    WHERE operation_id IS NULL;
