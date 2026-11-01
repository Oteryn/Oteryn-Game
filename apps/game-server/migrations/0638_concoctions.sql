-- W09: Kooldown Aid shares the immutable concoction activation and one-unit burn.
-- It is instantaneous; only its activation timestamp has a 24h cooldown.
-- Legacy Wealth Duplex rows and their expiry remain unchanged by this slice.
-- Rollback: stop routing new Kooldown uses; retain all immutable receipts.

DO $$
DECLARE name TEXT;
        kind_att SMALLINT;
        start_att SMALLINT;
        end_att SMALLINT;
BEGIN
    SELECT attnum INTO STRICT kind_att FROM pg_attribute
     WHERE attrelid='game_character_concoction_effects'::regclass AND attname='concoction';
    SELECT attnum INTO STRICT start_att FROM pg_attribute
     WHERE attrelid='game_character_concoction_effects'::regclass AND attname='started_at';
    SELECT attnum INTO STRICT end_att FROM pg_attribute
     WHERE attrelid='game_character_concoction_effects'::regclass AND attname='expires_at';
    -- Identify only 0582's two unnamed CHECKs by their exact column dependencies.
    SELECT conname INTO STRICT name FROM pg_constraint
     WHERE conrelid='game_character_concoction_effects'::regclass AND contype='c'
       AND conkey=ARRAY[kind_att]::SMALLINT[];
    EXECUTE format('ALTER TABLE game_character_concoction_effects DROP CONSTRAINT %I',name);
    SELECT conname INTO STRICT name FROM pg_constraint
     WHERE conrelid='game_character_concoction_effects'::regclass AND contype='c'
       AND cardinality(conkey)=2 AND conkey @> ARRAY[start_att,end_att]::SMALLINT[];
    EXECUTE format('ALTER TABLE game_character_concoction_effects DROP CONSTRAINT %I',name);
END $$;

ALTER TABLE game_character_concoction_effects
 ADD COLUMN wire_binding BYTEA CHECK(wire_binding IS NULL OR octet_length(wire_binding)=32),
 ADD CONSTRAINT game_concoction_kind CHECK(concoction IN ('wealth_duplex','kooldown_aid')),
 ADD CONSTRAINT game_concoction_duration CHECK(
   (concoction='wealth_duplex' AND expires_at=started_at+interval '1 hour') OR
   (concoction='kooldown_aid' AND expires_at=started_at AND wire_binding IS NOT NULL)
 );

CREATE FUNCTION game_concoction_activation_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE used game_item_use_records%ROWTYPE;
        expected_key TEXT;
        at TIMESTAMPTZ;
BEGIN
    SELECT * INTO used FROM game_item_use_records WHERE transaction_id=NEW.transaction_id;
    expected_key := CASE NEW.concoction
        WHEN 'wealth_duplex' THEN 'oteryn:item.tibia.i36727'
        WHEN 'kooldown_aid' THEN 'oteryn:item.tibia.i36723' END;
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
        WHEN 'wealth_duplex' THEN at+interval '1 hour' ELSE at END;
    RETURN NEW;
END $$;
CREATE TRIGGER game_concoction_activation_before_insert
 BEFORE INSERT ON game_character_concoction_effects
 FOR EACH ROW EXECUTE FUNCTION game_concoction_activation_guard();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_concoction_activation_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_concoction_activation_guard() FROM PUBLIC;
-- 0582's immutable, no-TRUNCATE, same-xact and reverse burn/effect proofs remain in force.
-- Its runtime SELECT/INSERT grants remain sufficient; no UPDATE/DELETE grant is added.
