-- Every grant in the NPC namespace belongs to an actual reciprocal NPC receipt.
-- A claimed alternative source_kind/provenance is not a separate authority.
CREATE FUNCTION game_npc_grant_namespace_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE npc_namespace_proof JSONB;
BEGIN
 IF NEW.unlock_key NOT LIKE 'oteryn:appearance-grant/npc/%' THEN RETURN NEW;END IF;
 IF TG_TABLE_NAME='game_account_appearance_unlock_receipts' THEN
  npc_namespace_proof:=to_jsonb(NEW);
 ELSE
  SELECT to_jsonb(npc_namespace_receipt) INTO npc_namespace_proof
   FROM game_account_appearance_unlock_receipts npc_namespace_receipt
   WHERE npc_namespace_receipt.grant_occurrence_id=NEW.grant_occurrence_id
    AND npc_namespace_receipt.account_id=NEW.account_id AND npc_namespace_receipt.unlock_key=NEW.unlock_key;
 END IF;
 IF npc_namespace_proof IS NULL OR npc_namespace_proof->>'source_kind' IS DISTINCT FROM 'npc'
  OR npc_namespace_proof->>'content_revision' IS DISTINCT FROM 'crystal-201189f-npc-cosmetics-v1'
  OR NOT EXISTS(SELECT 1 FROM game_npc_cosmetic_receipts npc_namespace_parent
   WHERE npc_namespace_parent.account_id=NEW.account_id
    AND npc_namespace_parent.character_id=(npc_namespace_proof->>'granting_character_id')::uuid
    AND NEW.unlock_key=ANY(npc_namespace_parent.grant_keys)
    AND uuid_send(npc_namespace_parent.transaction_id)=(npc_namespace_proof->>'source_event_id')::bytea
    AND npc_namespace_parent.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'NPC grant namespace requires its actual source parent' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
DO $$BEGIN EXECUTE format('ALTER FUNCTION game_npc_grant_namespace_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());END $$;
REVOKE ALL ON FUNCTION game_npc_grant_namespace_proven() FROM PUBLIC;
CREATE TRIGGER game_npc_grant_namespace_receipt BEFORE INSERT ON game_account_appearance_unlock_receipts
 FOR EACH ROW EXECUTE FUNCTION game_npc_grant_namespace_proven();
CREATE TRIGGER game_npc_grant_namespace_fact BEFORE INSERT ON game_account_appearance_unlocks
 FOR EACH ROW EXECUTE FUNCTION game_npc_grant_namespace_proven();
