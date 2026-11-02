-- OWNER-DEVIATION PREY-DIFF (2026-11-01): per-slot tiers and the daily offer set.
ALTER TABLE game_character_prey_slots
 ADD COLUMN IF NOT EXISTS chosen_tier SMALLINT CHECK(chosen_tier BETWEEN 1 AND 5),
 ADD COLUMN IF NOT EXISTS selection_epoch BIGINT NOT NULL DEFAULT 0 CHECK(selection_epoch>=0),
 ADD COLUMN IF NOT EXISTS selected_at_unix_ms BIGINT CHECK(selected_at_unix_ms>=0),
 ADD COLUMN IF NOT EXISTS selected_tier SMALLINT NOT NULL DEFAULT 0 CHECK(selected_tier BETWEEN 0 AND 5),
 ADD COLUMN IF NOT EXISTS consumed_combat_seconds BIGINT NOT NULL DEFAULT 0 CHECK(consumed_combat_seconds BETWEEN 0 AND 7200),
 ADD COLUMN IF NOT EXISTS wildcards_used BIGINT NOT NULL DEFAULT 0 CHECK(wildcards_used>=0),
 ADD COLUMN IF NOT EXISTS list_rerolls_free BIGINT NOT NULL DEFAULT 0 CHECK(list_rerolls_free>=0),
 ADD COLUMN IF NOT EXISTS list_rerolls_gold BIGINT NOT NULL DEFAULT 0 CHECK(list_rerolls_gold>=0),
 ADD COLUMN IF NOT EXISTS chosen_kills BIGINT NOT NULL DEFAULT 0 CHECK(chosen_kills>=0),
 ADD COLUMN IF NOT EXISTS chosen_loot_gold BIGINT NOT NULL DEFAULT 0 CHECK(chosen_loot_gold>=0),
 ADD COLUMN IF NOT EXISTS pick_again_gold BIGINT NOT NULL DEFAULT 0 CHECK(pick_again_gold>=0);
-- A legacy active selection has an unknown start time; do not fabricate one.
UPDATE game_character_prey_slots SET selection_epoch=1 WHERE state='ACTIVE' AND selection_epoch=0;

CREATE OR REPLACE FUNCTION game_prey_creature_list_valid(keys TEXT[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT CASE WHEN cardinality(keys)>64 THEN FALSE ELSE
  (cardinality(keys)=0 OR (array_ndims(keys)=1 AND array_lower(keys,1)=1))
  AND NOT EXISTS(SELECT 1 FROM unnest(keys) AS k WHERE k IS NULL OR octet_length(k) NOT BETWEEN 1 AND 256)
  AND cardinality(keys)=(SELECT count(DISTINCT k) FROM unnest(keys) AS k)
 END
$$;
REVOKE ALL ON FUNCTION game_prey_creature_list_valid(TEXT[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_prey_creature_list_valid(TEXT[]) TO oteryn_game_runtime,oteryn_game_control;

ALTER TABLE game_character_prey_receipts
 ADD COLUMN IF NOT EXISTS fee_kind TEXT NOT NULL DEFAULT 'LIST_REROLL' CHECK(fee_kind IN('NONE','LIST_REROLL','PICK_AGAIN')),
 ADD COLUMN IF NOT EXISTS fee_slot SMALLINT CHECK(fee_slot BETWEEN 1 AND 3),
 ADD COLUMN IF NOT EXISTS store_expansion BOOLEAN NOT NULL DEFAULT false;
DO $$ DECLARE c RECORD; BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_character_prey_receipts'::regclass AND contype='c'
  AND (pg_get_constraintdef(oid) LIKE '%octet_length(result)%' OR pg_get_constraintdef(oid) LIKE '%fee_gold_units%level_before%') LOOP
  EXECUTE format('ALTER TABLE game_character_prey_receipts DROP CONSTRAINT %I',c.conname);
 END LOOP;
END $$;
ALTER TABLE game_character_prey_receipts
 ADD CONSTRAINT game_prey_reply_bound CHECK(octet_length(result) BETWEEN 1 AND 65536),
 ADD CONSTRAINT game_prey_fee_quote CHECK(fee_gold_units=0 OR (fee_kind='LIST_REROLL' AND fee_gold_units=level_before*200) OR fee_kind='PICK_AGAIN');
-- Historical paid receipts have no typed slot and stay immutable/replayable.
-- Every new paid receipt must bind its slot before a fee can prove the action.
ALTER TABLE game_character_prey_receipts DROP CONSTRAINT IF EXISTS game_prey_paid_fee_slot;
ALTER TABLE game_character_prey_receipts ADD CONSTRAINT game_prey_paid_fee_slot
 CHECK(fee_gold_units=0 OR fee_slot IS NOT NULL) NOT VALID;

CREATE TABLE IF NOT EXISTS game_character_prey_offered (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 creature_key TEXT NOT NULL CHECK(octet_length(creature_key) BETWEEN 1 AND 256),
 reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch BETWEEN 1 AND 18446744073709551615),
 draw_occurrence UUID NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,creature_key),
 FOREIGN KEY(character_id,draw_occurrence) REFERENCES game_character_prey_receipts(character_id,occurrence) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX IF NOT EXISTS game_prey_offered_epoch ON game_character_prey_offered(character_id,reset_epoch,creature_key);
CREATE INDEX IF NOT EXISTS game_prey_offered_receipt ON game_character_prey_offered(character_id,draw_occurrence);
REVOKE ALL ON game_character_prey_offered FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_prey_offered TO oteryn_game_runtime;

CREATE OR REPLACE FUNCTION game_prey_offered_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF current_user='oteryn_game_runtime' AND (
  NEW.created_xact_id<>pg_current_xact_id()
  OR NOT EXISTS(SELECT 1 FROM game_character_prey_receipts r WHERE r.character_id=NEW.character_id AND r.occurrence=NEW.draw_occurrence AND r.created_xact_id=pg_current_xact_id())
  OR NEW.reset_epoch IS DISTINCT FROM (SELECT max(h.reset_epoch) FROM public.game_world_reset_history h JOIN game_character_roots r ON r.world_id=h.world_id WHERE r.character_id=NEW.character_id)
  OR (TG_OP='UPDATE' AND (NEW.character_id IS DISTINCT FROM OLD.character_id OR NEW.creature_key IS DISTINCT FROM OLD.creature_key OR NEW.reset_epoch<=OLD.reset_epoch))
 ) THEN RAISE EXCEPTION 'Prey offers require the current daily epoch and fenced draw receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_prey_offered_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_prey_offered_guard() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_prey_offered_consistent ON game_character_prey_offered;
CREATE CONSTRAINT TRIGGER game_prey_offered_consistent AFTER INSERT OR UPDATE ON game_character_prey_offered
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_prey_offered_guard();
DROP TRIGGER IF EXISTS game_prey_offered_no_truncate ON game_character_prey_offered;
CREATE TRIGGER game_prey_offered_no_truncate BEFORE TRUNCATE ON game_character_prey_offered EXECUTE FUNCTION game_character_reject_truncate();

-- Existing visible lists have no historical draw date. Conservatively exclude
-- them for the current committed day; the next World reset restores eligibility.
INSERT INTO game_character_prey_offered(character_id,creature_key,reset_epoch,draw_occurrence)
 SELECT s.character_id,k.key,h.reset_epoch,r.occurrence
 FROM game_character_prey_slots s JOIN game_character_roots c USING(character_id)
 CROSS JOIN LATERAL(SELECT reset_epoch FROM public.game_world_reset_history WHERE world_id=c.world_id ORDER BY reset_epoch DESC LIMIT 1) h
 CROSS JOIN LATERAL(SELECT occurrence FROM game_character_prey_receipts WHERE character_id=s.character_id ORDER BY committed_character_revision DESC LIMIT 1) r
 CROSS JOIN LATERAL unnest(s.creature_list || CASE WHEN s.creature_key IS NULL THEN '{}'::text[] ELSE ARRAY[s.creature_key] END) AS k(key)
 ON CONFLICT(character_id,creature_key) DO NOTHING;
