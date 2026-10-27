-- NEWS-HIST-IMBUE-PRICE-1: widen the existing imbuement receipt and fee owner to
-- the current Global application and cleansing fees. No second receipt owner is introduced.
ALTER TABLE game_item_imbuement_records
  DROP CONSTRAINT game_item_imbuement_records_check;
ALTER TABLE game_item_imbuement_records
  ADD CONSTRAINT game_item_imbuement_records_check CHECK(
   (action=1 AND jsonb_array_length(materials) BETWEEN 1 AND 3
     AND fee_gold_units=(CASE tier WHEN 1 THEN 7500 WHEN 2 THEN 60000 ELSE 250000 END)
     AND output_definition_family IS NULL AND output_production_key IS NULL AND output_revision_ref IS NULL)
   OR (action=2 AND jsonb_array_length(materials)=0 AND fee_gold_units=30000
     AND output_definition_family IS NULL AND output_production_key IS NULL AND output_revision_ref IS NULL)
   OR (action=3 AND tier IN (2,3) AND jsonb_array_length(materials) BETWEEN 1 AND 3
     AND fee_gold_units=(CASE tier WHEN 2 THEN 60000 ELSE 250000 END)
     AND output_definition_family='Item' AND output_production_key IS NOT NULL
     AND output_revision_ref IS NOT NULL));

DO $$
DECLARE body TEXT; widened TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass
    AND conname='game_item_fee_burns_purchase_cause';
 IF body IS NULL THEN RAISE EXCEPTION 'missing purchase fee constraint'; END IF;
 widened:=replace(body, 'ARRAY[5000, 30000, 200000, 15000]',
                        'ARRAY[7500, 60000, 250000, 30000]');
 widened:=replace(widened,
   'ARRAY[(5000)::bigint, (30000)::bigint, (200000)::bigint, (15000)::bigint]',
   'ARRAY[(7500)::bigint, (60000)::bigint, (250000)::bigint, (30000)::bigint]');
 widened:=replace(widened, 'IN (5000, 30000, 200000, 15000)',
                           'IN (7500, 60000, 250000, 30000)');
 IF widened=body THEN RAISE EXCEPTION 'missing legacy imbuement fee values'; END IF;
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause ' || widened;
END;
$$;
