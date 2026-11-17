-- T114 owner decision B 2026-11-09; Canary 47dfd51 taskboard/soulpit.lua:110-181.
-- A seal payment is a Task Board quest debit, never a fabricated Item craft.
CREATE TABLE game_soulseal_entries (
    transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
    handoff_id UUID NOT NULL UNIQUE REFERENCES game_soulpit_encounters(handoff_id) DEFERRABLE INITIALLY DEFERRED,
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
    intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 20),
    species_key TEXT NOT NULL,
    stars SMALLINT NOT NULL CHECK(stars BETWEEN 0 AND 5),
    cost SMALLINT NOT NULL CHECK(cost=(stars+1)*10),
    balance_before BIGINT NOT NULL CHECK(balance_before>=cost),
    balance_after BIGINT NOT NULL CHECK(balance_after=balance_before-cost),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(game_session_id,command_id),
    FOREIGN KEY(character_id,game_session_id,command_id)
        REFERENCES game_task_board_receipts(character_id,cause_id,cause_ordinal) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_soulseal_entries_character ON game_soulseal_entries(character_id);
CREATE INDEX game_soulseal_entries_world ON game_soulseal_entries(world_id);
ALTER TABLE game_soulpit_encounters ALTER COLUMN source_transaction_id DROP NOT NULL;
ALTER TABLE game_soulpit_encounters ADD COLUMN seal_transaction_id UUID UNIQUE
    REFERENCES game_soulseal_entries(transaction_id) DEFERRABLE INITIALLY DEFERRED;
ALTER TABLE game_soulpit_encounters ADD CONSTRAINT game_soulpit_one_funding
    CHECK(num_nonnulls(source_transaction_id,seal_transaction_id)=1);

CREATE FUNCTION game_soulseal_entry_consistent() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
        SELECT 1 FROM game_soulpit_encounters f
        JOIN game_activity_handoff_groups g USING(handoff_id)
        JOIN game_soulpit_participants p USING(handoff_id)
        JOIN game_activity_handoff_participants a ON a.handoff_id=p.handoff_id AND a.character_id=p.character_id
        JOIN game_task_board_receipts r ON r.character_id=NEW.character_id AND r.cause_id=NEW.game_session_id AND r.cause_ordinal=NEW.command_id
        JOIN game_character_quest_receipts q ON q.character_id=r.character_id AND q.cause_id=r.cause_id AND q.cause_ordinal=r.cause_ordinal AND q.cause_kind='use'
        WHERE f.handoff_id=NEW.handoff_id AND f.seal_transaction_id=NEW.transaction_id
          AND f.source_transaction_id IS NULL AND f.species_key=NEW.species_key
          AND f.created_xact_id=NEW.created_xact_id AND g.state=2 AND g.participant_count=1
          AND g.world_id=NEW.world_id AND p.character_id=NEW.character_id AND p.staging_slot=0
          AND a.source_game_session_id=NEW.game_session_id
          AND r.world_id=NEW.world_id AND r.cause_kind='command' AND r.refusal='null'::jsonb
          AND r.created_xact_id=NEW.created_xact_id
          AND r.action=jsonb_build_object('SoulsealFight',jsonb_build_object('species',NEW.species_key,'cost',NEW.cost))
          AND q.values_before[array_position(q.track_keys,'oteryn:taskboard/weekly/soulseals')]=NEW.balance_before
          AND q.values_after[array_position(q.track_keys,'oteryn:taskboard/weekly/soulseals')]=NEW.balance_after
          AND (r.reply->'balances'->>'soulseals')::bigint=NEW.balance_after
    ) THEN RAISE EXCEPTION 'Soulseal payment requires one atomic fenced solo handoff and quest debit' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;

-- Preserve the core proof; add the alternative seal proof, never an unproved entry.
CREATE OR REPLACE FUNCTION game_soulpit_entry_consistent() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE fight game_soulpit_encounters%ROWTYPE; expected_count INT;
BEGIN
    SELECT * INTO STRICT fight FROM game_soulpit_encounters WHERE handoff_id=NEW.handoff_id;
    SELECT participant_count INTO expected_count FROM game_activity_handoff_groups WHERE handoff_id=fight.handoff_id AND state=2;
    IF expected_count IS NULL OR expected_count NOT BETWEEN 1 AND 5
        OR NOT EXISTS (SELECT 1 FROM game_soulpit_entry_handoff_commits c WHERE c.handoff_id=fight.handoff_id AND c.committed_xact_id=fight.created_xact_id)
        OR (SELECT count(*) FROM game_soulpit_participants WHERE handoff_id=fight.handoff_id)<>expected_count
        OR NOT (
            (fight.seal_transaction_id IS NULL AND EXISTS (SELECT 1 FROM game_item_craft_records r
                JOIN game_activity_handoff_participants p ON p.handoff_id=fight.handoff_id AND p.character_id=r.character_id AND p.source_game_session_id=r.game_session_id
                WHERE r.transaction_id=fight.source_transaction_id AND r.recipe='oteryn:soulpit.entry'
                  AND r.created_xact_id=fight.created_xact_id AND r.world_id=p.world_id AND jsonb_array_length(r.outputs)=0))
            OR (fight.source_transaction_id IS NULL AND expected_count=1 AND EXISTS (SELECT 1 FROM game_soulseal_entries s
                WHERE s.transaction_id=fight.seal_transaction_id AND s.handoff_id=fight.handoff_id AND s.species_key=fight.species_key AND s.created_xact_id=fight.created_xact_id))
        )
    THEN RAISE EXCEPTION 'Soulpit debit and committed shared handoff must be atomic' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_soulseal_entry_consistent AFTER INSERT ON game_soulseal_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_soulseal_entry_consistent();
CREATE TRIGGER game_soulseal_entry_immutable BEFORE UPDATE OR DELETE ON game_soulseal_entries
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_soulseal_entry_no_truncate BEFORE TRUNCATE ON game_soulseal_entries
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
REVOKE ALL ON game_soulseal_entries FROM PUBLIC;
GRANT SELECT,INSERT ON game_soulseal_entries TO oteryn_game_runtime;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_soulseal_entry_consistent() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_soulpit_entry_consistent() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_soulseal_entry_consistent() FROM PUBLIC;
