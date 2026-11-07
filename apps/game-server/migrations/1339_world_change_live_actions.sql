-- WCONTRIB: mid-epoch effects stay inside the existing World changes owner.
ALTER TABLE game_world_raid_schedule_sources
 DROP CONSTRAINT game_world_raid_schedule_sources_source_kind_check,
 ADD CONSTRAINT game_world_raid_schedule_sources_source_kind_check CHECK(source_kind IN (1,2,3,4));
CREATE INDEX game_world_raid_firings_world_epoch_key
 ON game_world_raid_firings(world_id,reset_epoch,raid_key);
CREATE TABLE game_world_change_live (
 world_id UUID NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 state JSONB NOT NULL CHECK(jsonb_typeof(state)='object' AND octet_length(state::text)<=2048),
 updated_at_ms BIGINT NOT NULL CHECK(updated_at_ms>=0),
 PRIMARY KEY(world_id,reset_epoch),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_change_states
);
CREATE TABLE game_world_change_action_receipts (
 world_id UUID NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 operation_id BYTEA NOT NULL CHECK(octet_length(operation_id)=16),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 before_state JSONB NOT NULL,
 after_state JSONB NOT NULL,
 updated_at_ms BIGINT NOT NULL CHECK(updated_at_ms>=0),
 character_id UUID,
 game_session_id UUID,
 cause_ordinal NUMERIC(20,0),
 transition_key TEXT,
 node_id UUID,
 node_registration_revision NUMERIC(20,0),
 channel_id UUID,
 scope_generation NUMERIC(20,0),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(world_id,reset_epoch,revision),
 UNIQUE(world_id,reset_epoch,operation_id),
 UNIQUE(character_id,game_session_id,cause_ordinal),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_change_states,
 FOREIGN KEY(character_id,game_session_id,cause_ordinal,transition_key)
   REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
   DEFERRABLE INITIALLY DEFERRED,
 CHECK(jsonb_typeof(before_state)='object' AND octet_length(before_state::text)<=2048),
 CHECK(jsonb_typeof(after_state)='object' AND octet_length(after_state::text)<=2048),
 CHECK(CASE WHEN character_id IS NOT NULL THEN
   game_session_id IS NOT NULL AND cause_ordinal IS NOT NULL AND cause_ordinal BETWEEN 1 AND 18446744073709551615
   AND transition_key IS NOT NULL AND node_id IS NULL AND node_registration_revision IS NULL
   AND channel_id IS NULL AND scope_generation IS NULL
 ELSE game_session_id IS NULL AND cause_ordinal IS NULL AND transition_key IS NULL
   AND node_id IS NOT NULL AND node_registration_revision IS NOT NULL AND node_registration_revision BETWEEN 1 AND 18446744073709551615
   AND channel_id IS NOT NULL AND scope_generation IS NOT NULL AND scope_generation BETWEEN 1 AND 18446744073709551615 END)
);

CREATE FUNCTION game_world_change_action_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 IF NOT game_world_reset_map_epoch_current(NEW.world_id,NEW.reset_epoch)
   OR NOT EXISTS(SELECT 1 FROM game_world_reset_active a
     WHERE a.world_id=NEW.world_id AND a.reset_epoch=NEW.reset_epoch) THEN
   RAISE EXCEPTION 'world change action requires current epoch' USING ERRCODE='23514';
 END IF;
 IF NEW.character_id IS NULL THEN
   IF NOT game_node_lock_current_registration(NEW.node_id,NEW.node_registration_revision) THEN
     RAISE EXCEPTION 'world change tick requires current node' USING ERRCODE='23514';
   END IF;
   PERFORM a.scope_key FROM game_runtime_scope_assignments a
    JOIN game_durability_admission_runtime_guards g USING(scope_key)
    WHERE a.world_id=NEW.world_id AND a.channel_id=NEW.channel_id AND a.scope_kind=1 AND a.state=1
      AND a.holder_node_id=NEW.node_id AND a.holder_registration_revision=NEW.node_registration_revision
      AND a.ownership_generation=NEW.scope_generation AND g.ready AND g.ownership_generation=a.ownership_generation
    FOR SHARE OF a,g;
   IF NOT FOUND THEN
     RAISE EXCEPTION 'world change tick requires current Channel generation' USING ERRCODE='23514';
   END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_world_change_live_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_world_change_action_receipts r
   WHERE (r.world_id,r.reset_epoch,r.revision)=(NEW.world_id,NEW.reset_epoch,NEW.revision)
     AND r.after_state=NEW.state AND r.updated_at_ms=NEW.updated_at_ms
     AND r.created_xact_id=pg_current_xact_id()
     AND (TG_OP='INSERT' OR (r.before_state=OLD.state AND NEW.updated_at_ms>=OLD.updated_at_ms))) THEN
   RAISE EXCEPTION 'world live state lacks atomic action receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE FUNCTION game_world_change_live_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' AND NEW.revision=1 THEN RETURN NEW; END IF;
 IF TG_OP='UPDATE' AND NEW.world_id=OLD.world_id AND NEW.reset_epoch=OLD.reset_epoch
   AND NEW.revision=OLD.revision+1 THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'world live state cannot be deleted, rekeyed or skip revision' USING ERRCODE='23514';
END $$;
CREATE FUNCTION game_world_change_action_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_world_change_live s
   WHERE (s.world_id,s.reset_epoch,s.revision)=(NEW.world_id,NEW.reset_epoch,NEW.revision)
     AND s.state=NEW.after_state AND s.updated_at_ms=NEW.updated_at_ms)
   OR NEW.created_xact_id<>pg_current_xact_id()
   OR (NEW.character_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r
     WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.game_session_id
       AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
       AND r.created_xact_id=pg_current_xact_id() AND r.cause_kind IN ('command','use'))) THEN
   RAISE EXCEPTION 'incomplete atomic world change action' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_world_change_action_admit BEFORE INSERT ON game_world_change_action_receipts
 FOR EACH ROW EXECUTE FUNCTION game_world_change_action_admit();
CREATE CONSTRAINT TRIGGER game_world_change_action_complete AFTER INSERT ON game_world_change_action_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_world_change_action_complete();
CREATE CONSTRAINT TRIGGER game_world_change_live_proven AFTER INSERT OR UPDATE ON game_world_change_live
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_world_change_live_proven();
CREATE TRIGGER game_world_change_live_guard BEFORE INSERT OR UPDATE OR DELETE ON game_world_change_live
 FOR EACH ROW EXECUTE FUNCTION game_world_change_live_guard();
CREATE TRIGGER game_world_change_action_immutable BEFORE UPDATE OR DELETE ON game_world_change_action_receipts
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE TRIGGER game_world_change_action_no_truncate BEFORE TRUNCATE ON game_world_change_action_receipts
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_world_change_live_no_truncate BEFORE TRUNCATE ON game_world_change_live
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_world_change_action_admit','game_world_change_action_complete',
   'game_world_change_live_proven','game_world_change_live_guard'] LOOP
   EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
END $$;
REVOKE ALL ON game_world_change_live,game_world_change_action_receipts FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_world_change_live TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_world_change_action_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_live,game_world_change_action_receipts TO oteryn_game_control;

-- Personal two-week repeat restriction reads one bounded historical return.
CREATE INDEX game_world_change_action_character_return
 ON game_world_change_action_receipts(character_id,transition_key,updated_at_ms DESC)
 WHERE character_id IS NOT NULL;


-- Consolidated package migration: 1115_repeatable_quest_rewards.sql
-- Repeatable custody-backed exchanges retain all exact Quest/MINT/custody guards.
CREATE OR REPLACE FUNCTION game_quest_reward_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE expected JSONB; actual BIGINT; account UUID; grant_key TEXT; addon INTEGER;
BEGIN
    IF coalesce(NEW.plan->>'claim_scope','once') NOT IN ('once','per_command')
        OR (NEW.plan->>'claim_scope'='per_command' AND
            (NEW.plan->'give'='null'::jsonb OR jsonb_array_length(NEW.plan->'take')=0)) THEN
        RAISE EXCEPTION 'repeatable quest reward requires custody-backed MINT' USING ERRCODE='23514';
    END IF;
    IF NEW.created_xact_id<>pg_current_xact_id() OR NEW.plan->>'key' IS DISTINCT FROM NEW.transition_key
        OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r
            WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.cause_id
                AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
                AND r.quest_key=NEW.plan->>'quest' AND r.cause_kind='command'
                AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'quest reward lacks same-transaction quest receipt' USING ERRCODE='23514';
    END IF;
    FOR expected IN SELECT value FROM jsonb_array_elements(NEW.plan->'take') LOOP
        SELECT coalesce(sum(c.quantity),0) INTO actual FROM game_quest_callback_consumptions c
            WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
                AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
                AND c.before_item->>'definition_production_key'=expected->>'item'
                AND c.location_kind IN('container','nested','backpack','equipment')
                AND c.replacement IS NULL AND c.created_xact_id=pg_current_xact_id();
        IF actual IS DISTINCT FROM (expected->>'count')::bigint THEN
            RAISE EXCEPTION 'quest reward must consume exact carried item count' USING ERRCODE='23514';
        END IF;
    END LOOP;
    IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions c
        WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
            AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
            AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') e
                WHERE e->>'item'=c.before_item->>'definition_production_key')) THEN
        RAISE EXCEPTION 'quest reward consumed an unauthored input' USING ERRCODE='23514';
    END IF;
    IF (NEW.plan->'give'='null'::jsonb) IS DISTINCT FROM (NEW.mint_transaction_id IS NULL)
        OR (NEW.mint_transaction_id IS NOT NULL AND NOT EXISTS(
            SELECT 1 FROM game_reward_claim_mint_receipts m JOIN game_item_instances i
                ON i.item_instance_id=m.item_instance_id
            WHERE m.transaction_id=NEW.mint_transaction_id AND m.character_id=NEW.character_id
                AND m.game_session_id=NEW.cause_id AND m.command_id=NEW.cause_ordinal
                AND m.claim_production_key=CASE WHEN NEW.plan->>'claim_scope'='per_command'
                    AND NEW.plan->'give'<>'null'::jsonb AND jsonb_array_length(NEW.plan->'take')>0
                    THEN NEW.transition_key || '/occurrence/' || replace(NEW.cause_id::text,'-','') || '-' || NEW.cause_ordinal::text
                    ELSE NEW.transition_key END AND m.created_xact_id=pg_current_xact_id()
                AND i.definition_production_key=NEW.plan #>> '{give,item}'
                AND m.quantity=(NEW.plan #>> '{give,count}')::bigint)) THEN
        RAISE EXCEPTION 'quest reward lacks exact native backpack MINT' USING ERRCODE='23514';
    END IF;
    SELECT account_id INTO STRICT account FROM game_character_roots WHERE character_id=NEW.character_id;
    -- Account unlocks can already exist from another Character. They still
    -- satisfy the reward, but must never be replaced with a second receipt.
    IF NEW.plan->'outfit'<>'null'::jsonb THEN
        FOR addon IN 0..2 LOOP
            IF addon<>0 AND ((NEW.plan #>> '{outfit,addons}')::integer & (1 << (addon-1)))=0 THEN
                CONTINUE;
            END IF;
            grant_key:=(NEW.plan #>> '{outfit,outfit}') || '/quest-grant/' || addon;
            IF NOT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u
                JOIN game_account_appearance_unlock_receipts r USING(grant_occurrence_id,account_id,unlock_key)
                WHERE u.account_id=account AND u.unlock_key=grant_key
                    AND r.source_kind='quest'
                    AND r.content_revision='crystal-9f5a72c-quest-rewards-v1') THEN
                RAISE EXCEPTION 'quest outfit lacks compatible earned fact' USING ERRCODE='23514';
            END IF;
        END LOOP;
    END IF;
    FOREACH grant_key IN ARRAY NEW.grant_keys LOOP
        IF NOT EXISTS(SELECT 1 FROM generate_series(0,2) AS a(addon)
            WHERE (a.addon=0 OR ((NEW.plan #>> '{outfit,addons}')::integer & (1 << (a.addon-1)))<>0)
                AND grant_key=(NEW.plan #>> '{outfit,outfit}') || '/quest-grant/' || a.addon) THEN
            RAISE EXCEPTION 'quest outfit has an unauthored grant key' USING ERRCODE='23514';
        END IF;
        IF NOT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u
            JOIN game_account_appearance_unlock_receipts r USING(grant_occurrence_id,account_id,unlock_key)
            WHERE u.account_id=account AND u.unlock_key=grant_key
                AND r.granting_character_id=NEW.character_id AND r.source_kind='quest'
                AND r.content_revision='crystal-9f5a72c-quest-rewards-v1'
                AND r.xmin=pg_current_xact_id()::text::xid) THEN
            RAISE EXCEPTION 'quest outfit lacks same-transaction earned fact' USING ERRCODE='23514';
        END IF;
    END LOOP;
    RETURN NULL;
END $$;
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_complete() FROM PUBLIC;
