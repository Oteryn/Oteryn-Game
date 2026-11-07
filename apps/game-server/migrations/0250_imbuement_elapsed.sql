-- IMBUE-TIME-1: item-only checkpoints. Each receipt binds the exact applied lifetimes
-- and their before/after values; after_ms=0 is the immutable expiry audit.
CREATE TABLE game_item_imbuement_checkpoints (
    transaction_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    game_session_id UUID NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    changes JSONB NOT NULL CHECK (jsonb_typeof(changes)='array'
        AND jsonb_array_length(changes) BETWEEN 1 AND 30),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);

-- Preserve Apply/Clear and any independently added lifecycle arms. A DELETE guard
-- must return OLD: RETURN NEW suppresses deletion in a BEFORE DELETE trigger.
DO $migration$
DECLARE body TEXT; begin_at INTEGER;
BEGIN
    body:=pg_get_functiondef('game_item_imbuement_row_guard()'::regprocedure);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected imbuement guard'; END IF;
    EXECUTE overlay(body placing E'BEGIN\n'||$arm$
    IF TG_OP='DELETE' AND OLD.remaining_ms=0 AND EXISTS (
        SELECT 1 FROM game_item_imbuement_checkpoints c,
            LATERAL jsonb_array_elements(c.changes) d
        WHERE c.created_xact_id=pg_current_xact_id()
            AND (d->>'item')::uuid=OLD.item_instance_id
            AND (d->>'occurrence')::uuid=OLD.applying_occurrence_id
            AND (d->>'after_ms')::bigint=0
    ) THEN RETURN OLD; END IF;
$arm$ from begin_at for 5);
END $migration$;

CREATE FUNCTION game_item_imbuement_checkpoint_write() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d JSONB; b game_item_imbuements%ROWTYPE; item UUID;
BEGIN
    NEW.created_xact_id:=pg_current_xact_id();
    IF (SELECT count(DISTINCT elem.value->>'item') FROM jsonb_array_elements(NEW.changes) elem(value))>10
        OR (SELECT count(DISTINCT elem.value->>'occurrence') FROM jsonb_array_elements(NEW.changes) elem(value))
            <>jsonb_array_length(NEW.changes) THEN
        RAISE EXCEPTION 'imbuement checkpoint bound or duplicate lifetime' USING ERRCODE='23514';
    END IF;
    -- The Rust owner already holds the fenced Character root. Stable item/slot order
    -- matches equipment and shrine writers; at most ten equipped items / thirty rows.
    FOR d IN SELECT value FROM jsonb_array_elements(NEW.changes)
        ORDER BY value->>'item',value->>'occurrence' LOOP
        IF jsonb_typeof(d) IS DISTINCT FROM 'object'
            OR jsonb_typeof(d->'before_ms') IS DISTINCT FROM 'number'
            OR jsonb_typeof(d->'after_ms') IS DISTINCT FROM 'number'
            OR jsonb_typeof(d->'item') IS DISTINCT FROM 'string'
            OR jsonb_typeof(d->'occurrence') IS DISTINCT FROM 'string' THEN
            RAISE EXCEPTION 'malformed imbuement checkpoint' USING ERRCODE='23514';
        END IF;
        item:=(d->>'item')::uuid;
        PERFORM 1 FROM (SELECT item_instance_id,character_id,world_id FROM game_character_equipment_slots UNION ALL SELECT item_instance_id,character_id,world_id FROM game_item_container_slots) e JOIN game_item_instances i
            USING(item_instance_id) WHERE e.character_id=NEW.character_id
            AND e.world_id=NEW.world_id AND i.world_id=NEW.world_id
            AND i.item_instance_id=item AND i.lifecycle=1 FOR UPDATE OF i;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'checkpoint item is not equipped' USING ERRCODE='23514';
        END IF;
        SELECT * INTO b FROM game_item_imbuements
            WHERE item_instance_id=item AND applying_occurrence_id=(d->>'occurrence')::uuid
            FOR UPDATE;
        IF NOT FOUND OR b.remaining_ms<>(d->>'before_ms')::bigint
            OR NOT (d ?& ARRAY['before_ms','after_ms','item','occurrence'])
            OR (d->>'after_ms')::bigint<0
            OR (d->>'after_ms')::bigint>b.remaining_ms
            OR ((d->>'after_ms')::bigint=b.remaining_ms AND b.remaining_ms<>0) THEN
            RAISE EXCEPTION 'checkpoint lifetime/budget changed' USING ERRCODE='23514';
        END IF;
    END LOOP;
    RETURN NEW;
END $$;

CREATE FUNCTION game_item_imbuement_checkpoint_apply() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d JSONB;
BEGIN
    FOR d IN SELECT value FROM jsonb_array_elements(NEW.changes)
        ORDER BY value->>'item',value->>'occurrence' LOOP
        UPDATE game_item_imbuements SET remaining_ms=(d->>'after_ms')::bigint
            WHERE item_instance_id=(d->>'item')::uuid
            AND applying_occurrence_id=(d->>'occurrence')::uuid;
        DELETE FROM game_item_imbuements WHERE item_instance_id=(d->>'item')::uuid
            AND applying_occurrence_id=(d->>'occurrence')::uuid AND remaining_ms=0;
    END LOOP;
    RETURN NULL;
END $$;
CREATE TRIGGER game_item_imbuement_checkpoint_write BEFORE INSERT
    ON game_item_imbuement_checkpoints FOR EACH ROW
    EXECUTE FUNCTION game_item_imbuement_checkpoint_write();
CREATE TRIGGER game_item_imbuement_checkpoint_apply AFTER INSERT
    ON game_item_imbuement_checkpoints FOR EACH ROW
    EXECUTE FUNCTION game_item_imbuement_checkpoint_apply();
CREATE TRIGGER game_item_imbuement_checkpoints_immutable BEFORE UPDATE OR DELETE
    ON game_item_imbuement_checkpoints FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_imbuement_checkpoints_no_truncate BEFORE TRUNCATE
    ON game_item_imbuement_checkpoints EXECUTE FUNCTION game_item_reject_truncate();
DO $$
DECLARE name TEXT;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_item_imbuement_checkpoint_write()',
        'game_item_imbuement_checkpoint_apply()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',name,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
    END LOOP;
END $$;
REVOKE ALL ON game_item_imbuement_checkpoints FROM PUBLIC;
GRANT SELECT ON game_item_imbuement_checkpoints TO oteryn_game_runtime,oteryn_game_control;
GRANT INSERT(transaction_id,game_session_id,character_id,world_id,changes)
    ON game_item_imbuement_checkpoints TO oteryn_game_runtime;
