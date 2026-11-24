-- Whole-container stow keeps one existing conservation proof per source stack
-- and one terminal receipt for the confirmed command.
DO $constraint$
DECLARE old_name TEXT;
BEGIN
    SELECT conname INTO STRICT old_name FROM pg_constraint
    WHERE conrelid='game_character_stash_operations'::regclass AND contype='u'
      AND pg_get_constraintdef(oid)='UNIQUE (game_session_id, command_id, batch_ordinal)';
    EXECUTE format('ALTER TABLE game_character_stash_operations DROP CONSTRAINT %I',old_name);
END $constraint$;

ALTER TABLE game_character_stash_operations
    ADD COLUMN container_ordinal SMALLINT NOT NULL DEFAULT 0 CHECK(container_ordinal BETWEEN 0 AND 500),
    ADD UNIQUE(game_session_id, command_id, batch_ordinal, container_ordinal),
    ADD CHECK(batch_ordinal=0 OR container_ordinal=0),
    ADD CHECK(container_ordinal=0 OR (action=2 AND outcome=2 AND house_key IS NULL));

CREATE TABLE game_stash_container_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    preview_id NUMERIC(20,0) NOT NULL CHECK(preview_id BETWEEN 1 AND 18446744073709551615),
    stacks SMALLINT NOT NULL CHECK(stacks BETWEEN 1 AND 500),
    quantity INTEGER NOT NULL CHECK(quantity BETWEEN 1 AND 50000),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(game_session_id, command_id)
);

CREATE INDEX game_stash_container_receipts_character ON game_stash_container_receipts(character_id);
REVOKE ALL ON game_stash_container_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_stash_container_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_stash_container_receipts TO oteryn_game_control;

CREATE TRIGGER game_stash_container_immutable
    BEFORE UPDATE OR DELETE ON game_stash_container_receipts
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_stash_container_no_truncate
    BEFORE TRUNCATE ON game_stash_container_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_stash_container_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE batch game_stash_container_receipts%ROWTYPE;
BEGIN
    IF TG_TABLE_NAME='game_character_stash_operations' THEN
        SELECT * INTO batch FROM game_stash_container_receipts
          WHERE game_session_id=NEW.game_session_id AND command_id=NEW.command_id;
        IF NOT FOUND OR batch.created_xact_id<>pg_current_xact_id()
            OR (batch.character_id,batch.world_id)<>(NEW.character_id,NEW.world_id)
            OR NEW.container_ordinal>batch.stacks THEN
            RAISE EXCEPTION 'container stow child requires its current batch' USING ERRCODE='23514';
        END IF;
    ELSE
        IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS (
            SELECT 1
            FROM game_character_stash_operations r
            JOIN game_character_stash_entries e USING(transaction_id)
            WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
              AND r.created_xact_id=NEW.created_xact_id AND r.container_ordinal>0
              AND (r.character_id,r.world_id)=(NEW.character_id,NEW.world_id)
            HAVING count(*)=NEW.stacks AND sum(e.amount)=NEW.quantity
        ) THEN
            RAISE EXCEPTION 'container stow receipt requires all converted stacks' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_stash_container_child_proven
    AFTER INSERT ON game_character_stash_operations DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NEW.container_ordinal>0) EXECUTE FUNCTION game_stash_container_proven();
CREATE CONSTRAINT TRIGGER game_stash_container_receipt_proven
    AFTER INSERT ON game_stash_container_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_stash_container_proven();

DO $migration$
DECLARE body TEXT; old_condition TEXT;
BEGIN
    EXECUTE format('ALTER FUNCTION game_stash_container_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    body:=pg_get_functiondef('game_stash_admit()'::regprocedure);
    old_condition:=$old$OR NOT game_stash_carried_placement(i.item_instance_id,NEW.parent_item_instance_id,NEW.placement_ordinal,r.character_id,r.world_id)$old$;
    IF position(old_condition IN body)=0 THEN
        RAISE EXCEPTION 'missing stash placement proof anchor';
    END IF;
    EXECUTE replace(body,old_condition,$new$
       OR (r.container_ordinal>0 AND i.quantity<>NEW.quantity)
       OR (NOT game_stash_carried_placement(i.item_instance_id,NEW.parent_item_instance_id,NEW.placement_ordinal,r.character_id,r.world_id)
           AND NOT (r.container_ordinal>0 AND EXISTS(
               SELECT 1 FROM game_item_nested_entries n
               CROSS JOIN LATERAL game_item_loop_owner(n.parent_item_instance_id) o
               WHERE n.item_instance_id=i.item_instance_id AND n.world_id=r.world_id
                 AND n.parent_item_instance_id=NEW.parent_item_instance_id AND n.placement_ordinal=NEW.placement_ordinal
                 AND o.owner_character=r.character_id)))$new$);
END $migration$;
REVOKE ALL ON FUNCTION game_stash_container_proven() FROM PUBLIC;
