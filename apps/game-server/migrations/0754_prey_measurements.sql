-- PREY-DIFF measurement: one immutable row in the transaction ending a selection.
CREATE TABLE IF NOT EXISTS game_character_prey_history (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 slot SMALLINT NOT NULL CHECK(slot BETWEEN 1 AND 3),
 selection_epoch BIGINT NOT NULL CHECK(selection_epoch>0),
 creature_key TEXT NOT NULL CHECK(octet_length(creature_key) BETWEEN 1 AND 256),
 tier SMALLINT NOT NULL CHECK(tier BETWEEN 0 AND 5),
 bonus_type SMALLINT NOT NULL CHECK(bonus_type BETWEEN 1 AND 4),
 grade SMALLINT NOT NULL CHECK(grade BETWEEN 1 AND 10),
 -- NULL/zero tier only describe a legacy selection predating these measurements.
 selected_at_unix_ms BIGINT CHECK(selected_at_unix_ms>=0),
 ended_at_unix_ms BIGINT NOT NULL CHECK(ended_at_unix_ms>=0),
 consumed_combat_seconds BIGINT NOT NULL CHECK(consumed_combat_seconds BETWEEN 0 AND 7200),
 wildcards_used BIGINT NOT NULL CHECK(wildcards_used>=0),
 list_rerolls_free BIGINT NOT NULL CHECK(list_rerolls_free>=0),
 list_rerolls_gold BIGINT NOT NULL CHECK(list_rerolls_gold>=0),
 chosen_kills BIGINT NOT NULL CHECK(chosen_kills>=0),
 chosen_loot_gold BIGINT NOT NULL CHECK(chosen_loot_gold>=0),
 pick_again_gold BIGINT NOT NULL CHECK(pick_again_gold>=0),
 end_reason TEXT NOT NULL CHECK(end_reason IN('EXPIRED','REPLACED','CANCELLED')),
 end_occurrence UUID NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,slot,selection_epoch),
 CHECK(selected_at_unix_ms IS NULL OR ended_at_unix_ms>=selected_at_unix_ms),
 FOREIGN KEY(character_id,end_occurrence) REFERENCES game_character_prey_receipts(character_id,occurrence) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX IF NOT EXISTS game_prey_history_receipt ON game_character_prey_history(character_id,end_occurrence);
CREATE INDEX IF NOT EXISTS game_prey_history_tier ON game_character_prey_history(tier,bonus_type);
CREATE INDEX IF NOT EXISTS game_prey_history_creature ON game_character_prey_history(creature_key);
REVOKE ALL ON game_character_prey_history FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_prey_history TO oteryn_game_runtime;

CREATE OR REPLACE FUNCTION game_prey_history_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(SELECT 1 FROM game_character_prey_receipts r
  WHERE r.character_id=NEW.character_id AND r.occurrence=NEW.end_occurrence AND r.created_xact_id=pg_current_xact_id())
  OR NOT EXISTS(SELECT 1 FROM game_character_prey_slots s WHERE s.character_id=NEW.character_id AND s.slot=NEW.slot
   AND (s.selection_epoch>NEW.selection_epoch OR (s.selection_epoch=NEW.selection_epoch AND s.state<>'ACTIVE')))
 THEN RAISE EXCEPTION 'Prey history requires same-transaction transition receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_prey_history_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_prey_history_guard() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_prey_history_consistent ON game_character_prey_history;
CREATE CONSTRAINT TRIGGER game_prey_history_consistent AFTER INSERT ON game_character_prey_history
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_prey_history_guard();
DROP TRIGGER IF EXISTS game_prey_history_immutable ON game_character_prey_history;
CREATE TRIGGER game_prey_history_immutable BEFORE UPDATE OR DELETE ON game_character_prey_history FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
DROP TRIGGER IF EXISTS game_prey_history_no_truncate ON game_character_prey_history;
CREATE TRIGGER game_prey_history_no_truncate BEFORE TRUNCATE ON game_character_prey_history EXECUTE FUNCTION game_character_reject_truncate();

CREATE OR REPLACE VIEW game_prey_balance_by_tier AS
 SELECT tier,bonus_type,count(*) AS selections,
  count(selected_at_unix_ms) AS measured_selections,
  avg((ended_at_unix_ms-selected_at_unix_ms)::numeric/60000) AS real_minutes_per_prey,
  -- Legacy selections have no known beginning or pre-migration wildcard use.
  avg(wildcards_used::numeric) FILTER (WHERE selected_at_unix_ms IS NOT NULL) AS wildcards_per_prey,
  sum(consumed_combat_seconds) AS consumed_combat_seconds,
  sum(list_rerolls_free) AS list_rerolls_free,sum(list_rerolls_gold) AS list_rerolls_gold
 FROM game_character_prey_history WHERE tier BETWEEN 1 AND 5 GROUP BY tier,bonus_type;
CREATE OR REPLACE VIEW game_prey_balance_by_creature AS
 SELECT creature_key,count(*) AS selections,
  sum(chosen_kills) AS kills,sum(chosen_loot_gold) AS npc_loot_gold,
  sum(chosen_kills)::numeric*3600000/NULLIF(sum(ended_at_unix_ms-selected_at_unix_ms),0) AS kills_per_hour,
  sum(chosen_loot_gold)::numeric*3600000/NULLIF(sum(ended_at_unix_ms-selected_at_unix_ms),0) AS loot_gold_per_hour,
  sum(chosen_loot_gold)::numeric/NULLIF(sum(chosen_kills),0) AS loot_gold_per_kill,
  sum(chosen_kills)::numeric*3600/NULLIF(sum(consumed_combat_seconds),0) AS kills_per_combat_hour,
  sum(chosen_loot_gold)::numeric*3600/NULLIF(sum(consumed_combat_seconds),0) AS loot_gold_per_combat_hour
 FROM game_character_prey_history WHERE selected_at_unix_ms IS NOT NULL GROUP BY creature_key;
REVOKE ALL ON game_prey_balance_by_tier,game_prey_balance_by_creature FROM PUBLIC;
GRANT SELECT ON game_prey_balance_by_tier,game_prey_balance_by_creature TO oteryn_game_control;

CREATE OR REPLACE FUNCTION game_prey_ended_selection_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF ROW(NEW.character_id,NEW.slot) IS DISTINCT FROM ROW(OLD.character_id,OLD.slot)
 THEN RAISE EXCEPTION 'Prey slot identity is immutable' USING ERRCODE='23514'; END IF;
 IF OLD.state='ACTIVE' AND NEW.state='ACTIVE' AND NEW.selection_epoch=OLD.selection_epoch AND (
  ROW(NEW.creature_key,NEW.bonus_type,NEW.grade,NEW.bonus_percent,NEW.selected_at_unix_ms)
   IS DISTINCT FROM ROW(OLD.creature_key,OLD.bonus_type,OLD.grade,OLD.bonus_percent,OLD.selected_at_unix_ms)
  OR (OLD.selected_tier<>0 AND NEW.selected_tier<>OLD.selected_tier)
  OR NEW.hunting_s_left>OLD.hunting_s_left
  OR NEW.consumed_combat_seconds<>OLD.consumed_combat_seconds+(OLD.hunting_s_left-NEW.hunting_s_left)
  OR NEW.wildcards_used<OLD.wildcards_used OR NEW.list_rerolls_free<OLD.list_rerolls_free
  OR NEW.list_rerolls_gold<OLD.list_rerolls_gold OR NEW.chosen_kills<OLD.chosen_kills
  OR NEW.chosen_loot_gold<OLD.chosen_loot_gold OR NEW.pick_again_gold<OLD.pick_again_gold
 ) THEN RAISE EXCEPTION 'Prey lifetime identity and cumulative counters cannot be rewritten' USING ERRCODE='23514'; END IF;
 IF OLD.state='ACTIVE' AND (NEW.state<>'ACTIVE' OR NEW.selection_epoch IS DISTINCT FROM OLD.selection_epoch)
  AND NOT EXISTS(SELECT 1 FROM game_character_prey_history h WHERE h.character_id=OLD.character_id AND h.slot=OLD.slot
   AND h.selection_epoch=OLD.selection_epoch AND h.creature_key=OLD.creature_key AND h.grade=OLD.grade
   AND h.bonus_type=CASE OLD.bonus_type WHEN 'DAMAGE' THEN 1 WHEN 'REDUCTION' THEN 2 WHEN 'XP' THEN 3 WHEN 'LOOT' THEN 4 END
   AND h.selected_at_unix_ms IS NOT DISTINCT FROM OLD.selected_at_unix_ms
   AND (h.tier=OLD.selected_tier OR OLD.selected_tier=0)
   AND h.consumed_combat_seconds>=OLD.consumed_combat_seconds AND h.wildcards_used>=OLD.wildcards_used
   AND h.list_rerolls_free>=OLD.list_rerolls_free AND h.list_rerolls_gold>=OLD.list_rerolls_gold
   AND h.chosen_kills>=OLD.chosen_kills AND h.chosen_loot_gold>=OLD.chosen_loot_gold
   AND h.pick_again_gold>=OLD.pick_again_gold
   AND h.created_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'Ending a Prey selection requires its atomic measurement row' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_prey_ended_selection_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_prey_ended_selection_guard() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_prey_ended_selection_consistent ON game_character_prey_slots;
CREATE CONSTRAINT TRIGGER game_prey_ended_selection_consistent AFTER UPDATE ON game_character_prey_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_prey_ended_selection_guard();
