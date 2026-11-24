CREATE TABLE game_managed_container_move_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    preview_id NUMERIC(20,0) NOT NULL CHECK(preview_id BETWEEN 1 AND 18446744073709551615),
    moved SMALLINT NOT NULL CHECK(moved BETWEEN 1 AND 500),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(game_session_id,command_id)
);
CREATE INDEX game_managed_container_move_character ON game_managed_container_move_receipts(character_id);
REVOKE ALL ON game_managed_container_move_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_managed_container_move_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_managed_container_move_receipts TO oteryn_game_control;
CREATE TRIGGER game_managed_container_move_immutable
    BEFORE UPDATE OR DELETE ON game_managed_container_move_receipts
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_managed_container_move_no_truncate
    BEFORE TRUNCATE ON game_managed_container_move_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_managed_container_move_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS (
        SELECT 1 FROM game_item_loop_move_receipts r
        WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
          AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id
          AND r.created_xact_id=NEW.created_xact_id AND r.quick_loot_ordinal BETWEEN 1 AND NEW.moved
        HAVING count(*)=NEW.moved
    ) THEN
        RAISE EXCEPTION 'managed move receipt requires all moved stacks' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_managed_container_move_proven AFTER INSERT ON game_managed_container_move_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_managed_container_move_proven();
REVOKE ALL ON FUNCTION game_managed_container_move_proven() FROM PUBLIC;

-- Extend the shared move batch proof, retaining the existing quick loot and
-- depot-search arms. Only a current receipt can authorize carried move children.
DO $migration$
DECLARE body TEXT; at_begin INT;
BEGIN
    EXECUTE format('ALTER FUNCTION game_managed_container_move_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    body:=pg_get_functiondef('game_quick_loot_child_proven()'::regprocedure);
    at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF at_begin=0 THEN
        RAISE EXCEPTION 'missing item move batch proof anchor';
    END IF;
    EXECUTE overlay(body placing $arm$BEGIN
    IF NEW.quick_loot_ordinal>0 AND EXISTS (
        SELECT 1 FROM game_managed_container_move_receipts m
        WHERE (m.game_session_id,m.command_id,m.character_id,m.world_id,m.created_xact_id)=
              (NEW.game_session_id,NEW.command_id,NEW.character_id,NEW.world_id,NEW.created_xact_id)
          AND NEW.quick_loot_ordinal<=m.moved
    ) THEN
        IF NEW.operation NOT IN (1,2) OR (
            SELECT count(*) FROM game_item_loop_move_lines l
            WHERE l.transaction_id=NEW.transaction_id AND l.from_kind IN (1,2)
        )<>1 OR EXISTS (
            SELECT 1 FROM game_item_loop_move_lines l WHERE l.transaction_id=NEW.transaction_id
              AND NOT ((l.from_kind IN (1,2) AND l.to_kind IN (0,1,2))
                OR (NEW.operation=2 AND l.from_kind=9 AND l.to_kind=9))
        ) THEN
            RAISE EXCEPTION 'managed move must conserve carried stacks' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
$arm$ FROM at_begin FOR 5);
END $migration$;
