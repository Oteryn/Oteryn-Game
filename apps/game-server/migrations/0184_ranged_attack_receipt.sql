-- RANGED-ATTACK-1: reuse the Item owner's fenced receipt, lines and equipment writer.
-- A held attack command has many shots. Ordinary spell command uniqueness is unchanged.
ALTER TABLE game_spell_item_receipts ADD COLUMN IF NOT EXISTS swing_ordinal INTEGER;
ALTER TABLE game_spell_item_receipts DROP CONSTRAINT IF EXISTS game_spell_item_receipts_game_session_id_command_id_key;
-- One index also supports generic prepared replay queries: NULL is the ordinary
-- spell occurrence, while nonnegative ordinals are attack occurrences.
CREATE UNIQUE INDEX IF NOT EXISTS game_spell_item_occurrence_once
 ON game_spell_item_receipts(game_session_id,command_id,(COALESCE(swing_ordinal,-1)));
DO $$ DECLARE definition TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT definition FROM pg_constraint
 WHERE conrelid='game_spell_item_receipts'::regclass AND conname='game_spell_item_typed_cause';
 IF position('swing_ordinal' IN definition)>0 THEN RETURN; END IF;
 IF position('spell_family = ''Spell''::text' IN definition)=0 THEN
  RAISE EXCEPTION 'unexpected item receipt source constraint';
 END IF;
 definition:=replace(definition, 'spell_family = ''Spell''::text',
  '((spell_family = ''Spell'' AND swing_ordinal IS NULL) OR (spell_family = ''Item'' AND swing_ordinal BETWEEN 0 AND 65535))');
 ALTER TABLE game_spell_item_receipts DROP CONSTRAINT game_spell_item_typed_cause;
 EXECUTE 'ALTER TABLE game_spell_item_receipts ADD CONSTRAINT game_spell_item_typed_cause '||definition;
END $$;
ALTER TABLE game_spell_item_receipts DROP CONSTRAINT IF EXISTS game_spell_item_attack_binding;
ALTER TABLE game_spell_item_receipts ADD CONSTRAINT game_spell_item_attack_binding CHECK (
 (spell_family<>'Item' OR swing_ordinal IS NOT NULL) AND (swing_ordinal IS NULL OR (cause_kind=0 AND spell_family='Item'
 AND swing_ordinal BETWEEN 0 AND 65535 AND caster_lease_generation IS NOT NULL
 AND (convert_from(intent,'UTF8')::jsonb->>'swing_ordinal')::integer IS NOT DISTINCT FROM swing_ordinal)));

-- The new source may only burn one equipped projectile; spell/creature sources retain
-- their original proof trigger and all other existing mutation guards.
CREATE OR REPLACE FUNCTION game_ranged_attack_receipt_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.swing_ordinal IS NOT NULL AND (
   NEW.operation_count<>1 OR NEW.caster_lease_generation IS NULL
   OR NOT EXISTS (SELECT 1 FROM game_spell_item_lines l
       WHERE l.transaction_id=NEW.transaction_id AND l.operation_kind=5
       AND l.source_equipment_slot IN(5,6,10) AND l.quantity_before=l.quantity_after+1)
   OR (convert_from(NEW.cost,'UTF8')::jsonb->>'mana_before')::numeric
        IS DISTINCT FROM (convert_from(NEW.cost,'UTF8')::jsonb->>'mana_after')::numeric
   OR (convert_from(NEW.cost,'UTF8')::jsonb->>'soul_before')::numeric
        IS DISTINCT FROM (convert_from(NEW.cost,'UTF8')::jsonb->>'soul_after')::numeric
   OR convert_from(NEW.cost,'UTF8')::jsonb->'cooldowns_before'
        IS DISTINCT FROM convert_from(NEW.cost,'UTF8')::jsonb->'cooldowns_after') THEN
  RAISE EXCEPTION 'ranged shot requires one equipped projectile and no spell payment' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS game_ranged_attack_receipt_proven ON game_spell_item_receipts;
CREATE CONSTRAINT TRIGGER game_ranged_attack_receipt_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_ranged_attack_receipt_proven();
