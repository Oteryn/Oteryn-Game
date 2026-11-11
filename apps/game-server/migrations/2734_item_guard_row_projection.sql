-- Reuse only the trigger invocation's local row images. Every installed
-- authority query, proof branch, mutation, exception and lock remains in place.
DO $migration$
DECLARE
    body TEXT;
    begin_at INTEGER;
    old_call TEXT := 'to_jsonb(OLD)';
    new_call TEXT := 'to_jsonb(NEW)';
    refresh_anchor TEXT := E'    NEW.state_revision := OLD.state_revision + 1;\n    IF TG_OP = ''UPDATE''';
    entry TEXT := $entry$DECLARE
    v_instance_guard_old_json JSONB;
    v_instance_guard_new_json JSONB;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        v_instance_guard_old_json := to_jsonb(OLD);
        v_instance_guard_new_json := to_jsonb(NEW);
    END IF;$entry$;
    refresh TEXT := $refresh$    NEW.state_revision := OLD.state_revision + 1;
    IF TG_OP = 'UPDATE' THEN
        v_instance_guard_new_json := to_jsonb(NEW);
    END IF;
    IF TG_OP = 'UPDATE'$refresh$;
BEGIN
    body := pg_get_functiondef('game_item_instance_guard()'::regprocedure);
    IF (length(body)-length(replace(body,old_call,'')))/length(old_call) <> 72
       OR (length(body)-length(replace(body,new_call,'')))/length(new_call) <> 40
       OR (length(body)-length(replace(body,refresh_anchor,'')))/length(refresh_anchor) <> 1
       OR regexp_count(body,E'\\mBEGIN\\M',1,'i') <> 1
       OR regexp_count(body,E'\\mDECLARE\\M',1,'i') <> 0
       OR position('v_instance_guard_old_json' IN body) <> 0
       OR position('v_instance_guard_new_json' IN body) <> 0
       OR regexp_count(body,E'\\mNEW[[:space:]]*\\.[[:alnum:]_]+[[:space:]]*:=',1,'i') <> 11
       OR regexp_count(body,E'\\mNEW[[:space:]]*\\.state_revision[[:space:]]*:=',1,'i') <> 11
       OR body ~* E'\\mNEW[[:space:]]*:='
       OR body ~* E'\\mOLD([[:space:]]*\\.[[:alnum:]_]+)?[[:space:]]*:='
       OR body ~* E'\\m(INTO[[:space:]]+(STRICT[[:space:]]+)?|FOR[[:space:]]+|FOREACH[[:space:]]+)(OLD|NEW)\\M'
       OR body ~* E'\\m(CALL|EXECUTE|FETCH)\\M'
    THEN
        RAISE EXCEPTION 'unexpected installed Item instance guard';
    END IF;

    -- Replace existing expression sites before introducing the materializers.
    body := replace(body,old_call,'v_instance_guard_old_json');
    body := replace(body,new_call,'v_instance_guard_new_json');
    body := replace(body,refresh_anchor,refresh);
    begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    body := overlay(body PLACING entry FROM begin_at FOR 5);
    -- pg_get_functiondef keeps the installed security mode and search_path;
    -- CREATE OR REPLACE keeps the function identity, ownership and grants.
    EXECUTE body;
END $migration$;

-- Walk the same OLD, NEW row images in order without a SQL cursor for the
-- two-element local array. Keep both rechecks and every custody query/lock.
DO $migration$
DECLARE
    body TEXT;
    anchor TEXT := $old_loop$ FOR j IN SELECT x FROM unnest(ARRAY[CASE WHEN TG_OP<>'INSERT' THEN to_jsonb(OLD) END,
 CASE WHEN TG_OP<>'DELETE' THEN to_jsonb(NEW) END]) x WHERE x IS NOT NULL LOOP$old_loop$;
    replacement TEXT := $new_loop$ FOREACH j IN ARRAY ARRAY[CASE WHEN TG_OP<>'INSERT' THEN to_jsonb(OLD) END,
 CASE WHEN TG_OP<>'DELETE' THEN to_jsonb(NEW) END] LOOP
  IF j IS NULL THEN CONTINUE; END IF;$new_loop$;
BEGIN
    body := pg_get_functiondef('game_account_closure_custody_guard()'::regprocedure);
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'unexpected installed account custody guard';
    END IF;
    EXECUTE replace(body,anchor,replacement);
END $migration$;
