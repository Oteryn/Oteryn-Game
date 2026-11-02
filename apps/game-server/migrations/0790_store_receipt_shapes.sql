-- Reconcile the two no-item Store receipt shapes that landed in parallel.
-- 0720 admits Hireling service receipts (no item, purchase_request_id names the
-- reservation) and 0755 admits the Prey list expansion entitlement (no item,
-- entitlement_key). Each migration's CHECK refused the other's receipt, so neither
-- delivery could commit. One CHECK now names exactly the three receipt shapes; each
-- shape keeps its own deferred proof trigger (0345/0720 item mint, 0720 Hireling grant,
-- 0755 Prey expansion guard).
ALTER TABLE store_delivery_receipts
 DROP CONSTRAINT store_delivery_has_native_effect,
 DROP CONSTRAINT store_delivery_payload_kind,
 ADD CONSTRAINT store_delivery_payload_kind CHECK(
   (item_instance_id IS NOT NULL AND entitlement_key IS NULL)
   OR (item_instance_id IS NULL AND entitlement_key = 'prey-list-expansion' AND purchase_request_id IS NULL)
   OR (item_instance_id IS NULL AND entitlement_key IS NULL AND purchase_request_id IS NOT NULL)
 );
