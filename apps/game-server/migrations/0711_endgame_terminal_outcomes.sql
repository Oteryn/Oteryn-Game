-- W11: retained group decisions consumed only after the sealed shared-owner proof.
-- PR2784 ANSWER6547488765/6547661530: terminal unsuccessful outcome, one distinct
-- pair for the affected encounter participants. Individual deaths/disconnects never enter here.
-- ANSWER6547994219 retains the full W02 roster and explicit per-Character expiry
-- no-ops; only still-complete, nonexpired Soul War participants consume the pair.
-- W02's retained terminal authority is composed separately; this is not an encounter engine.
CREATE TABLE IF NOT EXISTS game_endgame_terminal_outcomes (
    attempt bytea PRIMARY KEY CHECK(octet_length(attempt)=32 AND attempt<>decode(repeat('00',32),'hex')),
    binding bytea NOT NULL CHECK(octet_length(binding)=32),
    encounter_kind smallint NOT NULL CHECK(encounter_kind IN(1,2)),
    world_id uuid NOT NULL,
    scope_kind smallint NOT NULL CHECK(scope_kind IN(1,2)),
    scope_id uuid NOT NULL,
    scope_ownership_generation numeric(20,0) NOT NULL CHECK(scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    participants bytea NOT NULL CHECK(octet_length(participants) BETWEEN 16 AND 80 AND octet_length(participants)%16=0),
    affected_participants bytea NOT NULL CHECK(octet_length(affected_participants)<=octet_length(participants) AND octet_length(affected_participants)%16=0),
    expired_participants bytea NOT NULL CHECK(octet_length(expired_participants)<=octet_length(participants) AND octet_length(expired_participants)%16=0),
    common_completed_flags smallint NOT NULL CHECK(common_completed_flags BETWEEN 0 AND 31),
    removed_flags smallint CHECK(removed_flags BETWEEN 1 AND 31 AND bit_count(removed_flags::integer::bit(5))=2),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK(encounter_kind=1 OR common_completed_flags<=15),
    CHECK(encounter_kind=1 OR octet_length(expired_participants)=0),
    CHECK((removed_flags IS NOT NULL)=(octet_length(affected_participants)>0)),
    CHECK(removed_flags IS NULL OR (removed_flags & common_completed_flags)=removed_flags),
    CHECK((removed_flags IS NOT NULL)=(CASE WHEN encounter_kind=1 THEN common_completed_flags=31
                                         ELSE bit_count(common_completed_flags::integer::bit(5))>=2 END))
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='game_character_soul_war_receipts'::regclass AND conname='game_soul_war_terminal_attempt') THEN
        ALTER TABLE game_character_soul_war_receipts ADD CONSTRAINT game_soul_war_terminal_attempt
            FOREIGN KEY(source_attempt) REFERENCES game_endgame_terminal_outcomes(attempt);
    END IF;
END $$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='game_character_rotten_blood_receipts'::regclass AND conname='game_rotten_blood_terminal_attempt') THEN
        ALTER TABLE game_character_rotten_blood_receipts ADD CONSTRAINT game_rotten_blood_terminal_attempt
            FOREIGN KEY(source_terminal_attempt) REFERENCES game_endgame_terminal_outcomes(attempt);
    END IF;
END $$;
CREATE INDEX IF NOT EXISTS game_rotten_blood_terminal_descendants ON game_character_rotten_blood_receipts(source_terminal_attempt) WHERE source_terminal_attempt IS NOT NULL;

CREATE OR REPLACE FUNCTION game_endgame_terminal_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE member uuid; previous uuid; points smallint; common smallint; member_world uuid;
    first_taint bigint; affected bytea:=decode('','hex'); expired bytea:=decode('','hex');
BEGIN
    common:=CASE WHEN NEW.encounter_kind=1 THEN 0 ELSE 15 END;
    FOR idx IN 0..octet_length(NEW.participants)/16-1 LOOP
        member:=encode(substring(NEW.participants FROM idx*16+1 FOR 16),'hex')::uuid;
        IF previous IS NOT NULL AND member<=previous THEN
            RAISE EXCEPTION 'terminal roster must be distinct and canonical' USING ERRCODE='23514';
        END IF;
        previous:=member;
        SELECT world_id INTO member_world FROM game_character_roots WHERE character_id=member;
        IF member_world IS DISTINCT FROM NEW.world_id THEN
            RAISE EXCEPTION 'terminal roster crosses worlds or contains an absent Character' USING ERRCODE='23514';
        END IF;
        IF NEW.encounter_kind=1 THEN
            -- Existing Soul War lifetime; no renewal of its first-taint clock.
            -- The canonical expired subset is the immutable no-op/expiry result
            -- for each expired Character, including when the entire roster expired.
            SELECT defeated_bosses,first_taint_at INTO points,first_taint
                FROM game_character_soul_war WHERE character_id=member;
            IF first_taint IS NOT NULL AND
                floor(extract(epoch FROM transaction_timestamp()))::numeric-first_taint>=1209600 THEN
                expired:=expired || uuid_send(member);
                points:=0;
            END IF;
            IF COALESCE(points,0)=31 THEN
                affected:=affected || uuid_send(member);
                common:=31;
            END IF;
        ELSE
            SELECT child_flags INTO points FROM game_character_rotten_blood WHERE character_id=member;
            common:=common & COALESCE(points,0);
        END IF;
    END LOOP;
    IF NEW.encounter_kind=2 AND bit_count(common::integer::bit(5))>=2 THEN
        affected:=NEW.participants;
    END IF;
    IF common<>NEW.common_completed_flags OR affected<>NEW.affected_participants OR expired<>NEW.expired_participants THEN
        RAISE EXCEPTION 'terminal pair or participant dispositions have no current completed flags' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION game_soul_war_terminal_source_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE decision game_endgame_terminal_outcomes%ROWTYPE; prior game_character_soul_war%ROWTYPE;
    binding jsonb; removed integer;
BEGIN
    binding:=convert_from(NEW.request_binding,'UTF8')::jsonb;
    IF COALESCE((binding->1) ? 'FailedMegalomania',false) IS DISTINCT FROM (NEW.source_attempt IS NOT NULL) THEN
        RAISE EXCEPTION 'Soul War failed final requires its terminal source' USING ERRCODE='23514';
    END IF;
    IF NEW.source_attempt IS NULL THEN RETURN NEW; END IF;
    SELECT * INTO decision FROM game_endgame_terminal_outcomes WHERE attempt=NEW.source_attempt;
    removed:=(binding#>>'{1,FailedMegalomania,removed_bosses}')::integer;
    IF decision.encounter_kind IS DISTINCT FROM 1 OR decision.removed_flags IS DISTINCT FROM removed
        OR decision.created_xact_id IS DISTINCT FROM pg_current_xact_id()
        OR NOT EXISTS(SELECT 1 FROM generate_series(0,octet_length(decision.affected_participants)/16-1) idx
            WHERE substring(decision.affected_participants FROM idx*16+1 FOR 16)=uuid_send(NEW.character_id)) THEN
        RAISE EXCEPTION 'Soul War failed final is not this group terminal decision' USING ERRCODE='23514';
    END IF;
    SELECT * INTO prior FROM game_character_soul_war WHERE character_id=NEW.character_id;
    IF prior.defeated_bosses IS DISTINCT FROM 31
        OR NEW.defeated_bosses IS DISTINCT FROM (prior.defeated_bosses & ~removed)
        OR NEW.first_taint_at IS DISTINCT FROM prior.first_taint_at
        OR NEW.teleport_access IS DISTINCT FROM prior.teleport_access
        OR NEW.megalomania_completed IS DISTINCT FROM prior.megalomania_completed
        OR NEW.final_reward_claimed IS DISTINCT FROM prior.final_reward_claimed
        OR NEW.committed_character_revision IS NULL THEN
        RAISE EXCEPTION 'Soul War failed final must retain the shared two-flag successor' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION game_rotten_blood_terminal_source_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE decision game_endgame_terminal_outcomes%ROWTYPE; removed integer;
BEGIN
    IF NEW.kind<>5 THEN RETURN NEW; END IF;
    SELECT * INTO decision FROM game_endgame_terminal_outcomes WHERE attempt=NEW.source_terminal_attempt;
    removed:=(NEW.before_state->>'child_flags')::integer & ~(NEW.result#>>'{state,child_flags}')::integer;
    IF decision.encounter_kind IS DISTINCT FROM 2 OR decision.removed_flags IS DISTINCT FROM removed
        OR decision.created_xact_id IS DISTINCT FROM pg_current_xact_id()
        OR NEW.committed_character_revision IS NULL
        OR NOT EXISTS(SELECT 1 FROM generate_series(0,octet_length(decision.affected_participants)/16-1) idx
            WHERE substring(decision.affected_participants FROM idx*16+1 FOR 16)=uuid_send(NEW.character_id)) THEN
        RAISE EXCEPTION 'Rotten Blood failed final is not this group terminal decision' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION game_endgame_terminal_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE actual integer; expected integer;
BEGIN
    expected:=octet_length(NEW.affected_participants)/16;
    IF NEW.encounter_kind=1 THEN
        SELECT count(*) INTO actual FROM game_character_soul_war_receipts
            WHERE source_attempt=NEW.attempt AND created_xact_id=pg_current_xact_id();
    ELSE
        SELECT count(*) INTO actual FROM game_character_rotten_blood_receipts
            WHERE source_terminal_attempt=NEW.attempt AND created_xact_id=pg_current_xact_id();
    END IF;
    IF actual<>expected THEN
        RAISE EXCEPTION 'terminal outcome and every affected participant reset must commit together' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_endgame_terminal_admit() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_soul_war_terminal_source_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_rotten_blood_terminal_source_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_endgame_terminal_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_endgame_terminal_outcomes'::regclass AND tgname='game_endgame_terminal_0_stamp') THEN
        CREATE TRIGGER game_endgame_terminal_0_stamp BEFORE INSERT ON game_endgame_terminal_outcomes
            FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
    END IF;
END $$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_endgame_terminal_outcomes'::regclass AND tgname='game_endgame_terminal_admit') THEN
        CREATE TRIGGER game_endgame_terminal_admit BEFORE INSERT ON game_endgame_terminal_outcomes
            FOR EACH ROW EXECUTE FUNCTION game_endgame_terminal_admit();
    END IF;
END $$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_endgame_terminal_outcomes'::regclass AND tgname='game_endgame_terminal_immutable') THEN
        CREATE TRIGGER game_endgame_terminal_immutable BEFORE UPDATE OR DELETE ON game_endgame_terminal_outcomes
            FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
    END IF;
END $$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_endgame_terminal_outcomes'::regclass AND tgname='game_endgame_terminal_no_truncate') THEN
        CREATE TRIGGER game_endgame_terminal_no_truncate BEFORE TRUNCATE ON game_endgame_terminal_outcomes
            EXECUTE FUNCTION game_character_reject_truncate();
    END IF;
END $$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_endgame_terminal_outcomes'::regclass AND tgname='game_endgame_terminal_complete') THEN
        CREATE CONSTRAINT TRIGGER game_endgame_terminal_complete AFTER INSERT ON game_endgame_terminal_outcomes
            DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_endgame_terminal_complete();
    END IF;
END $$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_soul_war_receipts'::regclass AND tgname='game_soul_war_terminal_source') THEN
        CREATE TRIGGER game_soul_war_terminal_source BEFORE INSERT ON game_character_soul_war_receipts
            FOR EACH ROW EXECUTE FUNCTION game_soul_war_terminal_source_guard();
    END IF;
END $$;
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_rotten_blood_receipts'::regclass AND tgname='game_rotten_blood_terminal_source') THEN
        CREATE TRIGGER game_rotten_blood_terminal_source BEFORE INSERT ON game_character_rotten_blood_receipts
            FOR EACH ROW EXECUTE FUNCTION game_rotten_blood_terminal_source_guard();
    END IF;
END $$;
REVOKE ALL ON game_endgame_terminal_outcomes FROM PUBLIC,oteryn_game_control;
GRANT SELECT,INSERT ON game_endgame_terminal_outcomes TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_endgame_terminal_admit(),game_soul_war_terminal_source_guard(),
    game_rotten_blood_terminal_source_guard(),game_endgame_terminal_complete() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_endgame_terminal_admit(),game_soul_war_terminal_source_guard(),
    game_rotten_blood_terminal_source_guard(),game_endgame_terminal_complete() TO oteryn_game_runtime;
