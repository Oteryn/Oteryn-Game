-- GM-TOOLS-2: a kick is an immutable moderation record owned by the existing
-- signed staff transaction. Runtime consumers can fence the target session from
-- this record without inventing a second moderation authority.
ALTER TABLE game_staff_operations DROP CONSTRAINT game_staff_operations_kind_check;
ALTER TABLE game_staff_operations ADD CONSTRAINT game_staff_operations_kind_check
    CHECK (kind IN ('INSPECT','MUTE','UNMUTE','KICK'));

CREATE TABLE game_staff_kicks (
    environment text NOT NULL,
    operation_id uuid NOT NULL,
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    created_at_ms bigint NOT NULL CHECK (created_at_ms > 0),
    PRIMARY KEY(environment, operation_id),
    FOREIGN KEY(environment,operation_id)
        REFERENCES game_staff_operations(environment,operation_id)
);

CREATE FUNCTION game_staff_kick_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_staff_operations;
BEGIN
    SELECT * INTO STRICT o FROM game_staff_operations
    WHERE environment=NEW.environment AND operation_id=NEW.operation_id;
    IF o.created_xact_id<>pg_current_xact_id() OR o.kind<>'KICK' OR o.result<>1
       OR o.target_character_id<>NEW.character_id OR o.starts_at_ms<>NEW.created_at_ms THEN
        RAISE EXCEPTION 'kick record lacks its atomic staff operation' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_staff_kick_bound BEFORE INSERT ON game_staff_kicks
    FOR EACH ROW EXECUTE FUNCTION game_staff_kick_bound();

CREATE OR REPLACE FUNCTION game_staff_operation_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE l game_staff_control_links; o game_staff_operations; h game_staff_control_heads;
BEGIN
    SELECT * INTO STRICT o FROM game_staff_operations WHERE environment=NEW.environment AND operation_id=NEW.operation_id;
    SELECT * INTO STRICT l FROM game_staff_control_links WHERE environment=o.environment AND control_seq=o.control_seq;
    SELECT * INTO STRICT h FROM game_staff_control_heads WHERE environment=o.environment;
    IF o.created_xact_id<>pg_current_xact_id() OR l.created_xact_id<>pg_current_xact_id() OR l.kind<>'REQUEST' OR l.operation_id<>o.operation_id
       OR (convert_from(l.canonical,'UTF8')::jsonb->'action'->>'kind') IS DISTINCT FROM o.kind
       OR replace(o.target_character_id::text,'-','') IS DISTINCT FROM (convert_from(l.canonical,'UTF8')::jsonb->'action'->>'character_id')
       OR NOT EXISTS(SELECT 1 FROM game_staff_action_outbox a WHERE a.environment=o.environment AND a.operation_id=o.operation_id AND a.created_xact_id=pg_current_xact_id() AND a.starts_at_ms=o.starts_at_ms)
       OR encode(sha256(o.outcome),'hex')<>o.outcome_sha256
       OR h.control_seq<>o.control_seq OR (o.requires_receipt AND h.pending_operation_id IS DISTINCT FROM o.operation_id)
       OR (NOT o.requires_receipt AND h.pending_operation_id IS NOT NULL) THEN
        RAISE EXCEPTION 'staff operation missing atomic evidence' USING ERRCODE='23514';
    END IF;
    IF o.result=1 AND o.kind='MUTE' AND NOT EXISTS(SELECT 1 FROM game_character_sanctions s WHERE s.environment=o.environment AND s.sanction_id=o.operation_id) THEN
        RAISE EXCEPTION 'mute has no effect' USING ERRCODE='23514';
    END IF;
    IF o.result=1 AND o.kind='UNMUTE' AND NOT EXISTS(SELECT 1 FROM game_character_sanctions s WHERE s.environment=o.environment AND s.lift_operation_id=o.operation_id) THEN
        RAISE EXCEPTION 'unmute has no effect' USING ERRCODE='23514';
    END IF;
    IF o.result=1 AND o.kind='KICK' AND NOT EXISTS(SELECT 1 FROM game_staff_kicks k WHERE k.environment=o.environment AND k.operation_id=o.operation_id) THEN
        RAISE EXCEPTION 'kick has no effect' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END; $$;

CREATE TRIGGER game_staff_kick_no_update BEFORE UPDATE OR DELETE ON game_staff_kicks
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
GRANT SELECT, INSERT ON game_staff_kicks TO oteryn_game_runtime;
GRANT SELECT ON game_staff_kicks TO oteryn_game_control;
