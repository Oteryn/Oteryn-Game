-- Tibia news 8996 (2026-10-06): Market placement fees are 2.5%, capped at
-- 2,500,000 gold. Existing offers retain the fee recorded at placement.

ALTER TABLE game_market_settlements
    DROP CONSTRAINT game_market_settlements_fee_check,
    ADD CONSTRAINT game_market_settlements_fee_check
        CHECK (fee BETWEEN 0 AND 2500000);

DO $$
DECLARE body TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body
 FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass
   AND conname='game_item_fee_burns_purchase_cause';
 ALTER TABLE game_item_fee_burns
     DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
     ||substr(body,7)
     ||') OR (cause_kind=6 AND charm_key IS NULL AND purchase_service_key=''oteryn:market.offer'' AND fee_gold_units BETWEEN 20 AND 2500000 AND line_count=0))';
END $$;

DO $$
DECLARE body TEXT; updated TEXT;
BEGIN
 SELECT pg_get_functiondef('game_market_write(uuid,uuid,uuid,numeric,numeric,numeric,uuid,smallint,uuid,text,text,smallint,integer,bigint,boolean,integer,bytea,bytea)'::regprocedure)
 INTO STRICT body;
 updated:=replace(body,
     'fee:=least(1000000,greatest(20,total/50))',
     'fee:=least(2500000,greatest(20,(total*25)/1000))');
 IF updated=body THEN
  RAISE EXCEPTION 'expected Market fee formula was not found';
 END IF;
 EXECUTE updated;
END $$;
