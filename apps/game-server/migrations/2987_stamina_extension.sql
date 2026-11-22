-- W09 Stamina Extension is an instant activation with a 24-hour per-kind cooldown.
-- Rollback: stop routing new uses and retain the immutable activation and build receipts.
ALTER TABLE game_character_concoction_effects
 DROP CONSTRAINT game_concoction_kind,
 DROP CONSTRAINT game_concoction_duration,
 ADD COLUMN stamina_before INTEGER,
 ADD COLUMN stamina_after INTEGER,
 ADD CONSTRAINT game_concoction_kind
   CHECK(concoction IN ('wealth_duplex','kooldown_aid','stamina_extension')),
 ADD CONSTRAINT game_concoction_duration CHECK(
   (concoction='wealth_duplex' AND expires_at=started_at+interval '1 hour') OR
   (concoction IN ('kooldown_aid','stamina_extension') AND expires_at=started_at AND wire_binding IS NOT NULL)
 ),
 ADD CONSTRAINT game_concoction_stamina_shape CHECK(
   (concoction <> 'stamina_extension' AND stamina_before IS NULL AND stamina_after IS NULL) OR
   (concoction = 'stamina_extension' AND stamina_before IS NOT NULL AND stamina_after IS NOT NULL
     AND stamina_before BETWEEN 0 AND 2520 AND stamina_after=least(2520,stamina_before+60))
 );

-- Extend 0638's shared typed burn, root lock, server clock and per-kind cooldown guard.
DO $$
DECLARE body TEXT;
        arm TEXT := 'WHEN ''kooldown_aid'' THEN ''oteryn:item.tibia.i36723'' END;';
BEGIN
    body := pg_get_functiondef('game_concoction_activation_guard()'::regprocedure);
    IF position(arm IN body)=0 THEN
        RAISE EXCEPTION 'expected the existing concoction activation guard';
    END IF;
    EXECUTE replace(body, arm,
        'WHEN ''kooldown_aid'' THEN ''oteryn:item.tibia.i36723''
         WHEN ''stamina_extension'' THEN ''oteryn:item.tibia.i36725'' END;');
END $$;

CREATE FUNCTION game_stamina_extension_activation_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE projected INTEGER;
BEGIN
    -- The shared activation guard already locked the Character root.
    SELECT stamina_minutes INTO projected FROM game_character_build_state
     WHERE character_id=NEW.character_id;
    IF NEW.stamina_after IS DISTINCT FROM coalesce(projected,2520)
       OR (NEW.stamina_before < NEW.stamina_after AND NOT EXISTS (
         SELECT 1 FROM game_character_build_receipts b
         JOIN game_character_build_state s USING(character_id)
         JOIN game_item_use_records u ON u.transaction_id=b.build_occurrence_id
         WHERE b.build_occurrence_id=NEW.transaction_id AND b.character_id=NEW.character_id
           AND b.cause='resting_stamina' AND b.created_xact_id=pg_current_xact_id()
           AND b.command_binding=u.request_binding
           AND b.build_occurrence_id=s.last_build_occurrence_id
           AND b.stamina_minutes_before=NEW.stamina_before AND b.stamina_minutes_after=NEW.stamina_after
       ))
    THEN RAISE EXCEPTION 'stamina extension activation requires its matching same-transaction stamina gain'
        USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_concoction_stamina_before_insert
 BEFORE INSERT ON game_character_concoction_effects
 FOR EACH ROW WHEN(NEW.concoction='stamina_extension')
 EXECUTE FUNCTION game_stamina_extension_activation_guard();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_stamina_extension_activation_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_stamina_extension_activation_guard() FROM PUBLIC;
REVOKE ALL ON game_character_concoction_effects FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_concoction_effects TO oteryn_game_runtime;
GRANT SELECT ON game_character_concoction_effects TO oteryn_game_control;
