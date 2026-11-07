-- STAMINA-1: admit receipt-owned hunting decreases; no new clock column.
ALTER TABLE game_character_build_receipts DROP CONSTRAINT game_character_build_receipts_cause_direction,
 ADD CONSTRAINT game_character_build_receipts_cause_direction CHECK (CASE cause
        WHEN 'training' THEN
            vocation_after = vocation_before
            AND stamina_minutes_after <= stamina_minutes_before
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
            AND (stamina_minutes_after < stamina_minutes_before OR (magic_level_after, mana_spent_after) > (magic_level_before, mana_spent_before)
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

CREATE OR REPLACE FUNCTION game_verify_training_integrity(p_character UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE v_state game_character_build_state%ROWTYPE; v_last RECORD; v_first_training NUMERIC;
BEGIN
 SELECT * INTO v_state FROM game_character_build_state WHERE character_id=p_character;
 IF NOT FOUND THEN RETURN; END IF;
 -- 0141 stamina predates this receipt chain. Its current value is the baseline
 -- at the first marker or hunting decrease; historical defaults are not evidence
 -- of an earlier stamina value. Pool/skill are new here and always chain-owned.
 SELECT min(b.committed_character_revision) INTO v_first_training
  FROM game_character_build_receipts b
  WHERE b.character_id=p_character AND (b.cause IN ('logout','offline_settlement') OR b.stamina_minutes_before<>b.stamina_minutes_after);
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
    OR (b.cause='training' AND (b.stamina_minutes_after>b.stamina_minutes_before OR (b.offline_pool_s_before,b.offline_skill_before) IS DISTINCT FROM (b.offline_pool_s_after,b.offline_skill_after)))
    OR (b.cause NOT IN ('training','logout','offline_settlement') AND (b.stamina_minutes_before,b.offline_pool_s_before,b.offline_skill_before) IS DISTINCT FROM (b.stamina_minutes_after,b.offline_pool_s_after,b.offline_skill_after)))) THEN
  RAISE EXCEPTION 'offline marker provenance mismatch';
 END IF;
END $$;

CREATE FUNCTION game_hunting_set_stamina(p_character UUID,p_occurrence UUID,p_target INTEGER)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE v_state game_character_build_state%ROWTYPE; v_receipt game_character_build_receipts%ROWTYPE;
BEGIN
 IF p_character IS NULL OR p_occurrence IS NULL OR p_target IS NULL OR p_target NOT BETWEEN 0 AND 2519 THEN
  RAISE EXCEPTION 'invalid hunting stamina inputs';
 END IF;
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'hunting character missing'; END IF;
 SELECT * INTO v_state FROM game_character_build_state WHERE character_id=p_character FOR UPDATE;
 SELECT * INTO v_receipt FROM game_character_build_receipts b WHERE b.build_occurrence_id=p_occurrence
  AND b.character_id=p_character AND b.cause='training' AND b.created_xact_id=pg_current_xact_id()
  AND b.build_occurrence_id=v_state.last_build_occurrence_id
  AND b.committed_character_revision=v_state.committed_character_revision;
 IF NOT FOUND OR v_state.stamina_minutes<>v_receipt.stamina_minutes_before
  OR p_target<>v_receipt.stamina_minutes_after OR p_target>=v_state.stamina_minutes THEN
  RAISE EXCEPTION 'hunting stamina receipt mismatch';
 END IF;
 UPDATE game_character_build_state SET stamina_minutes=p_target WHERE character_id=p_character;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_hunting_set_stamina(UUID,UUID,INTEGER) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_hunting_set_stamina(UUID,UUID,INTEGER) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_hunting_set_stamina(UUID,UUID,INTEGER) TO oteryn_game_runtime;
