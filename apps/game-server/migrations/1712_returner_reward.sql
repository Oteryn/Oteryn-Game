-- T145 owner decision B: use native session history; never infer from a reward streak.
CREATE INDEX game_returner_account_session ON game_durability_reconnect_sessions(account_id,training_session_sequence DESC);
CREATE FUNCTION game_returner_session_eligible(p_session UUID,p_character UUID,p_world UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT coalesce((SELECT p.session_state=3 AND e.ended_at_us>=p.training_created_at_us
   AND e.ended_at_us<=s.training_created_at_us
   AND to_timestamp(s.training_created_at_us/1000000.0)>=
       ((to_timestamp(e.ended_at_us/1000000.0) AT TIME ZONE 'UTC')+interval '12 months') AT TIME ZONE 'UTC'
 FROM public.game_durability_reconnect_sessions s
 JOIN LATERAL (SELECT old.* FROM public.game_durability_reconnect_sessions old
  WHERE old.account_id=s.account_id AND old.training_session_sequence<s.training_session_sequence
   ORDER BY old.training_session_sequence DESC LIMIT 1) p ON true
 JOIN public.game_character_session_ends e ON e.game_session_id=p.game_session_id
 WHERE s.game_session_id=p_session AND s.character_id=p_character AND s.world_id=p_world
  AND s.training_created_at_us IS NOT NULL),false)
$$;
CREATE TABLE game_returner_reward_entitlements (
 offer_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(offer_id)),
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 world_id UUID NOT NULL,
 source_session_id UUID NOT NULL UNIQUE REFERENCES game_durability_reconnect_sessions(game_session_id),
 predecessor_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
 UNIQUE(account_id,predecessor_session_id),
 created_at_us BIGINT NOT NULL DEFAULT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint CHECK(created_at_us>=0)
);
CREATE INDEX game_returner_character_pending ON game_returner_reward_entitlements(character_id,world_id,created_at_us DESC);
CREATE INDEX game_returner_predecessor ON game_returner_reward_entitlements(predecessor_session_id);
CREATE INDEX game_returner_account ON game_returner_reward_entitlements(account_id);
CREATE TABLE game_returner_reward_claims (
 offer_id UUID PRIMARY KEY REFERENCES game_returner_reward_entitlements(offer_id),
 game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 UNIQUE(game_session_id,command_id)
);
CREATE TRIGGER returner_entitlement_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON game_returner_reward_entitlements
 FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE TRIGGER returner_claim_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON game_returner_reward_claims
 FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE FUNCTION game_returner_entitlement_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 IF NOT public.game_returner_session_eligible(NEW.source_session_id,NEW.character_id,NEW.world_id)
 OR NOT EXISTS(SELECT 1 FROM public.game_durability_reconnect_sessions s WHERE s.game_session_id=NEW.source_session_id
  AND s.account_id=NEW.account_id AND s.training_created_at_us<=NEW.created_at_us
  AND NEW.predecessor_session_id=(SELECT p.game_session_id FROM public.game_durability_reconnect_sessions p WHERE p.account_id=s.account_id AND p.training_session_sequence<s.training_session_sequence ORDER BY p.training_session_sequence DESC LIMIT 1))
 THEN RAISE EXCEPTION 'returner entitlement lacks native inactivity evidence' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER returner_entitlement_final AFTER INSERT ON game_returner_reward_entitlements
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_returner_entitlement_guard();
CREATE FUNCTION game_returner_claim_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM public.game_returner_reward_entitlements e
 JOIN public.game_item_instances i ON i.item_instance_id=NEW.item_instance_id
 JOIN public.game_item_character_inbox_locations l ON l.item_instance_id=i.item_instance_id AND l.world_id=i.world_id
 WHERE e.offer_id=NEW.offer_id AND l.character_id=e.character_id AND i.world_id=e.world_id
  AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i266'
  AND i.quantity=1 AND i.minted_transaction_id=e.offer_id)
 THEN RAISE EXCEPTION 'returner claim requires its atomic Inbox mint' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER returner_claim_final AFTER INSERT ON game_returner_reward_claims
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_returner_claim_guard();
REVOKE ALL ON game_returner_reward_entitlements,game_returner_reward_claims FROM PUBLIC;
GRANT SELECT,INSERT ON game_returner_reward_entitlements,game_returner_reward_claims TO oteryn_game_runtime;
GRANT SELECT ON game_returner_reward_entitlements,game_returner_reward_claims TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_returner_session_eligible(UUID,UUID,UUID),game_returner_entitlement_guard(),game_returner_claim_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_returner_session_eligible(UUID,UUID,UUID) TO oteryn_game_runtime,oteryn_game_control;
