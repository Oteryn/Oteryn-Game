-- Winter Update 2026 CT measurement (Content session, 2026-11-16):
-- evidence/2026-11-16-testserver/fragment-workshop.txt selects 12.5M gold +15 fragments.
-- Crystal 201189f8, player_wheel.cpp:1592 still charges 12M.
-- Extend the current definitions, including the carried/stash/output and 2771 reshape guards.
DO $migration$
DECLARE body TEXT; old_cost TEXT := 'WHEN 2 THEN 12000000'; new_cost TEXT := 'WHEN 2 THEN 12500000';
BEGIN
 SELECT pg_get_functiondef('game_atelier_assets_guard()'::regprocedure) INTO STRICT body;
 IF position(new_cost IN body)=0 THEN
  IF position(old_cost IN body)=0 THEN RAISE EXCEPTION 'missing Atelier supreme grade two cost'; END IF;
  EXECUTE replace(body,old_cost,new_cost);
 END IF;

 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 IF position('12500000' IN body)=0 THEN
  IF position('12000000' IN body)=0 THEN RAISE EXCEPTION 'missing Atelier supreme grade two fee bound'; END IF;
  body:=replace(body,'12000000','12500000');
 END IF;
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause '||body||' NOT VALID';
END $migration$;

REVOKE ALL ON FUNCTION game_atelier_assets_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_atelier_assets_guard() TO oteryn_game_runtime;
