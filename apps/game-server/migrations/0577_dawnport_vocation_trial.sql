-- S16c: Dawnport vocation trial tiles switch a character below level 20 between vocations
-- (Crystal 201189f8 data-global/scripts/movements/others/dawnport_vocation_trial.lua:323-355,
-- data-global/lib/others/dawnport.lua:26-120). The owner re-levels cumulative progress under
-- the new vocation exactly as for vocation_choice; only the vocation direction differs.
ALTER TABLE game_character_build_receipts
 DROP CONSTRAINT game_character_build_receipts_cause_check,
 DROP CONSTRAINT game_character_build_receipts_cause_direction,
 ADD CONSTRAINT game_character_build_receipts_cause_check CHECK (cause IN ('training','resting_stamina','vocation_choice','vocation_trial','promotion','logout','offline_settlement')),
 ADD CONSTRAINT game_character_build_receipts_cause_direction CHECK (CASE cause
        WHEN 'resting_stamina' THEN
            vocation_after = vocation_before
            AND stamina_minutes_after > stamina_minutes_before
            AND stamina_minutes_after <= 2520
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
        WHEN 'vocation_trial' THEN
            vocation_after <> 'none' AND vocation_after <> vocation_before
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
