-- T193: keep daily credits immutable; spend them through a separate checked chain.
CREATE TABLE game_cosmetic_token_purchases (
 purchase_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 world_id UUID NOT NULL,
 previous_purchase_id UUID,
 price BIGINT NOT NULL CHECK(price>0),
 spent_before BIGINT NOT NULL CHECK(spent_before>=0),
 spent_after BIGINT NOT NULL CHECK(spent_after=spent_before+price),
 UNIQUE(character_id,world_id,purchase_id),
 FOREIGN KEY(character_id,world_id,previous_purchase_id)
   REFERENCES game_cosmetic_token_purchases(character_id,world_id,purchase_id),
 CHECK(previous_purchase_id IS NOT NULL OR spent_before=0)
);
CREATE UNIQUE INDEX cosmetic_token_spend_successor ON game_cosmetic_token_purchases(character_id,world_id,previous_purchase_id) NULLS NOT DISTINCT;
CREATE TABLE game_character_cosmetic_token_spends (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 world_id UUID NOT NULL,
 spent BIGINT NOT NULL CHECK(spent>0),
 last_purchase_id UUID NOT NULL,
 PRIMARY KEY(character_id,world_id),
 FOREIGN KEY(character_id,world_id,last_purchase_id)
   REFERENCES game_cosmetic_token_purchases(character_id,world_id,purchase_id)
);
CREATE TABLE game_character_store_cosmetics (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 world_id UUID NOT NULL,
 offer_key TEXT NOT NULL CHECK(octet_length(offer_key) BETWEEN 1 AND 128),
 purchase_id UUID NOT NULL UNIQUE REFERENCES store_purchase_requests(purchase_request_id),
 operation_id UUID UNIQUE,
 PRIMARY KEY(character_id,world_id,offer_key)
);
REVOKE ALL ON game_cosmetic_token_purchases,game_character_cosmetic_token_spends,game_character_store_cosmetics FROM PUBLIC;
GRANT SELECT,INSERT ON game_cosmetic_token_purchases TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_character_cosmetic_token_spends TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE,DELETE ON game_character_store_cosmetics TO oteryn_game_runtime;
GRANT SELECT ON game_cosmetic_token_purchases,game_character_cosmetic_token_spends,game_character_store_cosmetics TO oteryn_game_control;
CREATE TRIGGER cosmetic_token_purchases_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON game_cosmetic_token_purchases FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE TRIGGER cosmetic_token_spends_no_delete BEFORE DELETE OR TRUNCATE ON game_character_cosmetic_token_spends FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE FUNCTION game_cosmetic_spend_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE e public.game_cosmetic_token_purchases%ROWTYPE; previous BIGINT;
BEGIN
 IF TG_TABLE_NAME='game_cosmetic_token_purchases' THEN
  IF NEW.previous_purchase_id IS NOT NULL THEN
   SELECT spent_after INTO STRICT previous FROM public.game_cosmetic_token_purchases WHERE purchase_id=NEW.previous_purchase_id AND character_id=NEW.character_id AND world_id=NEW.world_id;
   IF previous<>NEW.spent_before THEN RAISE EXCEPTION 'cosmetic spend discontinuity' USING ERRCODE='23514'; END IF;
  END IF;
  IF NOT EXISTS(SELECT 1 FROM public.game_store_purchase_history h JOIN public.store_purchase_requests q USING(purchase_request_id) JOIN public.game_character_store_cosmetics g ON g.purchase_id=q.purchase_request_id WHERE h.purchase_request_id=NEW.purchase_id AND h.character_id=NEW.character_id AND h.price=NEW.price AND h.currency='COSMETIC_TOKEN' AND q.state=2 AND g.character_id=NEW.character_id AND g.world_id=NEW.world_id AND g.operation_id IS NOT NULL) THEN
   RAISE EXCEPTION 'cosmetic spend requires atomic grant, answer and history' USING ERRCODE='23514';
  END IF;
 END IF;
 SELECT p.* INTO STRICT e FROM public.game_character_cosmetic_token_spends s JOIN public.game_cosmetic_token_purchases p ON p.purchase_id=s.last_purchase_id JOIN public.game_character_cosmetic_tokens c ON c.character_id=s.character_id AND c.world_id=s.world_id WHERE s.character_id=NEW.character_id AND s.world_id=NEW.world_id AND s.spent=p.spent_after AND s.spent<=c.balance;
 IF EXISTS(SELECT 1 FROM public.game_cosmetic_token_purchases WHERE previous_purchase_id=e.purchase_id) THEN RAISE EXCEPTION 'cosmetic spend must name latest entry' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_cosmetic_spend_guard() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER cosmetic_token_purchase_final AFTER INSERT ON game_cosmetic_token_purchases DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cosmetic_spend_guard();
CREATE CONSTRAINT TRIGGER cosmetic_token_spend_final AFTER INSERT OR UPDATE ON game_character_cosmetic_token_spends DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cosmetic_spend_guard();
CREATE FUNCTION game_store_cosmetic_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN
  IF OLD.operation_id IS NOT NULL THEN RAISE EXCEPTION 'committed cosmetic is immutable' USING ERRCODE='23514'; END IF;
  RETURN OLD;
 END IF;
 IF OLD.operation_id IS NOT NULL OR NEW.operation_id IS NULL OR ROW(NEW.character_id,NEW.world_id,NEW.offer_key,NEW.purchase_id) IS DISTINCT FROM ROW(OLD.character_id,OLD.world_id,OLD.offer_key,OLD.purchase_id) THEN RAISE EXCEPTION 'immutable cosmetic binding' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_store_cosmetic_immutable() FROM PUBLIC;
CREATE TRIGGER store_cosmetic_transition BEFORE UPDATE OR DELETE ON game_character_store_cosmetics FOR EACH ROW EXECUTE FUNCTION game_store_cosmetic_immutable();
CREATE TRIGGER store_cosmetic_no_truncate BEFORE TRUNCATE ON game_character_store_cosmetics FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
-- Preserve the existing item, Hireling, Prey and Charm proof branches.
ALTER TABLE store_delivery_receipts DROP CONSTRAINT store_delivery_payload_kind;
ALTER TABLE store_delivery_receipts ADD CONSTRAINT store_delivery_payload_kind CHECK(
 (item_instance_id IS NOT NULL AND entitlement_key IS NULL)
 OR (item_instance_id IS NULL AND entitlement_key='prey-list-expansion' AND purchase_request_id IS NULL)
 OR (item_instance_id IS NULL AND entitlement_key='charm-expansion' AND purchase_request_id IS NOT NULL)
 OR (item_instance_id IS NULL AND entitlement_key IS NULL AND purchase_request_id IS NOT NULL)
 OR (item_instance_id IS NULL AND entitlement_key LIKE 'store-cosmetic:%' AND purchase_request_id IS NOT NULL));
DROP TRIGGER store_hireling_delivery_proven ON store_delivery_receipts;
CREATE CONSTRAINT TRIGGER store_hireling_delivery_proven AFTER INSERT ON store_delivery_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.purchase_request_id IS NOT NULL AND NEW.entitlement_key IS DISTINCT FROM 'charm-expansion' AND (NEW.entitlement_key IS NULL OR NEW.entitlement_key NOT LIKE 'store-cosmetic:%')) EXECUTE FUNCTION game_store_hireling_delivery_proven();
CREATE FUNCTION game_store_cosmetic_delivery_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM public.game_character_store_cosmetics g JOIN public.store_purchase_requests q ON q.purchase_request_id=g.purchase_id JOIN public.store_receipt_outbox o ON o.operation_id=g.operation_id WHERE g.operation_id=NEW.operation_id AND g.purchase_id=NEW.purchase_request_id AND g.character_id=NEW.character_id AND g.world_id=NEW.world_id AND q.account_id=NEW.account_id AND NEW.entitlement_key='store-cosmetic:'||g.offer_key) THEN RAISE EXCEPTION 'Store cosmetic requires atomic grant and outbox' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_store_cosmetic_delivery_proven() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER store_cosmetic_delivery_proven AFTER INSERT ON store_delivery_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.entitlement_key LIKE 'store-cosmetic:%') EXECUTE FUNCTION game_store_cosmetic_delivery_proven();

CREATE FUNCTION game_store_cosmetic_grant_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 IF NEW.operation_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM public.game_cosmetic_token_purchases p WHERE p.purchase_id=NEW.purchase_id AND p.purchase_id=NEW.operation_id AND p.character_id=NEW.character_id AND p.world_id=NEW.world_id) AND NOT EXISTS(SELECT 1 FROM public.store_delivery_receipts r WHERE r.operation_id=NEW.operation_id AND r.purchase_request_id=NEW.purchase_id AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id AND r.entitlement_key='store-cosmetic:'||NEW.offer_key) THEN RAISE EXCEPTION 'cosmetic grant requires token purchase or Platform delivery proof' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_store_cosmetic_grant_proven() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER store_cosmetic_grant_proven AFTER INSERT OR UPDATE ON game_character_store_cosmetics DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_store_cosmetic_grant_proven();
