-- W10: Primal Menace contributors unlock one Hazard level, inside the native kill transaction.
-- Crystal 201189f creaturescripts_the_primal_menace_killed.lua:3-18; hazard.lua:116-126.
-- Add an origin stamp to the existing death owner so a historic death can never be
-- reinterpreted as a new progression grant after later level selections.
ALTER TABLE game_party_kill_xp_receipts
    ADD COLUMN IF NOT EXISTS created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id();
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid='game_party_kill_xp_receipts'::regclass
                   AND tgname='game_party_kill_xp_origin_stamp') THEN
        CREATE TRIGGER game_party_kill_xp_origin_stamp BEFORE INSERT ON game_party_kill_xp_receipts
            FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS game_character_primal_hazard_receipts (
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    death_key bytea NOT NULL REFERENCES game_party_kill_xp_receipts(death_key),
    contributors bytea NOT NULL CHECK (octet_length(contributors) BETWEEN 16 AND 80
                                       AND octet_length(contributors)%16=0),
    group_minimum smallint NOT NULL CHECK (group_minimum BETWEEN 1 AND 12),
    unlocked_before smallint NOT NULL CHECK (unlocked_before BETWEEN 1 AND 12),
    unlocked_after smallint NOT NULL CHECK (unlocked_after BETWEEN unlocked_before AND 12),
    selected smallint NOT NULL CHECK (selected BETWEEN 1 AND unlocked_before),
    result bytea NOT NULL CHECK (octet_length(result)=2
        AND get_byte(result,0)=unlocked_after AND get_byte(result,1)=selected),
    command_session_id uuid NOT NULL,
    connection_generation numeric(20,0) NOT NULL CHECK (connection_generation BETWEEN 1 AND 18446744073709551615),
    character_lease_generation numeric(20,0) NOT NULL CHECK (character_lease_generation BETWEEN 1 AND 18446744073709551615),
    scope_ownership_generation numeric(20,0) NOT NULL CHECK (scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    original_character_revision numeric(20,0) NOT NULL CHECK (original_character_revision BETWEEN 1 AND 18446744073709551615),
    committed_character_revision numeric(20,0) CHECK (committed_character_revision=original_character_revision+1
                                                     AND committed_character_revision<=18446744073709551615),
    level_before bigint NOT NULL, level_after bigint NOT NULL CHECK (level_after=level_before),
    experience_before bigint NOT NULL, experience_after bigint NOT NULL CHECK (experience_after=experience_before),
    profile_revision text NOT NULL, ruleset_revision text NOT NULL, content_revision text NOT NULL,
    simulation_revision text NOT NULL, evidence_revision text NOT NULL,
    declaration_revision text NOT NULL, policy_revision text NOT NULL, reward_revision text NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id,death_key),
    UNIQUE (character_id,committed_character_revision),
    CHECK (unlocked_after=CASE WHEN selected=unlocked_before AND selected=group_minimum
                             THEN LEAST(12,unlocked_before+1) ELSE unlocked_before END),
    CHECK ((committed_character_revision IS NOT NULL)=(unlocked_after<>unlocked_before))
);
CREATE INDEX IF NOT EXISTS game_primal_hazard_death ON game_character_primal_hazard_receipts(death_key);

-- Keep every branch of 0337/0534/0539: their latest shared view is the parent.
-- Descendant names retain the historical chain prefix checked by re-runnable
-- migrations 0332/0337/0534. Replace only the complete relation name below,
-- never that prefix in an already-installed descendant.
CREATE OR REPLACE VIEW game_character_hazard_revision_chain_primal WITH (security_invoker=true) AS
 SELECT * FROM game_character_hazard_revision_chain
 UNION ALL
 SELECT character_id,original_character_revision,committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
 content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_primal_hazard_receipts WHERE committed_character_revision IS NOT NULL;
CREATE OR REPLACE VIEW game_character_hazard_transition_receipts WITH (security_invoker=true) AS
 SELECT character_id,committed_character_revision,result,created_xact_id FROM game_character_hazard_receipts
 UNION ALL
 SELECT character_id,committed_character_revision,result,created_xact_id FROM game_character_primal_hazard_receipts;
DO $$ DECLARE body text; BEGIN
    SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure) INTO STRICT body;
    IF position('game_character_hazard_revision_chain ' IN body)>0 THEN
        EXECUTE replace(body,'game_character_hazard_revision_chain ','game_character_hazard_revision_chain_primal ');
    END IF; -- Reapplying this migration never rewinds a later additive chain wrapper.
    SELECT pg_get_functiondef('game_hazard_state_guard()'::regprocedure) INTO STRICT body;
    IF position('game_character_hazard_receipts' IN body)>0 THEN
        EXECUTE replace(body,'game_character_hazard_receipts','game_character_hazard_transition_receipts');
    END IF;
END $$;

CREATE OR REPLACE FUNCTION game_primal_hazard_receipt_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE member uuid; previous uuid; points smallint; minimum smallint:=12; own_member boolean:=false;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_party_kill_xp_receipts d WHERE d.death_key=NEW.death_key
        AND d.creature_key='oteryn:creature.the_primal_menace' AND d.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Primal unlock needs this transaction native death receipt' USING ERRCODE='23514';
    END IF;
    FOR idx IN 0..octet_length(NEW.contributors)/16-1 LOOP
        member:=encode(substring(NEW.contributors FROM idx*16+1 FOR 16),'hex')::uuid;
        IF previous IS NOT NULL AND member<=previous THEN
            RAISE EXCEPTION 'Primal contributors must be distinct and canonical' USING ERRCODE='23514';
        END IF;
        previous:=member;
        own_member:=own_member OR member=NEW.character_id;
        SELECT h.selected INTO points FROM game_character_hazard h
            JOIN game_party_kill_xp_members m USING(character_id)
            WHERE m.death_key=NEW.death_key AND h.character_id=member;
        IF points IS NULL THEN
            RAISE EXCEPTION 'Primal contributor has no native kill member or Hazard state' USING ERRCODE='23514';
        END IF;
        minimum:=LEAST(minimum,points);
    END LOOP;
    IF NOT own_member OR minimum<>NEW.group_minimum OR NOT EXISTS (
        SELECT 1 FROM game_character_roots r JOIN game_character_hazard h USING(character_id)
        WHERE r.character_id=NEW.character_id AND r.character_revision=NEW.original_character_revision
            AND h.unlocked=NEW.unlocked_before AND h.selected=NEW.selected
    ) THEN
        RAISE EXCEPTION 'Primal unlock disagrees with contributor predecessor' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION game_primal_hazard_receipt_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE count_members integer;
BEGIN
    SELECT count(*) INTO count_members FROM game_character_primal_hazard_receipts r
        WHERE r.death_key=NEW.death_key AND r.contributors=NEW.contributors
            AND r.group_minimum=NEW.group_minimum AND r.created_xact_id=pg_current_xact_id();
    IF count_members<>octet_length(NEW.contributors)/16 OR NOT EXISTS (
        SELECT 1 FROM game_character_roots r JOIN game_character_hazard h USING(character_id)
        WHERE r.character_id=NEW.character_id
            AND r.character_revision=COALESCE(NEW.committed_character_revision,NEW.original_character_revision)
            AND h.unlocked=NEW.unlocked_after AND h.selected=NEW.selected
    ) THEN
        RAISE EXCEPTION 'Primal unlock must retain all contributors and its exact state' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_primal_hazard_receipt_admit() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_primal_hazard_receipt_consistent() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_primal_hazard_receipts'::regclass
                   AND tgname='game_primal_hazard_receipt_0_stamp') THEN
        CREATE TRIGGER game_primal_hazard_receipt_0_stamp BEFORE INSERT ON game_character_primal_hazard_receipts
            FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
        CREATE TRIGGER game_primal_hazard_receipt_admit BEFORE INSERT ON game_character_primal_hazard_receipts
            FOR EACH ROW EXECUTE FUNCTION game_primal_hazard_receipt_admit();
        CREATE TRIGGER game_primal_hazard_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_primal_hazard_receipts
            FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
        CREATE TRIGGER game_primal_hazard_receipt_no_truncate BEFORE TRUNCATE ON game_character_primal_hazard_receipts
            EXECUTE FUNCTION game_character_reject_truncate();
        CREATE CONSTRAINT TRIGGER game_primal_hazard_character_chain AFTER INSERT ON game_character_primal_hazard_receipts
            DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
        CREATE CONSTRAINT TRIGGER game_primal_hazard_receipt_consistent AFTER INSERT ON game_character_primal_hazard_receipts
            DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_primal_hazard_receipt_consistent();
    END IF;
END $$;
REVOKE ALL ON game_character_primal_hazard_receipts,game_character_hazard_revision_chain_primal,
    game_character_hazard_transition_receipts FROM PUBLIC,oteryn_game_control;
GRANT SELECT,INSERT ON game_character_primal_hazard_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_hazard_revision_chain_primal,game_character_hazard_transition_receipts TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_primal_hazard_receipt_admit(),game_primal_hazard_receipt_consistent() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_primal_hazard_receipt_admit(),game_primal_hazard_receipt_consistent() TO oteryn_game_runtime;
