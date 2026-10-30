-- W22c Wealth Duplex. One use burns one concoction (item-use cause 527) and starts a one-hour
-- effect row in the same transaction; a kill settled while it is live rolls its loot once more.
-- Crystal ff7ede59 concoctions.lua:179-199, config.lua.dist:310-311.
-- Rollback: refuse cause 527 at the router in a successor; retain every effect row.
ALTER TABLE game_item_use_records
 DROP CONSTRAINT game_item_use_records_cause_kind_check,
 ADD CONSTRAINT game_item_use_records_cause_kind_check CHECK(cause_kind IN (1,2,3,328,527)),
 DROP CONSTRAINT game_item_use_cause_shape,
 ADD CONSTRAINT game_item_use_cause_shape
 CHECK((cause_kind IN (1,328,527) AND shape=1) OR cause_kind=2 OR (cause_kind=3 AND shape IN (3,4)));

CREATE TABLE game_character_concoction_effects (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_use_records(transaction_id)
   DEFERRABLE INITIALLY DEFERRED,
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 concoction TEXT NOT NULL CHECK(concoction='wealth_duplex'),
 started_at TIMESTAMPTZ NOT NULL,
 expires_at TIMESTAMPTZ NOT NULL CHECK(expires_at=started_at+interval '1 hour'),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX game_character_concoction_effects_latest
 ON game_character_concoction_effects(character_id,concoction,started_at DESC);
CREATE TRIGGER game_character_concoction_effects_immutable
 BEFORE UPDATE OR DELETE ON game_character_concoction_effects
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_character_concoction_effects_no_truncate
 BEFORE TRUNCATE ON game_character_concoction_effects
 EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_character_concoction_effects_stamp
 BEFORE INSERT ON game_character_concoction_effects
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

-- Each burn has exactly its effect and each effect its burn, written by one transaction.
CREATE FUNCTION game_concoction_effect_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 IF NOT EXISTS (
   SELECT 1 FROM public.game_item_use_records u
   JOIN public.game_character_concoction_effects e USING(transaction_id)
   WHERE u.transaction_id=NEW.transaction_id AND u.cause_kind=527 AND u.shape=1
    AND u.character_id=e.character_id AND u.created_xact_id=e.created_xact_id
 ) THEN RAISE EXCEPTION 'concoction burn and effect differ' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_concoction_effect_has_burn
 AFTER INSERT ON game_character_concoction_effects
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_concoction_effect_consistent();
CREATE CONSTRAINT TRIGGER game_concoction_burn_has_effect
 AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause_kind=527)
 EXECUTE FUNCTION game_concoction_effect_consistent();

REVOKE ALL ON game_character_concoction_effects FROM PUBLIC;
REVOKE ALL ON FUNCTION game_concoction_effect_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_concoction_effects TO oteryn_game_runtime;
GRANT SELECT ON game_character_concoction_effects TO oteryn_game_control;
