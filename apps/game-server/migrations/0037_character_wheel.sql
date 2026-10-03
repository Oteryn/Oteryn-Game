-- Explicit local candidate Character Wheel owner. No missing projection implies zero.
-- Initial rows record the Game owner's complete empty allocations/selected-gem state;
-- later scroll/quest/gem writers require a separate qualified source contract.
CREATE TABLE game_character_wheel_state (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 allocation INTEGER[] NOT NULL CHECK(array_length(allocation,1)=36 AND array_lower(allocation,1)=1 AND array_position(allocation,NULL) IS NULL AND 0<=ALL(allocation) AND 200>=ALL(allocation)),
 extra_points INTEGER NOT NULL CHECK(extra_points=0),
 maximum_grade_modifier INTEGER NOT NULL CHECK(maximum_grade_modifier=0),
 revelation_bonus INTEGER[] NOT NULL CHECK(revelation_bonus=ARRAY[0,0,0,0]),
 selected_gems JSONB NOT NULL CHECK(selected_gems='[]'::jsonb),
 last_transaction_id UUID NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_character_wheel_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 character_id UUID NOT NULL REFERENCES game_character_wheel_state(character_id),
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_revision NUMERIC(20,0) NOT NULL CHECK(character_revision BETWEEN 1 AND 18446744073709551615),
 connection_generation NUMERIC(20,0) NOT NULL CHECK(connection_generation BETWEEN 1 AND 18446744073709551615),
 lease_generation NUMERIC(20,0) NOT NULL CHECK(lease_generation BETWEEN 1 AND 18446744073709551615),
 world_id UUID NOT NULL,channel_id UUID NOT NULL,
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 revision_before NUMERIC(20,0) NOT NULL,revision_after NUMERIC(20,0) NOT NULL CHECK(revision_after=revision_before+1 AND revision_after<=18446744073709551615),
 allocation_before INTEGER[] NOT NULL,allocation_after INTEGER[] NOT NULL,
 premium_account_id UUID NOT NULL,premium_authority_revision NUMERIC(20,0) NOT NULL,
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32),intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 8192 AND sha256(intent)=binding),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),UNIQUE(game_session_id,command_id)
);
CREATE TABLE game_character_wheel_audit_outbox (
 event_id UUID PRIMARY KEY REFERENCES game_character_wheel_receipts(event_id),
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_character_wheel_receipts(transaction_id),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 8192),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_wheel_state_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN
 IF TG_OP='INSERT' THEN
  IF NEW.revision<>1 OR NEW.last_transaction_id IS NOT NULL OR NEW.allocation<>array_fill(0,ARRAY[36]) THEN RAISE EXCEPTION 'Wheel initial owner shape' USING ERRCODE='23514';END IF;
 ELSE
  IF NEW.character_id<>OLD.character_id OR NEW.content_digest<>OLD.content_digest OR NEW.extra_points<>OLD.extra_points OR NEW.maximum_grade_modifier<>OLD.maximum_grade_modifier OR NEW.revelation_bonus<>OLD.revelation_bonus OR NEW.selected_gems<>OLD.selected_gems OR NEW.created_xact_id<>OLD.created_xact_id OR NEW.revision<>OLD.revision+1 OR NOT EXISTS(
   SELECT 1 FROM game_character_wheel_receipts r WHERE r.transaction_id=NEW.last_transaction_id AND r.character_id=NEW.character_id AND r.content_digest=NEW.content_digest AND r.revision_before=OLD.revision AND r.revision_after=NEW.revision AND r.allocation_before=OLD.allocation AND r.allocation_after=NEW.allocation AND r.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'Wheel successor requires exact same-TX receipt' USING ERRCODE='23514';END IF;
 END IF;
 RETURN NEW;END $$;
CREATE TRIGGER game_wheel_state_guard BEFORE INSERT OR UPDATE ON game_character_wheel_state FOR EACH ROW EXECUTE FUNCTION game_wheel_state_guard();
CREATE FUNCTION game_wheel_receipt_proven() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id()
 OR NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.character_id AND r.account_id=NEW.premium_account_id AND r.world_id=NEW.world_id AND r.lifecycle=1 AND r.character_revision=NEW.character_revision)
 OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id AND s.world_id=NEW.world_id AND s.runtime_scope_kind=1 AND s.runtime_scope_channel_id=NEW.channel_id AND s.current_generation=NEW.connection_generation AND s.character_lease_generation=NEW.lease_generation AND s.scope_ownership_generation=NEW.ownership_generation AND s.session_state IN(1,2))
 OR NOT EXISTS(SELECT 1 FROM game_spell_premium_accounts a WHERE a.account_id=NEW.premium_account_id AND a.authority_revision=NEW.premium_authority_revision AND NOT a.conflicting AND convert_from(a.evidence,'UTF8')::jsonb->>'entitlement_state'='ACTIVE')
 OR NOT EXISTS(SELECT 1 FROM game_character_wheel_state s WHERE s.character_id=NEW.character_id AND s.last_transaction_id=NEW.transaction_id AND s.content_digest=NEW.content_digest AND s.revision=NEW.revision_after AND s.allocation=NEW.allocation_after)
 OR NOT EXISTS(SELECT 1 FROM game_character_wheel_audit_outbox a WHERE a.transaction_id=NEW.transaction_id AND a.event_id=NEW.event_id AND a.envelope=NEW.intent AND a.created_xact_id=pg_current_xact_id()) THEN
 RAISE EXCEPTION 'Wheel receipt requires actual owner successor/audit' USING ERRCODE='23514';END IF;RETURN NULL;END $$;
CREATE CONSTRAINT TRIGGER game_wheel_receipt_proven AFTER INSERT ON game_character_wheel_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_wheel_receipt_proven();
CREATE FUNCTION game_wheel_immutable() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Wheel owner history is immutable' USING ERRCODE='23514';END $$;
CREATE TRIGGER game_wheel_no_delete BEFORE DELETE OR TRUNCATE ON game_character_wheel_state FOR EACH STATEMENT EXECUTE FUNCTION game_wheel_immutable();
CREATE TRIGGER game_wheel_receipt_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON game_character_wheel_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_wheel_immutable();
CREATE TRIGGER game_wheel_audit_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON game_character_wheel_audit_outbox FOR EACH STATEMENT EXECUTE FUNCTION game_wheel_immutable();
ALTER FUNCTION game_wheel_state_guard() SET search_path FROM CURRENT;
ALTER FUNCTION game_wheel_receipt_proven() SET search_path FROM CURRENT;
ALTER FUNCTION game_wheel_immutable() SET search_path FROM CURRENT;
REVOKE ALL ON game_character_wheel_state,game_character_wheel_receipts,game_character_wheel_audit_outbox FROM PUBLIC;
REVOKE ALL ON FUNCTION game_wheel_state_guard(),game_wheel_receipt_proven(),game_wheel_immutable() FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_wheel_state TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_wheel_receipts,game_character_wheel_audit_outbox TO oteryn_game_runtime;
GRANT SELECT ON game_character_wheel_state,game_character_wheel_receipts,game_character_wheel_audit_outbox TO oteryn_game_control;
