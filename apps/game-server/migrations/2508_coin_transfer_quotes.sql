-- T132: quote authority only; Platform remains the sole coin authority.
ALTER TABLE game_coin_operations DROP CONSTRAINT game_coin_operations_amount_check;
ALTER TABLE game_coin_operations ADD CONSTRAINT game_coin_operations_amount_check CHECK(
 (kind='transfer' AND amount>0 AND amount%25=0 AND price=0 AND fee=0 AND held_gold=0 AND seller IS NOT NULL AND buyer IS NOT NULL AND seller<>buyer)
 OR (kind<>'transfer' AND amount BETWEEN 0 AND 64000));
CREATE TABLE game_coin_gift_quotes(
 quote_id UUID PRIMARY KEY DEFAULT game_character_uuid_v7(),
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 actor UUID NOT NULL REFERENCES game_character_roots(character_id),world_id UUID NOT NULL,
 session_id UUID NOT NULL,connection_generation NUMERIC(20,0) NOT NULL,lease_generation NUMERIC(20,0) NOT NULL,
 recipient UUID NOT NULL REFERENCES game_character_roots(character_id),
 recipient_account UUID NOT NULL REFERENCES game_character_account_guards(account_id),recipient_name TEXT NOT NULL,
 amount BIGINT NOT NULL CHECK(amount>0 AND amount%25=0),
 expires_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()+interval '60 seconds',
 consumed_by UUID UNIQUE REFERENCES game_coin_operations(operation_id)
);
CREATE INDEX coin_gift_quote_actor ON game_coin_gift_quotes(actor);
CREATE INDEX coin_gift_quote_account ON game_coin_gift_quotes(account_id);
CREATE INDEX coin_gift_quote_recipient ON game_coin_gift_quotes(recipient);
CREATE INDEX coin_gift_quote_recipient_account ON game_coin_gift_quotes(recipient_account);
CREATE FUNCTION game_coin_gift_quote_terms() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' OR to_jsonb(NEW)-'consumed_by' IS DISTINCT FROM to_jsonb(OLD)-'consumed_by' OR OLD.consumed_by IS NOT NULL THEN
  RAISE EXCEPTION 'immutable gift quote and single confirmation' USING ERRCODE='23514';
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER coin_gift_quote_terms BEFORE UPDATE OR DELETE ON game_coin_gift_quotes FOR EACH ROW EXECUTE FUNCTION game_coin_gift_quote_terms();
REVOKE ALL ON game_coin_gift_quotes FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_coin_gift_quotes TO oteryn_game_runtime;
GRANT SELECT ON game_coin_gift_quotes TO oteryn_game_control;
