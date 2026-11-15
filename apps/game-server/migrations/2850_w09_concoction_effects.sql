-- W09FX Crystal 201189f8 scripts/systems/concoctions.lua:7-37;
-- config.lua.dist:310-312: 24h cooldown, 1h online duration for every timed identity.
-- Rollback: refuse the added items at dispatch; retain all immutable activation history.
ALTER TABLE game_character_concoction_effects DROP CONSTRAINT game_concoction_kind,
 DROP CONSTRAINT game_concoction_duration,
 ADD CONSTRAINT game_concoction_kind CHECK(concoction IN ('kooldown_aid','strike_enhancement','stamina_extension','charm_upgrade','wealth_duplex','bestiary_betterment','fire_resilience','ice_resilience','earth_resilience','energy_resilience','holy_resilience','death_resilience','physical_resilience','fire_amplification','ice_amplification','earth_amplification','energy_amplification','holy_amplification','death_amplification','physical_amplification')),
 ADD CONSTRAINT game_concoction_duration CHECK(
  (concoction<>'kooldown_aid' AND expires_at=started_at+interval '1 hour') OR
  (concoction='kooldown_aid' AND expires_at=started_at AND wire_binding IS NOT NULL));

CREATE OR REPLACE FUNCTION game_concoction_activation_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE used game_item_use_records%ROWTYPE;
        expected_key TEXT;
        at TIMESTAMPTZ;
BEGIN
    SELECT * INTO used FROM game_item_use_records WHERE transaction_id=NEW.transaction_id;
    expected_key := CASE NEW.concoction
        WHEN 'kooldown_aid' THEN 'oteryn:item.tibia.i36723'
        WHEN 'strike_enhancement' THEN 'oteryn:item.tibia.i36724'
        WHEN 'stamina_extension' THEN 'oteryn:item.tibia.i36725'
        WHEN 'charm_upgrade' THEN 'oteryn:item.tibia.i36726'
        WHEN 'wealth_duplex' THEN 'oteryn:item.tibia.i36727'
        WHEN 'bestiary_betterment' THEN 'oteryn:item.tibia.i36728'
        WHEN 'fire_resilience' THEN 'oteryn:item.tibia.i36729'
        WHEN 'ice_resilience' THEN 'oteryn:item.tibia.i36730'
        WHEN 'earth_resilience' THEN 'oteryn:item.tibia.i36731'
        WHEN 'energy_resilience' THEN 'oteryn:item.tibia.i36732'
        WHEN 'holy_resilience' THEN 'oteryn:item.tibia.i36733'
        WHEN 'death_resilience' THEN 'oteryn:item.tibia.i36734'
        WHEN 'physical_resilience' THEN 'oteryn:item.tibia.i36735'
        WHEN 'fire_amplification' THEN 'oteryn:item.tibia.i36736'
        WHEN 'ice_amplification' THEN 'oteryn:item.tibia.i36737'
        WHEN 'earth_amplification' THEN 'oteryn:item.tibia.i36738'
        WHEN 'energy_amplification' THEN 'oteryn:item.tibia.i36739'
        WHEN 'holy_amplification' THEN 'oteryn:item.tibia.i36740'
        WHEN 'death_amplification' THEN 'oteryn:item.tibia.i36741'
        WHEN 'physical_amplification' THEN 'oteryn:item.tibia.i36742'
        END;
    IF expected_key IS NULL OR used.transaction_id IS NULL
       OR used.cause_kind IS DISTINCT FROM 527 OR used.shape IS DISTINCT FROM 1
       OR used.character_id IS DISTINCT FROM NEW.character_id
       OR used.created_xact_id IS DISTINCT FROM pg_current_xact_id()
       OR used.source_definition_family IS DISTINCT FROM 'Item'
       OR used.source_definition_production_key IS DISTINCT FROM expected_key
    THEN RAISE EXCEPTION 'concoction activation requires its matching same-transaction burn'
        USING ERRCODE='23514'; END IF;

    -- Serialize activations with the same Character root as the canonical item writer.
    -- Neither a direct INSERT nor simultaneous commands can bypass the per-kind cooldown.
    PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'concoction activation requires its Character root' USING ERRCODE='23514';
    END IF;
    at := public.game_daily_effect_now();
    IF EXISTS (SELECT 1 FROM game_character_concoction_effects
                WHERE character_id=NEW.character_id AND concoction=NEW.concoction
                  AND started_at+interval '24 hours'>at)
    THEN RAISE EXCEPTION 'concoction activation is on cooldown' USING ERRCODE='23514'; END IF;
    NEW.started_at := at;
    NEW.expires_at := CASE NEW.concoction
        WHEN 'kooldown_aid' THEN at ELSE at+interval '1 hour' END;
    RETURN NEW;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_concoction_activation_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_concoction_activation_guard() FROM PUBLIC;
-- 0582's immutable, no-TRUNCATE, same-xact and reverse burn/effect proofs remain in force.
-- Its runtime SELECT/INSERT grants remain sufficient; no UPDATE/DELETE grant is added.

-- Crystal player.cpp:6549-6550 doubles the credited increment, including talisman credit.
ALTER TABLE game_character_bestiary_kill_receipts DROP CONSTRAINT game_bestiary_kill_increment_bound,
 ADD CONSTRAINT game_bestiary_kill_increment_bound CHECK(kill_count_increment IN (1,2,4));
