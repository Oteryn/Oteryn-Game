-- P08 Proficiency Catalysts (mechanic 0494). One catalyst unit burns through the item-use
-- record (cause 494, shape 1) together with one `catalyst` proficiency receipt whose occurrence is
-- the burn's TransactionId. A catalyst line moves progress as training does. The deferred guard
-- pairs both rows in one transaction and bounds the gain by the burnt catalyst: Lesser 5,000,
-- Proficiency 25,000, Greater 100,000 (English TibiaWiki, read 2026-10-30; Crystal
-- `data/scripts/actions/items/proficiency_catalysts.lua:1-34` for the latter two).
-- Rollback: stop routing catalyst uses in a successor; retain every record and receipt.

ALTER TABLE game_item_use_records
 DROP CONSTRAINT game_item_use_records_cause_kind_check,
 ADD CONSTRAINT game_item_use_records_cause_kind_check CHECK(cause_kind IN (1,2,3,328,494)),
 DROP CONSTRAINT game_item_use_cause_shape,
 ADD CONSTRAINT game_item_use_cause_shape
 CHECK((cause_kind IN (1,328,494) AND shape=1) OR cause_kind=2 OR (cause_kind=3 AND shape IN (3,4)));

ALTER TABLE game_character_proficiency_receipts
    DROP CONSTRAINT game_character_proficiency_receipts_cause_check,
    ADD CONSTRAINT game_character_proficiency_receipts_cause_check
        CHECK (cause IN ('training', 'perk_selection', 'migration', 'perk_modification', 'catalyst'));
ALTER TABLE game_character_proficiency_receipt_lines
    DROP CONSTRAINT game_character_proficiency_receipt_lines_cause_check,
    ADD CONSTRAINT game_character_proficiency_receipt_lines_cause_check
        CHECK (cause IN ('training', 'perk_selection', 'migration', 'perk_modification', 'catalyst')),
    DROP CONSTRAINT game_character_proficiency_line_cause_direction,
    ADD CONSTRAINT game_character_proficiency_line_cause_direction CHECK (CASE cause
        WHEN 'training' THEN progress_after > progress_before
            AND definition_revision_after = definition_revision_before
            AND selections_after IS NOT DISTINCT FROM selections_before
        WHEN 'catalyst' THEN progress_after > progress_before
            AND definition_revision_after = definition_revision_before
            AND selections_after IS NOT DISTINCT FROM selections_before
        WHEN 'perk_selection' THEN progress_after = progress_before
            AND definition_revision_after = definition_revision_before
            AND cardinality(selections_after) = cardinality(selections_before)
            AND ((selections_before[1] IS DISTINCT FROM selections_after[1])::INTEGER
              + (selections_before[2] IS DISTINCT FROM selections_after[2])::INTEGER
              + (selections_before[3] IS DISTINCT FROM selections_after[3])::INTEGER
              + (selections_before[4] IS DISTINCT FROM selections_after[4])::INTEGER
              + (selections_before[5] IS DISTINCT FROM selections_after[5])::INTEGER
              + (selections_before[6] IS DISTINCT FROM selections_after[6])::INTEGER
              + (selections_before[7] IS DISTINCT FROM selections_after[7])::INTEGER) = 1
        WHEN 'migration' THEN progress_after = progress_before
            AND definition_revision_after <> definition_revision_before
        WHEN 'perk_modification' THEN progress_after = progress_before
            AND definition_revision_after = definition_revision_before
            AND selections_after IS NOT DISTINCT FROM selections_before
        ELSE FALSE
    END);

CREATE FUNCTION game_proficiency_catalyst_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE v_id UUID;
BEGIN
 IF TG_TABLE_NAME='game_item_use_records' THEN v_id:=NEW.transaction_id;
 ELSE v_id:=NEW.proficiency_occurrence_id; END IF;
 IF NOT EXISTS (
   SELECT 1 FROM public.game_item_use_records u
   JOIN public.game_character_proficiency_receipts r ON r.proficiency_occurrence_id=u.transaction_id
   JOIN public.game_character_proficiency_receipt_lines l ON l.proficiency_occurrence_id=r.proficiency_occurrence_id
   WHERE u.transaction_id=v_id AND u.cause_kind=494 AND u.shape=1 AND r.cause='catalyst'
    AND r.character_id=u.character_id AND u.created_xact_id=pg_current_xact_id()
    AND l.progress_after-l.progress_before<=CASE u.source_definition_production_key
        WHEN 'oteryn:item.tibia.i54266' THEN 5000
        WHEN 'oteryn:item.tibia.i51588' THEN 25000
        WHEN 'oteryn:item.tibia.i51589' THEN 100000
        ELSE 0 END)
  OR (SELECT count(*) FROM public.game_character_proficiency_receipt_lines
       WHERE proficiency_occurrence_id=v_id)<>1
  OR EXISTS (SELECT 1 FROM public.game_character_proficiency_modification_lines
              WHERE proficiency_occurrence_id=v_id) THEN
   RAISE EXCEPTION 'catalyst burn and proficiency receipt differ' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_proficiency_catalyst_burn_consistent AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause_kind=494)
 EXECUTE FUNCTION game_proficiency_catalyst_consistent();
CREATE CONSTRAINT TRIGGER game_proficiency_catalyst_receipt_consistent
 AFTER INSERT ON game_character_proficiency_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause='catalyst')
 EXECUTE FUNCTION game_proficiency_catalyst_consistent();
REVOKE ALL ON FUNCTION game_proficiency_catalyst_consistent() FROM PUBLIC;
