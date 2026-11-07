CREATE FUNCTION pg_temp.srv7_fee_check(p_row JSONB,p_expression TEXT) RETURNS BOOLEAN LANGUAGE plpgsql AS $$
DECLARE outcome BOOLEAN;
BEGIN
 EXECUTE 'SELECT ('||p_expression||') FROM jsonb_populate_record(NULL::game_item_fee_burns,$1)' INTO outcome USING p_row;
 RETURN outcome IS DISTINCT FROM FALSE;
END $$;
DO $$
DECLARE expression TEXT; item JSONB;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expression FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 IF NOT pg_temp.srv7_fee_check('{"cause_kind":473,"charm_key":null,"purchase_service_key":"oteryn:service.horse-station.rental","fee_gold_units":500}',expression) THEN
 RAISE EXCEPTION 'actual catalogue rejects exact typed HorseRental500';END IF;
 FOR item IN SELECT value FROM jsonb_array_elements('[
 {"cause_kind":473,"charm_key":null,"purchase_service_key":null,"fee_gold_units":500},
 {"cause_kind":473,"charm_key":null,"purchase_service_key":"oteryn:service.horse-station.rental","fee_gold_units":499},
 {"cause_kind":473,"charm_key":null,"purchase_service_key":"oteryn:service.horse-station.rental","fee_gold_units":501},
 {"cause_kind":473,"charm_key":"arbitrary","purchase_service_key":"oteryn:service.horse-station.rental","fee_gold_units":500},
 {"cause_kind":473,"charm_key":null,"purchase_service_key":"oteryn:service.horse-station.other","fee_gold_units":500}]'::jsonb) LOOP
 IF pg_temp.srv7_fee_check(item,expression) THEN RAISE EXCEPTION 'malformed typed rental purchase accepted';END IF;
 END LOOP;
 RAISE NOTICE 'PASS exact rental473 shape and malformed cases';
END $$;
DO $$
DECLARE expression TEXT; item JSONB;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expression FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 IF NOT pg_temp.srv7_fee_check('{"cause_kind":474,"charm_key":null,"purchase_service_key":"oteryn:service.npc-cosmetic.king_tibianus.golden-buy-armor","fee_gold_units":500000000}',expression) THEN
 RAISE EXCEPTION 'actual catalogue rejects exact declared NPCsource474 price';END IF;
 FOR item IN SELECT value FROM jsonb_array_elements('[
 {"cause_kind":474,"charm_key":null,"purchase_service_key":null,"fee_gold_units":500000000},
 {"cause_kind":474,"charm_key":null,"purchase_service_key":"oteryn:service.npc-cosmetic.king_tibianus.golden-buy-armor","fee_gold_units":1},
 {"cause_kind":474,"charm_key":null,"purchase_service_key":"oteryn:service.npc-cosmetic.unknown.reward","fee_gold_units":500000000},
 {"cause_kind":474,"charm_key":"arbitrary","purchase_service_key":"oteryn:service.npc-cosmetic.king_tibianus.golden-buy-armor","fee_gold_units":500000000}]'::jsonb) LOOP
 IF pg_temp.srv7_fee_check(item,expression) THEN RAISE EXCEPTION 'malformed typed NPC purchase accepted';END IF;
 END LOOP;
 RAISE NOTICE 'PASS exact declared NPC474 shape and malformed cases';
END $$;
DO $$
DECLARE expression TEXT; previous TEXT; cause INTEGER; service TEXT; price BIGINT; row_data JSONB;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expression FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 SELECT old.expression INTO STRICT previous FROM srv7_previous_purchase_expression old;
 IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause' AND convalidated) THEN RAISE EXCEPTION 'new purchase constraint must be validated';END IF;
 FOR cause IN 1..472 LOOP
  FOREACH service IN ARRAY ARRAY[NULL::TEXT,'oteryn:service.hireling.food.day','oteryn:service.horse-station.rental'] LOOP
   FOREACH price IN ARRAY ARRAY[0::BIGINT,500,15000,90000,500000000] LOOP
    row_data:=jsonb_build_object('cause_kind',cause,'charm_key',NULL,'purchase_service_key',service,'fee_gold_units',price);
    IF pg_temp.srv7_fee_check(row_data,expression) IS DISTINCT FROM pg_temp.srv7_fee_check(row_data,previous) THEN
     RAISE EXCEPTION 'prior fee purchase-cause semantics changed for %',cause;
    END IF;
   END LOOP;
  END LOOP;
 END LOOP;
 RAISE NOTICE 'PASS all prior causes preserve original catalogue CHECK semantics';
END $$;
