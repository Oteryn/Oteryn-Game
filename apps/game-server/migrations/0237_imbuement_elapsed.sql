-- IMBUETIME: fenced, idempotent elapsed checkpoints and exactly-once expiry.
CREATE TABLE game_item_imbuement_checkpoints (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 game_session_id UUID NOT NULL, character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL, elapsed_ms BIGINT NOT NULL CHECK(elapsed_ms BETWEEN 1 AND 604800000),
 in_combat BOOLEAN NOT NULL, occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
-- Extend the original guard without granting the runtime direct row mutation. Only this
-- transaction's immutable checkpoint receipt admits a decrease or exhausted-row deletion.
CREATE OR REPLACE FUNCTION game_item_imbuement_row_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' AND NOT ((OLD.remaining_ms=0 AND EXISTS(
   SELECT 1 FROM game_item_imbuement_checkpoints c WHERE c.created_xact_id=pg_current_xact_id()))
   OR EXISTS(SELECT 1 FROM game_item_imbuement_records r WHERE r.action=2
     AND r.item_instance_id=OLD.item_instance_id AND r.slot=OLD.slot AND r.category=OLD.category
     AND r.imbuement_key=OLD.imbuement_key AND r.tier=OLD.tier
     AND r.created_xact_id=pg_current_xact_id())) THEN
  RAISE EXCEPTION 'imbuement deletion requires an admitted audited clear or expiry writer' USING ERRCODE='23514';
 END IF;
 IF TG_OP='UPDATE' AND ((NEW.item_instance_id,NEW.slot,NEW.category,NEW.imbuement_key,
   NEW.imbuement_revision,NEW.tier,NEW.applying_occurrence_id,NEW.applying_transaction_id)
   IS DISTINCT FROM (OLD.item_instance_id,OLD.slot,OLD.category,OLD.imbuement_key,
   OLD.imbuement_revision,OLD.tier,OLD.applying_occurrence_id,OLD.applying_transaction_id)
   OR NEW.remaining_ms>OLD.remaining_ms) THEN
  RAISE EXCEPTION 'imbuement binding is immutable and remaining time only decreases' USING ERRCODE='23514';
 END IF;
 IF TG_OP='INSERT' AND NEW.remaining_ms<>72000000 THEN
  RAISE EXCEPTION 'an imbuement starts with twenty hours of eligible time' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_item_imbuement_checkpoint_write() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 UPDATE game_item_imbuements b SET remaining_ms=GREATEST(0,b.remaining_ms-NEW.elapsed_ms)
 FROM game_character_equipment_slots e, game_item_instances i
 WHERE e.character_id=NEW.character_id AND e.item_instance_id=b.item_instance_id
   AND i.item_instance_id=b.item_instance_id AND i.world_id=NEW.world_id AND i.lifecycle=1
   AND (b.category IN (8,9) OR NEW.in_combat);
 DELETE FROM game_item_imbuements b USING game_character_equipment_slots e
 WHERE e.character_id=NEW.character_id AND e.item_instance_id=b.item_instance_id AND b.remaining_ms=0;
 RETURN NULL;
END $$;
CREATE TRIGGER game_item_imbuement_checkpoint_write AFTER INSERT ON game_item_imbuement_checkpoints
 FOR EACH ROW EXECUTE FUNCTION game_item_imbuement_checkpoint_write();
CREATE TRIGGER game_item_imbuement_checkpoints_immutable BEFORE UPDATE OR DELETE ON game_item_imbuement_checkpoints
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_imbuement_checkpoints_no_truncate BEFORE TRUNCATE ON game_item_imbuement_checkpoints
 EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_item_imbuement_checkpoints FROM PUBLIC;
GRANT SELECT,INSERT ON game_item_imbuement_checkpoints TO oteryn_game_runtime;
GRANT SELECT ON game_item_imbuement_checkpoints TO oteryn_game_control;
