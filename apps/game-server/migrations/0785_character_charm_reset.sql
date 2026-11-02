-- WB2/P01. Crystal src/io/iobestiary.cpp:648-672: reset all stages/assignments,
-- refund earned balances, retain promotion and Expansion. Existing cause 1 also describes
-- the reset's unassignment fee, using reserved operation key oteryn:charm.reset_all.
ALTER TABLE game_character_charm_receipts
 ADD COLUMN reset_generation NUMERIC(20,0) NOT NULL DEFAULT 0 CHECK(reset_generation>=0),
 ADD COLUMN created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id();
ALTER TABLE game_character_charm_receipts DROP CONSTRAINT game_character_charm_receipts_command_kind_check;
ALTER TABLE game_character_charm_receipts ADD CONSTRAINT game_character_charm_receipts_command_kind_check CHECK(command_kind IN(1,2,3,4));
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_character_charm_receipts'::regclass AND conname='game_character_charm_receipts_kind_shape';
 ALTER TABLE game_character_charm_receipts DROP CONSTRAINT game_character_charm_receipts_kind_shape;
 EXECUTE 'ALTER TABLE game_character_charm_receipts ADD CONSTRAINT game_character_charm_receipts_kind_shape CHECK(('
 ||substr(body,7)||') OR (command_kind=4 AND charm_key=''oteryn:charm.reset_all'' AND charm_category=1
 AND stage_before IS NULL AND stage_after IS NULL AND stage_cost IS NULL AND race_key IS NULL
 AND fee_transaction_id IS NOT NULL AND fee_gold_units IS NOT NULL))';
END $$;
DROP INDEX game_character_charm_unlock_step;
CREATE UNIQUE INDEX game_character_charm_unlock_step ON game_character_charm_receipts(character_id,charm_key,reset_generation,stage_after) WHERE command_kind=1;
DROP INDEX game_character_charm_assignment_step;
CREATE UNIQUE INDEX game_character_charm_assignment_step ON game_character_charm_receipts(character_id,charm_key,reset_generation,command_kind,assignment_ordinal) WHERE command_kind IN(2,3);

CREATE FUNCTION game_character_charm_reset_generation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 SELECT coalesce(max(committed_character_revision),0) INTO NEW.reset_generation
 FROM game_character_charm_receipts WHERE character_id=NEW.character_id AND command_kind=4;
 RETURN NEW;
END $$;
CREATE TRIGGER game_character_charm_reset_generation BEFORE INSERT ON game_character_charm_receipts
 FOR EACH ROW EXECUTE FUNCTION game_character_charm_reset_generation();

-- The only allowed unlock deletion is the projection of a paid reset at a newer
-- revision in this transaction. Deferred consistency additionally proves its gold fee.
DO $$ DECLARE body TEXT; BEGIN
 body:=pg_get_functiondef('game_character_charm_unlock_row_guard()'::regprocedure);
 body:=replace(body,'BEGIN', 'BEGIN
 IF TG_OP=''DELETE'' AND EXISTS(SELECT 1 FROM game_character_charm_receipts
 WHERE character_id=OLD.character_id AND command_kind=4
 AND committed_character_revision>OLD.committed_character_revision
 AND created_xact_id = pg_current_xact_id()) THEN RETURN OLD; END IF;');
 EXECUTE body;
END $$;
REVOKE ALL ON game_character_charm_unlocks FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_character_charm_unlocks TO oteryn_game_runtime;
GRANT SELECT ON game_character_charm_unlocks TO oteryn_game_control;
CREATE CONSTRAINT TRIGGER game_character_charm_unlock_deleted_consistent AFTER DELETE ON game_character_charm_unlocks
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_charm_consistency_guard();

DO $$ DECLARE body TEXT; BEGIN
 body:=pg_get_functiondef('game_character_charm_consistency_guard()'::regprocedure);
 body:=replace(body,'v_character UUID;', 'v_character UUID; v_reset NUMERIC;');
 body:=replace(body,'    IF EXISTS (', '    SELECT coalesce(max(committed_character_revision),0) INTO v_reset FROM game_character_charm_receipts WHERE character_id=v_character AND command_kind=4;
    IF EXISTS (');
 body:=replace(body,'WHERE r.character_id = v_character AND r.command_kind = 1', 'WHERE r.character_id = v_character AND r.command_kind = 1 AND r.committed_character_revision > v_reset');
 body:=replace(body,'WHERE r.character_id = v_character AND r.command_kind IN (2, 3)', 'WHERE r.character_id = v_character AND r.command_kind IN (2, 3) AND r.committed_character_revision > v_reset');
 body:=replace(body,'WHERE p.character_id = v_character AND p.command_kind = 1', 'WHERE p.character_id = v_character AND p.command_kind = 1 AND p.committed_character_revision > v_reset');
 body:=replace(body,'WHERE r.character_id = v_character AND r.command_kind = 3', 'WHERE r.character_id = v_character AND r.command_kind IN (3,4)');
 EXECUTE body;
END $$;

CREATE FUNCTION game_character_charm_reset_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.command_kind=4 AND (
   EXISTS(SELECT 1 FROM game_character_charm_unlocks WHERE character_id=NEW.character_id AND committed_character_revision<=NEW.committed_character_revision)
   OR EXISTS(SELECT 1 FROM game_character_charm_assignments WHERE character_id=NEW.character_id AND committed_character_revision<=NEW.committed_character_revision)
   OR NEW.fee_gold_units IS DISTINCT FROM (
     SELECT (100000 + CASE WHEN NEW.level_before>100 THEN NEW.level_before*11000 ELSE 0 END)
       * CASE WHEN EXISTS(SELECT 1 FROM game_character_charm_expansions WHERE character_id=NEW.character_id) THEN 75 ELSE 100 END / 100
   )
 ) THEN RAISE EXCEPTION 'Charm reset must clear projections and pay its exact fee' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_character_charm_reset_proven AFTER INSERT ON game_character_charm_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_charm_reset_proven();
REVOKE ALL ON FUNCTION game_character_charm_reset_generation(),game_character_charm_reset_proven() FROM PUBLIC;
