-- M38: reserve the exact free Anna/Targuna receipt in the existing arrival owner.
-- Native offers stay disabled until placement, citizenship and tutorial reset are atomic.
ALTER TABLE game_newhaven_travel_receipts
 DROP CONSTRAINT game_travel_route_shape,
 ADD CONSTRAINT game_travel_route_shape CHECK (
   (route IN ('rookgaard','thais_peninsula','targuna') AND request_binding IS NULL AND fee_transaction_id IS NULL AND fare_gold_units IS NULL AND base_fare_gold_units IS NULL AND postman_discount_applied IS NULL)
   OR (route ~ '^oteryn:service[.]travel[.][a-z0-9_]{1,80}/[a-z0-9_]{1,80}$'
       AND request_binding IS NOT NULL AND fare_gold_units IS NOT NULL AND fare_gold_units BETWEEN 0 AND 1000000
       AND base_fare_gold_units IS NOT NULL AND base_fare_gold_units BETWEEN 0 AND 1000000
       AND postman_discount_applied IS NOT NULL
       AND fare_gold_units=GREATEST(0,base_fare_gold_units-CASE WHEN postman_discount_applied THEN 10 ELSE 0 END)
       AND ((fare_gold_units=0 AND fee_transaction_id IS NULL) OR (fare_gold_units>0 AND fee_transaction_id IS NOT NULL))));
