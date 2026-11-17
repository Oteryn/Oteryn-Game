-- T144: immutable selected output evidence, sharing the existing fenced daily/Inbox commit.
CREATE TABLE game_daily_reward_picks (
 claim_id UUID NOT NULL REFERENCES game_character_daily_reward_claims(transaction_id),
 line SMALLINT NOT NULL CHECK(line BETWEEN 0 AND 19),
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 source_id BIGINT NOT NULL CHECK(source_id BETWEEN 1 AND 4294967295),
 quantity SMALLINT NOT NULL CHECK(quantity BETWEEN 1 AND 20),
 charges SMALLINT CHECK(charges=50),
 PRIMARY KEY(claim_id,line)
);
CREATE TRIGGER daily_reward_picks_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
 ON game_daily_reward_picks FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE FUNCTION game_daily_reward_pick_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE c public.game_character_daily_reward_claims%ROWTYPE; allowance INTEGER;
BEGIN
 SELECT * INTO STRICT c FROM public.game_character_daily_reward_claims WHERE transaction_id=NEW.claim_id;
 allowance:=CASE (c.lane_day-1)%7+1 WHEN 1 THEN 5 WHEN 2 THEN 5 WHEN 4 THEN 10 WHEN 6 THEN 1 ELSE 0 END * CASE WHEN c.premium THEN 2 ELSE 1 END;
 IF c.reward_version<>2 OR (SELECT sum(quantity) FROM public.game_daily_reward_picks WHERE claim_id=c.transaction_id)>allowance
 OR NOT EXISTS(SELECT 1 FROM public.game_item_instances i
  JOIN public.game_item_bindings b USING(item_instance_id,world_id)
  JOIN public.game_item_character_inbox_locations l USING(item_instance_id,world_id,character_id)
  WHERE i.item_instance_id=NEW.item_instance_id AND i.world_id=c.world_id AND b.character_id=c.character_id
   AND i.quantity=NEW.quantity AND i.definition_production_key='oteryn:item.tibia.i'||NEW.source_id::text
   AND i.minted_transaction_id=c.transaction_id)
 THEN RAISE EXCEPTION 'daily selection requires current allowance and atomic bound Inbox mint' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER daily_reward_pick_final AFTER INSERT ON game_daily_reward_picks
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_daily_reward_pick_guard();
REVOKE ALL ON game_daily_reward_picks FROM PUBLIC;
GRANT SELECT,INSERT ON game_daily_reward_picks TO oteryn_game_runtime;
GRANT SELECT ON game_daily_reward_picks TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_daily_reward_pick_guard() FROM PUBLIC;
