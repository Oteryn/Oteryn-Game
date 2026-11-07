-- L2-owned extensions to unchanged QENGINE2 participants and state operations.
-- All original proof clauses remain; extend the shared bound and exact set_if only.
DO $migration$
DECLARE body TEXT; original TEXT; replacement TEXT; constraint_name TEXT;
BEGIN
    SELECT conname,pg_get_constraintdef(oid) INTO STRICT constraint_name,body
        FROM pg_constraint WHERE conrelid='game_quest_reward_participants'::regclass
            AND position('cardinality(track_keys)' IN pg_get_constraintdef(oid))>0;
    replacement:=replace(body,'<= 8','<= 16');
    IF replacement=body THEN RAISE EXCEPTION 'quest participant capacity anchor changed'; END IF;
    EXECUTE format('ALTER TABLE game_quest_reward_participants DROP CONSTRAINT %I',constraint_name);
    EXECUTE format('ALTER TABLE game_quest_reward_participants ADD CONSTRAINT %I %s',constraint_name,replacement);
    body:=pg_get_functiondef('game_quest_reward_participants_complete()'::regprocedure);
    IF position(')>8' IN body)=0 THEN RAISE EXCEPTION 'quest participant operation bound changed'; END IF;
    EXECUTE replace(body,')>8',')>16');
    body:=pg_get_functiondef('game_quest_reward_effects_valid(jsonb,text[],bigint[],bigint[])'::regprocedure);
    IF position('NOT BETWEEN 1 AND 8' IN body)=0 THEN RAISE EXCEPTION 'quest operation capacity changed'; END IF;
    body:=replace(body,'NOT BETWEEN 1 AND 8','NOT BETWEEN 1 AND 16');
    original:=$old$IF coalesce((v_effect->>'preserve')::boolean,false) THEN v_expected:=v_before[v_index];$old$;
    replacement:=$new$IF coalesce((v_effect->>'preserve')::boolean,false)
        OR (v_effect ? 'set_if' AND v_before[v_index] IS DISTINCT FROM (v_effect->>'set_if')::bigint)
        THEN v_expected:=v_before[v_index];$new$;
    IF position(original IN body)=0 THEN RAISE EXCEPTION 'quest exact conditional effect anchor changed'; END IF;
    EXECUTE replace(body,original,replacement);
    body:=pg_get_functiondef('game_quest_reward_state_ops_complete()'::regprocedure);
    IF position('v_count>8' IN body)=0 THEN RAISE EXCEPTION 'quest complete operation bound changed'; END IF;
    EXECUTE replace(body,'v_count>8','v_count>16');
END $migration$;
