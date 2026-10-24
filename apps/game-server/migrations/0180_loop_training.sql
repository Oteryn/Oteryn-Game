-- LOOP-TRAIN-1, OFFLINE0 §§3–6: extend the canonical build chain, never a second progress owner.
-- Source rates remain provisional and disabled in shipped configuration.
ALTER TABLE game_durability_reconnect_sessions
    ADD COLUMN training_session_sequence BIGINT GENERATED ALWAYS AS IDENTITY,
    ADD COLUMN training_created_at_us BIGINT NULL CHECK (training_created_at_us >= 0),
    ADD COLUMN training_predecessor_session UUID NULL REFERENCES game_durability_reconnect_sessions(game_session_id);
CREATE INDEX game_training_session_predecessor ON game_durability_reconnect_sessions(character_id, training_session_sequence DESC) WHERE training_created_at_us IS NOT NULL;
CREATE FUNCTION game_training_session_clock() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        -- Existing admission serialization owns the Character key; DB time and explicit
        -- predecessor identity survive reconnect/restart. Never infer lease minus one.
        PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
        NEW.training_created_at_us := floor(extract(epoch FROM clock_timestamp())*1000000)::bigint;
        SELECT game_session_id INTO NEW.training_predecessor_session
          FROM game_durability_reconnect_sessions WHERE character_id=NEW.character_id
            AND training_created_at_us IS NOT NULL
          ORDER BY training_session_sequence DESC LIMIT 1;
    ELSIF NEW.training_session_sequence IS DISTINCT FROM OLD.training_session_sequence
       OR NEW.training_created_at_us IS DISTINCT FROM OLD.training_created_at_us
       OR NEW.training_predecessor_session IS DISTINCT FROM OLD.training_predecessor_session THEN
        RAISE EXCEPTION 'training session clock is immutable';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_training_session_clock BEFORE INSERT OR UPDATE ON game_durability_reconnect_sessions FOR EACH ROW EXECUTE FUNCTION game_training_session_clock();

ALTER TABLE game_character_build_state
 ADD COLUMN offline_pool_s INTEGER NOT NULL DEFAULT 43200 CHECK (offline_pool_s BETWEEN 0 AND 43200),
 ADD COLUMN offline_skill SMALLINT NOT NULL DEFAULT -1 CHECK (offline_skill BETWEEN -1 AND 5);
ALTER TABLE game_character_build_receipts
 ADD COLUMN stamina_minutes_before INTEGER NOT NULL DEFAULT 2520 CHECK (stamina_minutes_before BETWEEN 0 AND 2520),
 ADD COLUMN stamina_minutes_after INTEGER NOT NULL DEFAULT 2520 CHECK (stamina_minutes_after BETWEEN 0 AND 2520),
 ADD COLUMN offline_pool_s_before INTEGER NOT NULL DEFAULT 43200 CHECK (offline_pool_s_before BETWEEN 0 AND 43200),
 ADD COLUMN offline_pool_s_after INTEGER NOT NULL DEFAULT 43200 CHECK (offline_pool_s_after BETWEEN 0 AND 43200),
 ADD COLUMN offline_skill_before SMALLINT NOT NULL DEFAULT -1 CHECK (offline_skill_before BETWEEN -1 AND 5),
 ADD COLUMN offline_skill_after SMALLINT NOT NULL DEFAULT -1 CHECK (offline_skill_after BETWEEN -1 AND 5);
ALTER TABLE game_character_death_receipts
 ADD COLUMN stamina_minutes_before INTEGER NULL CHECK (stamina_minutes_before BETWEEN 0 AND 2520),
 ADD COLUMN stamina_minutes_after INTEGER NULL CHECK (stamina_minutes_after BETWEEN 0 AND 2520),
 ADD COLUMN offline_pool_s_before INTEGER NULL CHECK (offline_pool_s_before BETWEEN 0 AND 43200),
 ADD COLUMN offline_pool_s_after INTEGER NULL CHECK (offline_pool_s_after BETWEEN 0 AND 43200),
 ADD COLUMN offline_skill_before SMALLINT NULL CHECK (offline_skill_before BETWEEN -1 AND 5),
 ADD COLUMN offline_skill_after SMALLINT NULL CHECK (offline_skill_after BETWEEN -1 AND 5);
-- Historical deaths inherit the pre-feature seed; no historical gameplay value is invented.
ALTER TABLE game_character_death_receipts DISABLE TRIGGER USER;
UPDATE game_character_death_receipts SET stamina_minutes_before=2520,stamina_minutes_after=2520,
 offline_pool_s_before=43200,offline_pool_s_after=43200,offline_skill_before=-1,offline_skill_after=-1
 WHERE vocation IS NOT NULL;
ALTER TABLE game_character_death_receipts ENABLE TRIGGER USER;
ALTER TABLE game_character_death_receipts ADD CONSTRAINT game_death_offline_all_or_none CHECK (
 (vocation IS NULL AND num_nonnulls(stamina_minutes_before,stamina_minutes_after,offline_pool_s_before,offline_pool_s_after,offline_skill_before,offline_skill_after)=0)
 OR (vocation IS NOT NULL AND num_nulls(stamina_minutes_before,stamina_minutes_after,offline_pool_s_before,offline_pool_s_after,offline_skill_before,offline_skill_after)=0
 AND (stamina_minutes_before,offline_pool_s_before,offline_skill_before)=(stamina_minutes_after,offline_pool_s_after,offline_skill_after)));
CREATE FUNCTION game_training_death_carry() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.vocation IS NOT NULL THEN
  SELECT stamina_minutes,offline_pool_s,offline_skill INTO NEW.stamina_minutes_before,NEW.offline_pool_s_before,NEW.offline_skill_before
    FROM game_character_build_state WHERE character_id=NEW.character_id;
  NEW.stamina_minutes_before:=coalesce(NEW.stamina_minutes_before,2520);
  NEW.offline_pool_s_before:=coalesce(NEW.offline_pool_s_before,43200);
  NEW.offline_skill_before:=coalesce(NEW.offline_skill_before,-1);
  NEW.stamina_minutes_after:=NEW.stamina_minutes_before;
  NEW.offline_pool_s_after:=NEW.offline_pool_s_before;
  NEW.offline_skill_after:=NEW.offline_skill_before;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_training_death_carry BEFORE INSERT ON game_character_death_receipts FOR EACH ROW EXECUTE FUNCTION game_training_death_carry();
ALTER TABLE game_character_build_receipts DROP CONSTRAINT game_character_build_receipts_cause_check,
 DROP CONSTRAINT game_character_build_receipts_cause_direction,
 ADD CONSTRAINT game_character_build_receipts_cause_check CHECK (cause IN ('training','vocation_choice','promotion','logout','offline_settlement')),
 ADD CONSTRAINT game_character_build_receipts_cause_direction CHECK (CASE cause
        WHEN 'training' THEN
            vocation_after = vocation_before
            AND (magic_level_after, mana_spent_after) >= (magic_level_before, mana_spent_before)
            AND (fist_level_after, fist_tries_after) >= (fist_level_before, fist_tries_before)
            AND (club_level_after, club_tries_after) >= (club_level_before, club_tries_before)
            AND (sword_level_after, sword_tries_after) >= (sword_level_before, sword_tries_before)
            AND (axe_level_after, axe_tries_after) >= (axe_level_before, axe_tries_before)
            AND (distance_level_after, distance_tries_after)
                >= (distance_level_before, distance_tries_before)
            AND (shielding_level_after, shielding_tries_after)
                >= (shielding_level_before, shielding_tries_before)
            AND (fishing_level_after, fishing_tries_after)
                >= (fishing_level_before, fishing_tries_before)
            AND ((magic_level_after, mana_spent_after) > (magic_level_before, mana_spent_before)
              OR (fist_level_after, fist_tries_after) > (fist_level_before, fist_tries_before)
              OR (club_level_after, club_tries_after) > (club_level_before, club_tries_before)
              OR (sword_level_after, sword_tries_after) > (sword_level_before, sword_tries_before)
              OR (axe_level_after, axe_tries_after) > (axe_level_before, axe_tries_before)
              OR (distance_level_after, distance_tries_after)
                 > (distance_level_before, distance_tries_before)
              OR (shielding_level_after, shielding_tries_after)
                 > (shielding_level_before, shielding_tries_before)
              OR (fishing_level_after, fishing_tries_after)
                 > (fishing_level_before, fishing_tries_before))
        WHEN 'vocation_choice' THEN
            vocation_before = 'none' AND vocation_after <> 'none'
        WHEN 'promotion' THEN
            vocation_before <> 'none' AND vocation_after <> 'none'
            AND vocation_after <> vocation_before
            AND (magic_level_after, mana_spent_after, fist_level_after, fist_tries_after,
                 club_level_after, club_tries_after, sword_level_after, sword_tries_after,
                 axe_level_after, axe_tries_after, distance_level_after, distance_tries_after,
                 shielding_level_after, shielding_tries_after, fishing_level_after,
                 fishing_tries_after)
              = (magic_level_before, mana_spent_before, fist_level_before, fist_tries_before,
                 club_level_before, club_tries_before, sword_level_before, sword_tries_before,
                 axe_level_before, axe_tries_before, distance_level_before, distance_tries_before,
                 shielding_level_before, shielding_tries_before, fishing_level_before,
                 fishing_tries_before)
        WHEN 'offline_settlement' THEN
            vocation_after = vocation_before
            AND (magic_level_after, mana_spent_after) >= (magic_level_before, mana_spent_before)
            AND (fist_level_after, fist_tries_after) >= (fist_level_before, fist_tries_before)
            AND (club_level_after, club_tries_after) >= (club_level_before, club_tries_before)
            AND (sword_level_after, sword_tries_after) >= (sword_level_before, sword_tries_before)
            AND (axe_level_after, axe_tries_after) >= (axe_level_before, axe_tries_before)
            AND (distance_level_after, distance_tries_after)
                >= (distance_level_before, distance_tries_before)
            AND (shielding_level_after, shielding_tries_after)
                >= (shielding_level_before, shielding_tries_before)
            AND (fishing_level_after, fishing_tries_after)
                >= (fishing_level_before, fishing_tries_before)
        WHEN 'logout' THEN vocation_after = vocation_before AND (magic_level_after, mana_spent_after, fist_level_after, fist_tries_after,
                 club_level_after, club_tries_after, sword_level_after, sword_tries_after,
                 axe_level_after, axe_tries_after, distance_level_after, distance_tries_after,
                 shielding_level_after, shielding_tries_after, fishing_level_after,
                 fishing_tries_after)
              = (magic_level_before, mana_spent_before, fist_level_before, fist_tries_before,
                 club_level_before, club_tries_before, sword_level_before, sword_tries_before,
                 axe_level_before, axe_tries_before, distance_level_before, distance_tries_before,
                 shielding_level_before, shielding_tries_before, fishing_level_before,
                 fishing_tries_before)
    END);

CREATE TABLE game_character_training_markers (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
 lease_generation NUMERIC(20,0) NOT NULL CHECK (lease_generation BETWEEN 1 AND 18446744073709551615),
 marker_version SMALLINT NOT NULL DEFAULT 2 CHECK (marker_version=2),
 cause TEXT NOT NULL CHECK (cause IN ('logout','offline_settlement')),
 build_occurrence_id UUID NOT NULL UNIQUE REFERENCES game_character_build_receipts(build_occurrence_id) DEFERRABLE INITIALLY DEFERRED,
 selected_skill SMALLINT NOT NULL CHECK (selected_skill BETWEEN -1 AND 5),
 committed_at_us BIGINT NOT NULL CHECK (committed_at_us>=0),
 as_of_us BIGINT NOT NULL CHECK (as_of_us=committed_at_us),
 PRIMARY KEY(character_id,lease_generation,cause)
);
CREATE FUNCTION game_training_marker_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 NEW.committed_at_us:=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint;
 NEW.as_of_us:=NEW.committed_at_us;
 IF NOT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.game_session_id=NEW.game_session_id
    AND s.character_id=NEW.character_id AND s.character_lease_generation=NEW.lease_generation) THEN
   RAISE EXCEPTION 'training marker session mismatch';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_training_marker_stamp BEFORE INSERT ON game_character_training_markers FOR EACH ROW EXECUTE FUNCTION game_training_marker_stamp();
CREATE TRIGGER game_training_marker_immutable BEFORE UPDATE OR DELETE ON game_character_training_markers FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_training_marker_no_truncate BEFORE TRUNCATE ON game_character_training_markers EXECUTE FUNCTION game_character_reject_truncate();

-- Independent deferred chain validation covers ordinary build and death writers as well as
-- markers. Direct row edits and forged no-op marker causes cannot bypass the chain.
CREATE FUNCTION game_verify_training_integrity(p_character UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE v_state game_character_build_state%ROWTYPE; v_last RECORD; v_first_training NUMERIC;
BEGIN
 SELECT * INTO v_state FROM game_character_build_state WHERE character_id=p_character;
 IF NOT FOUND THEN RETURN; END IF;
 -- 0141 stamina predates this receipt chain. Its current value is the baseline
 -- at the first training marker; historical receipt defaults are not evidence
 -- of an earlier stamina value. Pool/skill are new here and always chain-owned.
 SELECT min(b.committed_character_revision) INTO v_first_training
  FROM game_character_training_markers m JOIN game_character_build_receipts b USING(build_occurrence_id)
  WHERE m.character_id=p_character;
 SELECT * INTO v_last FROM (
  SELECT committed_character_revision r,stamina_minutes_after stamina,offline_pool_s_after pool,offline_skill_after skill
   FROM game_character_build_receipts WHERE character_id=p_character
  UNION ALL
  SELECT committed_character_revision,stamina_minutes_after,offline_pool_s_after,offline_skill_after
   FROM game_character_death_receipts WHERE character_id=p_character AND vocation IS NOT NULL
 ) x ORDER BY r DESC LIMIT 1;
 IF NOT FOUND OR (v_state.offline_pool_s,v_state.offline_skill) IS DISTINCT FROM (v_last.pool,v_last.skill)
    OR (v_first_training IS NOT NULL AND v_state.stamina_minutes<>v_last.stamina) THEN
  RAISE EXCEPTION 'offline build projection mismatch';
 END IF;
 IF EXISTS (SELECT 1 FROM (
  SELECT r,stamina,pool,skill,lag(after_stamina,1,2520) OVER(ORDER BY r) prev_stamina,
   lag(after_pool,1,43200) OVER(ORDER BY r) prev_pool,lag(after_skill,1,-1::smallint) OVER(ORDER BY r) prev_skill FROM (
   SELECT committed_character_revision r,stamina_minutes_before stamina,offline_pool_s_before pool,offline_skill_before skill,
    stamina_minutes_after after_stamina,offline_pool_s_after after_pool,offline_skill_after after_skill FROM game_character_build_receipts WHERE character_id=p_character
   UNION ALL SELECT committed_character_revision,stamina_minutes_before,offline_pool_s_before,offline_skill_before,
    stamina_minutes_after,offline_pool_s_after,offline_skill_after FROM game_character_death_receipts WHERE character_id=p_character AND vocation IS NOT NULL
  ) receipts
 ) chain WHERE (pool,skill) IS DISTINCT FROM (prev_pool,prev_skill)
    OR (r>v_first_training AND stamina<>prev_stamina)) THEN
  RAISE EXCEPTION 'offline receipt chain mismatch';
 END IF;
 IF EXISTS (SELECT 1 FROM game_character_build_receipts b LEFT JOIN game_character_training_markers m USING(build_occurrence_id)
   WHERE b.character_id=p_character AND ((b.cause IN ('logout','offline_settlement') AND (m.build_occurrence_id IS NULL OR m.character_id<>b.character_id OR m.cause<>b.cause OR m.selected_skill<>b.offline_skill_after OR b.committed_at<>m.committed_at_us/1000))
    OR (b.cause NOT IN ('logout','offline_settlement') AND (b.stamina_minutes_before,b.offline_pool_s_before,b.offline_skill_before) IS DISTINCT FROM (b.stamina_minutes_after,b.offline_pool_s_after,b.offline_skill_after)))) THEN
  RAISE EXCEPTION 'offline marker provenance mismatch';
 END IF;
END $$;
CREATE FUNCTION game_training_consistency_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_verify_training_integrity(NEW.character_id); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER game_training_state_consistent AFTER INSERT OR UPDATE ON game_character_build_state DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_training_consistency_guard();
CREATE CONSTRAINT TRIGGER game_training_build_consistent AFTER INSERT ON game_character_build_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_training_consistency_guard();
CREATE CONSTRAINT TRIGGER game_training_death_consistent AFTER INSERT ON game_character_death_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_training_consistency_guard();
CREATE CONSTRAINT TRIGGER game_training_marker_consistent AFTER INSERT ON game_character_training_markers DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_training_consistency_guard();
REVOKE ALL ON game_character_training_markers FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_training_markers TO oteryn_game_runtime;
GRANT SELECT ON game_character_training_markers TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_verify_training_integrity(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_verify_training_integrity(UUID) TO oteryn_game_runtime,oteryn_game_control;

-- Immutable provenance binds the build gain to its paid timed-row checkpoint/expiry.
CREATE TABLE game_character_exercise_receipts (
 build_occurrence_id UUID PRIMARY KEY REFERENCES game_character_build_receipts(build_occurrence_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 binding BYTEA NOT NULL CHECK (octet_length(binding)=32),
 committed_character_revision NUMERIC(20,0) NOT NULL CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
 charges_before INTEGER NOT NULL CHECK (charges_before BETWEEN 1 AND 65535),
 charges_after INTEGER NOT NULL CHECK (charges_after>=0 AND charges_after<charges_before),
 expected_weapon_revision NUMERIC(20,0) NOT NULL CHECK (expected_weapon_revision BETWEEN 0 AND 18446744073709551614),
 weapon_revision NUMERIC(20,0) NOT NULL CHECK (weapon_revision=expected_weapon_revision+CASE WHEN charges_after=0 THEN 0 ELSE 1 END),
 family SMALLINT NOT NULL CHECK (family BETWEEN 0 AND 6),
 gain BIGINT NOT NULL CHECK (gain>0),
 gain_per_charge BIGINT NOT NULL CHECK (gain_per_charge BETWEEN 1 AND 1000000),
 CHECK (gain=(charges_before-charges_after)::bigint*gain_per_charge),
 UNIQUE(item_instance_id,expected_weapon_revision)
);
CREATE TRIGGER game_exercise_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_exercise_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_exercise_receipt_no_truncate BEFORE TRUNCATE ON game_character_exercise_receipts EXECUTE FUNCTION game_character_reject_truncate();
REVOKE ALL ON game_character_exercise_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_exercise_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_exercise_receipts TO oteryn_game_control;

GRANT INSERT(offline_pool_s,offline_skill),UPDATE(offline_pool_s,offline_skill) ON game_character_build_state TO oteryn_game_runtime;

-- Stamina remains outside runtime INSERT/UPDATE privileges (0141). This seam only
-- applies the exact offline recovery of a same-transaction settlement receipt.
CREATE FUNCTION game_training_set_stamina(p_character UUID,p_occurrence UUID,p_target INTEGER)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE
 v_state game_character_build_state%ROWTYPE;
 v_receipt game_character_build_receipts%ROWTYPE;
 v_elapsed BIGINT;
 v_recoverable BIGINT;
 v_missing INTEGER;
 v_expected INTEGER;
BEGIN
 IF p_character IS NULL OR p_occurrence IS NULL OR p_target IS NULL
    OR p_target NOT BETWEEN 0 AND 2520 THEN
  RAISE EXCEPTION 'invalid training stamina inputs';
 END IF;
 -- Match the writer's root -> build lock order.
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'training character missing'; END IF;
 SELECT * INTO v_state FROM game_character_build_state WHERE character_id=p_character FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'training build missing'; END IF;
 SELECT b.* INTO v_receipt FROM game_character_build_receipts b
  JOIN game_character_training_markers m USING(build_occurrence_id)
  WHERE b.build_occurrence_id=p_occurrence AND b.character_id=p_character
   AND b.cause='offline_settlement' AND m.cause=b.cause AND m.character_id=b.character_id
   AND b.created_xact_id=pg_current_xact_id()
   AND v_state.last_build_occurrence_id=b.build_occurrence_id
   AND v_state.committed_character_revision=b.committed_character_revision;
 IF NOT FOUND OR v_state.stamina_minutes<>v_receipt.stamina_minutes_before
    OR p_target<>v_receipt.stamina_minutes_after THEN
  RAISE EXCEPTION 'training stamina settlement mismatch';
 END IF;
 -- Same immutable predecessor and latest-build condition as the native planner.
 -- A crash, missing predecessor or intervening death/build grants no recovery.
 SELECT greatest(0,(s.training_created_at_us-m.committed_at_us)/1000000)::bigint INTO v_elapsed
  FROM game_character_training_markers current_marker
  JOIN game_durability_reconnect_sessions s ON s.game_session_id=current_marker.game_session_id
  JOIN game_durability_reconnect_sessions p ON p.game_session_id=s.training_predecessor_session
  JOIN game_character_training_markers m ON m.game_session_id=p.game_session_id
   AND m.character_id=p.character_id AND m.lease_generation=p.character_lease_generation AND m.cause='logout'
  JOIN game_character_build_receipts b ON b.build_occurrence_id=m.build_occurrence_id
   AND b.committed_character_revision=v_receipt.original_character_revision
  WHERE current_marker.build_occurrence_id=p_occurrence AND s.character_id=p_character
   AND s.training_created_at_us IS NOT NULL AND p.session_state=3 AND p.world_id=s.world_id
   AND m.committed_at_us<=s.training_created_at_us;
 -- OFFLINE0: 21-day cap, 600s delay, 180s/min below 39h, then 360s/min to 42h.
 v_recoverable:=greatest(0,least(coalesce(v_elapsed,0),1814400)-600);
 v_missing:=2340-least(v_state.stamina_minutes,2340);
 IF v_recoverable/180<=v_missing THEN
  v_expected:=v_state.stamina_minutes+(v_recoverable/180)::integer;
 ELSE
  v_expected:=least(2520,greatest(v_state.stamina_minutes,2340)
    +((v_recoverable-180*v_missing)/360)::integer);
 END IF;
 IF p_target<>v_expected THEN RAISE EXCEPTION 'training stamina exceeds offline recovery'; END IF;
 UPDATE game_character_build_state SET stamina_minutes=p_target WHERE character_id=p_character;
END $$;
-- Tests and deployments may install the ledger into a dedicated schema.
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_training_set_stamina(UUID,UUID,INTEGER) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_training_set_stamina(UUID,UUID,INTEGER) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_training_set_stamina(UUID,UUID,INTEGER) TO oteryn_game_runtime;

DO $$ BEGIN EXECUTE format('GRANT USAGE ON SEQUENCE %s TO oteryn_game_runtime',pg_get_serial_sequence('game_durability_reconnect_sessions','training_session_sequence')); END $$;
-- The marker's single post-lock DB clock is also the build receipt clock.
CREATE FUNCTION game_training_build_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE stamp BIGINT;
BEGIN
 IF NEW.cause IN ('logout','offline_settlement') THEN
  SELECT committed_at_us INTO STRICT stamp FROM game_character_training_markers WHERE build_occurrence_id=NEW.build_occurrence_id;
  NEW.committed_at:=stamp/1000;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_training_build_stamp BEFORE INSERT ON game_character_build_receipts FOR EACH ROW EXECUTE FUNCTION game_training_build_stamp();

-- The source write and gained build must be the same transaction, with matching charges.
ALTER TABLE game_item_timed_state_writes ADD COLUMN training_created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id();
CREATE FUNCTION game_training_timed_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN NEW.training_created_xact_id:=pg_current_xact_id(); RETURN NEW; END $$;
CREATE TRIGGER game_training_timed_stamp BEFORE INSERT ON game_item_timed_state_writes FOR EACH ROW EXECUTE FUNCTION game_training_timed_stamp();
CREATE FUNCTION game_exercise_receipt_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS (SELECT 1 FROM game_character_build_receipts b JOIN game_item_timed_state_writes t ON t.item_instance_id=NEW.item_instance_id AND t.expected_revision=NEW.expected_weapon_revision
  WHERE b.build_occurrence_id=NEW.build_occurrence_id AND b.character_id=NEW.character_id AND b.cause='training'
    AND b.committed_character_revision=NEW.committed_character_revision AND b.created_xact_id=t.training_created_xact_id
    AND t.transaction_id=NEW.build_occurrence_id AND t.holder_character_id=NEW.character_id
    AND t.charges_before=NEW.charges_before
    AND ((NEW.charges_after>0 AND t.charges_after=NEW.charges_after)
      OR (NEW.charges_after=0 AND t.charges_after=t.charges_before AND EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=NEW.item_instance_id AND i.lifecycle=2)))
    AND t.cause=CASE WHEN NEW.charges_after=0 THEN 3 ELSE 1 END) THEN
  RAISE EXCEPTION 'exercise build and paid charge provenance mismatch';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_exercise_receipt_consistent AFTER INSERT ON game_character_exercise_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_exercise_receipt_guard();
