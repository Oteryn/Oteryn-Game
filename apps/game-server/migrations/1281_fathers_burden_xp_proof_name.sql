-- Disambiguate the local expected XP from the obligation's amount column.
-- Preserve every exact material/cape/progress check in the existing proof.
DO $migration$
DECLARE body TEXT;
BEGIN
 body:=pg_get_functiondef('game_father_values_proven()'::regprocedure);
 IF strpos(body,'amount BIGINT;')=0 OR strpos(body,'x.amount=amount')=0
   OR strpos(body,'amount:=2500;')=0 OR strpos(body,'amount:=8000;')=0
 THEN RAISE EXCEPTION 'unexpected Father XP proof body'; END IF;
 body:=replace(body,'amount BIGINT;','expected_xp BIGINT;');
 body:=replace(body,'amount:=2500;','expected_xp:=2500;');
 body:=replace(body,'amount:=8000;','expected_xp:=8000;');
 EXECUTE replace(body,'x.amount=amount','x.amount=expected_xp');
END $migration$;
