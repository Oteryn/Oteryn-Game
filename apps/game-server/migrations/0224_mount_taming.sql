-- MOUNTTAME. Ownership remains in the existing account appearance facts.
-- Retain even non-consuming failures so reconnect/retry cannot reroll an attempt.
-- Rollback: disable new taming at the router in a successor; retain all receipts.
ALTER TABLE game_item_use_records
 DROP CONSTRAINT game_item_use_records_cause_kind_check,
 ADD CONSTRAINT game_item_use_records_cause_kind_check CHECK(cause_kind IN (1,2,3,328)),
 DROP CONSTRAINT game_item_use_cause_shape,
 ADD CONSTRAINT game_item_use_cause_shape
 CHECK((cause_kind IN (1,328) AND shape=1) OR cause_kind=2 OR (cause_kind=3 AND shape IN (3,4)));

CREATE TABLE game_mount_taming_attempts (
 attempt_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(attempt_id)),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 channel_id UUID NOT NULL CHECK(game_character_is_uuid_v7(channel_id)),
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
 game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 wire_binding BYTEA NOT NULL CHECK(octet_length(wire_binding)=32),
 target_identity BYTEA NOT NULL CHECK(octet_length(target_identity)=16),
 target_generation NUMERIC(20,0) NOT NULL CHECK(target_generation BETWEEN 1 AND 18446744073709551615),
 item_key TEXT NOT NULL CHECK(octet_length(item_key) BETWEEN 1 AND 512),
 grant_key TEXT NOT NULL CHECK(octet_length(grant_key) BETWEEN 1 AND 512),
 outcome SMALLINT NOT NULL CHECK(outcome BETWEEN 0 AND 3),
 draw NUMERIC(20,0) NOT NULL CHECK(draw BETWEEN 0 AND 18446744073709551615),
 removed BOOLEAN NOT NULL,
 consumed_transaction_id UUID REFERENCES game_item_use_records(transaction_id)
   DEFERRABLE INITIALLY DEFERRED,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),
 CHECK(consumed_transaction_id IS NULL OR consumed_transaction_id=attempt_id),
 CHECK(outcome<>0 OR (removed AND consumed_transaction_id IS NOT NULL))
);
CREATE UNIQUE INDEX game_mount_taming_removed_once ON game_mount_taming_attempts
 (world_id,channel_id,scope_generation,target_identity,target_generation) WHERE removed;
CREATE TRIGGER game_mount_taming_immutable BEFORE UPDATE OR DELETE ON game_mount_taming_attempts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_mount_taming_no_truncate BEFORE TRUNCATE ON game_mount_taming_attempts
 EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_mount_taming_stamp BEFORE INSERT ON game_mount_taming_attempts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

CREATE FUNCTION game_mount_taming_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE a public.game_mount_taming_attempts%ROWTYPE;
BEGIN
 IF TG_TABLE_NAME='game_mount_taming_attempts' THEN a:=NEW;
 ELSIF TG_TABLE_NAME='game_item_use_records' THEN
   SELECT * INTO a FROM public.game_mount_taming_attempts WHERE consumed_transaction_id=NEW.transaction_id;
   IF a.attempt_id IS NULL THEN RAISE EXCEPTION 'taming burn lacks attempt' USING ERRCODE='23514'; END IF;
 ELSE
   SELECT * INTO a FROM public.game_mount_taming_attempts WHERE attempt_id=NEW.grant_occurrence_id;
   IF a.attempt_id IS NULL OR a.outcome<>0 OR a.grant_key<>NEW.unlock_key
      OR a.character_id<>NEW.granting_character_id OR NEW.content_revision<>'mount-taming-v1'
      OR a.created_xact_id<>pg_current_xact_id() OR NEW.source_event_id<>uuid_send(a.attempt_id) THEN
     RAISE EXCEPTION 'taming grant lacks matching success' USING ERRCODE='23514';
   END IF;
 END IF;
 IF a.consumed_transaction_id IS NOT NULL AND NOT EXISTS (
   SELECT 1 FROM public.game_item_use_records u WHERE u.transaction_id=a.consumed_transaction_id
    AND u.created_xact_id=a.created_xact_id AND u.cause_kind=328 AND u.shape=1
    AND u.character_id=a.character_id AND u.game_session_id=a.game_session_id AND u.command_id=a.command_id
    AND u.world_id=a.world_id AND u.channel_id=a.channel_id AND u.runtime_scope_ownership_generation=a.scope_generation
 ) THEN RAISE EXCEPTION 'taming receipt differs from burn' USING ERRCODE='23514'; END IF;
 IF a.outcome=0 AND NOT EXISTS (
   SELECT 1 FROM public.game_account_appearance_unlock_receipts r
   JOIN public.game_account_appearance_unlocks f USING(grant_occurrence_id,account_id,unlock_key)
   WHERE r.grant_occurrence_id=a.attempt_id AND r.granting_character_id=a.character_id
    AND r.source_kind='mount_taming' AND r.content_revision='mount-taming-v1' AND r.unlock_key=a.grant_key
    AND r.source_event_id=uuid_send(a.attempt_id)
    AND r.account_id=(SELECT account_id FROM public.game_character_roots WHERE character_id=a.character_id)
 ) THEN RAISE EXCEPTION 'taming success lacks earned mount' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_mount_taming_attempt_consistent AFTER INSERT ON game_mount_taming_attempts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_mount_taming_consistent();
CREATE CONSTRAINT TRIGGER game_mount_taming_burn_consistent AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause_kind=328)
 EXECUTE FUNCTION game_mount_taming_consistent();
CREATE CONSTRAINT TRIGGER game_mount_taming_grant_consistent AFTER INSERT ON game_account_appearance_unlock_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.source_kind='mount_taming')
 EXECUTE FUNCTION game_mount_taming_consistent();
REVOKE ALL ON game_mount_taming_attempts FROM PUBLIC;
REVOKE ALL ON FUNCTION game_mount_taming_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_mount_taming_attempts TO oteryn_game_runtime;
GRANT SELECT ON game_mount_taming_attempts TO oteryn_game_control;
