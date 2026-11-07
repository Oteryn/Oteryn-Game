-- Keep QBASE1205 byte-for-byte; repair only the ambiguous PL/pgSQL local.
-- Father remains a single atomic eleven-track write with its existing proofs.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT;
BEGIN
    body:=pg_get_functiondef('game_father_values_proven()'::regprocedure);
    FOR original,replacement IN VALUES
        ('expected_rule TEXT; amount BIGINT;','expected_rule TEXT; expected_xp BIGINT;'),
        ('amount:=8000;','expected_xp:=8000;'),
        ('amount:=2500;','expected_xp:=2500;'),
        ('x.amount=amount','x.amount=expected_xp')
    LOOP
        IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
            RAISE EXCEPTION 'unexpected QBASE Father XP guard';
        END IF;
        body:=replace(body,original,replacement);
    END LOOP;
    EXECUTE body;
END $migration$;
REVOKE ALL ON FUNCTION game_father_values_proven() FROM PUBLIC;
