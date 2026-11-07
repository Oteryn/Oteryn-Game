-- Closed owner interface: Source Snapshot.proof seals hazard_state
-- {unlocked,selected,received_prize} and selected_integer. Ordinary source-mint
-- completeness remains enforced by the parent's generic mint child owner.
CREATE TABLE game_npc_source_hazard_effects (
 npc_transaction_id UUID PRIMARY KEY REFERENCES game_npc_cosmetic_receipts(transaction_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL,
 source_key TEXT NOT NULL CHECK(source_key IN('oteryn:npc-cosmetic/gnomadness/ripptor-prize','oteryn:npc-cosmetic/gnomadness/ripptor-select')),
 program_sha256 BYTEA NOT NULL CHECK(octet_length(program_sha256)=32),
 committed_character_revision NUMERIC(20,0) NOT NULL,
 unlocked SMALLINT NOT NULL CHECK(unlocked BETWEEN 1 AND 12),
 selected_before SMALLINT NOT NULL CHECK(selected_before BETWEEN 1 AND unlocked),
 selected_after SMALLINT NOT NULL CHECK(selected_after BETWEEN 1 AND unlocked),
 result BYTEA NOT NULL CHECK(octet_length(result)=2 AND get_byte(result,0)=unlocked AND get_byte(result,1)=selected_after),
 prize BOOLEAN NOT NULL,created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id)
);
CREATE INDEX game_npc_source_hazard_effects_character ON game_npc_source_hazard_effects(character_id);
CREATE TABLE game_npc_primal_prize_claims (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 npc_transaction_id UUID NOT NULL UNIQUE REFERENCES game_npc_source_hazard_effects(npc_transaction_id),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_npc_source_hazard_effects,game_npc_primal_prize_claims FROM PUBLIC;
GRANT SELECT,INSERT ON game_npc_source_hazard_effects,game_npc_primal_prize_claims TO oteryn_game_runtime;
CREATE FUNCTION game_npc_source_hazard_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_npc_source_context%ROWTYPE;r game_npc_cosmetic_receipts%ROWTYPE;u SMALLINT;s SMALLINT;BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'immutable NPC Hazard child';END IF;
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 SELECT * INTO r FROM game_npc_cosmetic_receipts WHERE transaction_id=NEW.npc_transaction_id;
 SELECT unlocked,selected INTO u,s FROM game_character_hazard WHERE character_id=NEW.character_id FOR UPDATE;
 IF c.backend_pid IS NULL OR c.captured IS DISTINCT FROM TRUE OR c.npc_key<>'oteryn:npc.gnomadness'
  OR NEW.npc_transaction_id IS DISTINCT FROM c.npc_transaction_id OR NEW.character_id IS DISTINCT FROM c.character_id
  OR NEW.game_session_id IS DISTINCT FROM c.game_session_id OR NEW.command_id IS DISTINCT FROM c.command_id
  OR NEW.source_key IS DISTINCT FROM c.source_key OR NEW.program_sha256 IS DISTINCT FROM c.program_sha256
  OR NEW.committed_character_revision IS DISTINCT FROM c.original_revision+1
  OR r.character_id IS DISTINCT FROM NEW.character_id OR r.game_session_id IS DISTINCT FROM NEW.game_session_id
  OR r.command_id IS DISTINCT FROM NEW.command_id OR r.source_key IS DISTINCT FROM NEW.source_key
  OR r.program_sha256 IS DISTINCT FROM NEW.program_sha256 OR r.created_xact_id IS DISTINCT FROM pg_current_xact_id()
  OR r.committed_character_revision IS DISTINCT FROM NEW.committed_character_revision
  OR NEW.unlocked IS DISTINCT FROM u OR NEW.selected_before IS DISTINCT FROM s
  OR NEW.unlocked IS DISTINCT FROM (c.before_state->'hazard_state'->>'unlocked')::smallint
  OR NEW.selected_before IS DISTINCT FROM (c.before_state->'hazard_state'->>'selected')::smallint
  OR NEW.selected_after IS DISTINCT FROM (c.before_state->>'selected_integer')::smallint
  OR NEW.prize IS DISTINCT FROM (NEW.source_key='oteryn:npc-cosmetic/gnomadness/ripptor-prize')
  OR (NEW.prize AND (NEW.selected_after<6 OR (c.before_state->'hazard_state'->>'received_prize')::boolean IS DISTINCT FROM FALSE
     OR EXISTS(SELECT 1 FROM game_npc_primal_prize_claims WHERE character_id=NEW.character_id)))
  OR NOT ((c.expected_plan->'effects') @> '[{"kind":"set_hazard_level","owner":"hazard.gnomprona-gardens","selection":"hazard_level"}]'::jsonb)
  OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=NEW.character_id AND character_revision=NEW.committed_character_revision)
 THEN RAISE EXCEPTION 'NPC Hazard source/Node/before/selection proof mismatch';END IF;
 NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_npc_source_hazard_admit BEFORE INSERT OR UPDATE OR DELETE ON game_npc_source_hazard_effects FOR EACH ROW EXECUTE FUNCTION game_npc_source_hazard_admit();
CREATE FUNCTION game_npc_primal_claim_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'immutable primal prize';END IF;
 IF NOT EXISTS(SELECT 1 FROM game_npc_source_hazard_effects h JOIN game_npc_source_context c ON c.npc_transaction_id=h.npc_transaction_id
 WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id() AND h.prize
 AND h.created_xact_id=pg_current_xact_id() AND h.character_id=NEW.character_id AND h.npc_transaction_id=NEW.npc_transaction_id)
 THEN RAISE EXCEPTION 'primal prize lacks exact NPC child';END IF;
 NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_npc_primal_claim_admit BEFORE INSERT OR UPDATE OR DELETE ON game_npc_primal_prize_claims FOR EACH ROW EXECUTE FUNCTION game_npc_primal_claim_admit();
CREATE FUNCTION game_npc_source_hazard_selected_proof(p_character UUID,p_unlocked SMALLINT,p_before SMALLINT,p_after SMALLINT) RETURNS BOOLEAN LANGUAGE SQL SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_source_hazard_effects h JOIN game_npc_cosmetic_receipts r ON r.transaction_id=h.npc_transaction_id
 JOIN game_npc_source_context c ON c.npc_transaction_id=h.npc_transaction_id JOIN game_character_roots root ON root.character_id=h.character_id
 WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id() AND c.captured
 AND h.character_id=p_character AND h.unlocked=p_unlocked AND h.selected_before=p_before AND h.selected_after=p_after
 AND h.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id() AND root.character_revision=h.committed_character_revision)
$$;
REVOKE ALL ON FUNCTION game_npc_source_hazard_selected_proof(UUID,SMALLINT,SMALLINT,SMALLINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_source_hazard_selected_proof(UUID,SMALLINT,SMALLINT,SMALLINT) TO oteryn_game_runtime;
DO $patch$ DECLARE body TEXT;BEGIN
 body:=pg_get_functiondef('game_hazard_state_guard()'::regprocedure);
 IF strpos(body,'game_npc_source_hazard_selected_proof')<>0 THEN RAISE EXCEPTION 'NPC Hazard branch already present';END IF;
 IF strpos(body,'BEGIN')=0 THEN RAISE EXCEPTION 'Hazard owner boundary changed';END IF;
 EXECUTE regexp_replace(body,'BEGIN', $code$BEGIN
 IF TG_OP='UPDATE' AND NEW.unlocked=OLD.unlocked AND game_npc_source_hazard_selected_proof(NEW.character_id,NEW.unlocked,OLD.selected,NEW.selected) THEN RETURN NULL;END IF;
 $code$);
END $patch$;
CREATE FUNCTION game_npc_source_hazard_complete() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE parent game_npc_cosmetic_receipts%ROWTYPE;
BEGIN
 SELECT * INTO parent FROM game_npc_cosmetic_receipts WHERE transaction_id=NEW.npc_transaction_id;
 IF game_npc_source_parent_current(NEW.npc_transaction_id,NEW.source_key) IS DISTINCT FROM TRUE
 OR NOT EXISTS(SELECT 1 FROM game_character_hazard WHERE character_id=NEW.character_id AND unlocked=NEW.unlocked AND selected=NEW.selected_after)
 OR (NEW.prize AND (
  NOT EXISTS(SELECT 1 FROM game_npc_primal_prize_claims WHERE character_id=NEW.character_id AND npc_transaction_id=NEW.npc_transaction_id AND created_xact_id=NEW.created_xact_id)
  OR game_npc_source_output_complete(NEW.npc_transaction_id) IS DISTINCT FROM TRUE
  OR NOT EXISTS(SELECT 1 FROM game_account_appearance_unlocks fact
   JOIN game_account_appearance_unlock_receipts earned ON earned.grant_occurrence_id=fact.grant_occurrence_id
    AND earned.account_id=fact.account_id AND earned.unlock_key=fact.unlock_key
   JOIN game_npc_cosmetic_receipts origin ON origin.account_id=fact.account_id
    AND origin.character_id=earned.granting_character_id AND uuid_send(origin.transaction_id)=earned.source_event_id
    AND fact.unlock_key=ANY(origin.grant_keys)
   WHERE fact.account_id=parent.account_id
    AND fact.unlock_key='oteryn:appearance-grant/npc/gnomadness/noxious_ripptor/mount'
    AND earned.source_kind='npc' AND earned.content_revision='crystal-201189f-npc-cosmetics-v1'
    AND origin.source_key='oteryn:npc-cosmetic/gnomadness/ripptor-prize')
 )) THEN RAISE EXCEPTION 'NPC Hazard child lacks exact state/prize/MINT/earned mount' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_source_hazard_complete AFTER INSERT ON game_npc_source_hazard_effects DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_hazard_complete();
CREATE FUNCTION game_npc_source_hazard_child_required() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.source_key IN('oteryn:npc-cosmetic/gnomadness/ripptor-prize','oteryn:npc-cosmetic/gnomadness/ripptor-select') AND NOT EXISTS(SELECT 1 FROM game_npc_source_hazard_effects WHERE npc_transaction_id=NEW.transaction_id AND created_xact_id=NEW.created_xact_id)
 THEN RAISE EXCEPTION 'Gnomadness parent requires same-XID Hazard child';END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_source_hazard_child_required AFTER INSERT ON game_npc_cosmetic_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_hazard_child_required();

-- Recheck a current child after any later state mutation, including an early
-- SET CONSTRAINTS boundary. Ordinary Hazard updates outside this source TX
-- retain their existing complete proof unchanged.
CREATE FUNCTION game_npc_source_hazard_successor() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE source_character UUID;child game_npc_source_hazard_effects%ROWTYPE;
BEGIN
 source_character:=CASE WHEN TG_OP='DELETE' THEN OLD.character_id ELSE NEW.character_id END;
 FOR child IN SELECT * FROM game_npc_source_hazard_effects WHERE character_id=source_character AND created_xact_id=pg_current_xact_id() LOOP
  IF NOT EXISTS(SELECT 1 FROM game_character_hazard actual WHERE actual.character_id=child.character_id AND actual.unlocked=child.unlocked AND actual.selected=child.selected_after)
   OR game_npc_source_parent_current(child.npc_transaction_id,child.source_key) IS DISTINCT FROM TRUE THEN
   RAISE EXCEPTION 'NPC Hazard state lost its exact source successor' USING ERRCODE='23514';END IF;
 END LOOP;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_source_hazard_successor AFTER INSERT OR UPDATE OR DELETE ON game_character_hazard
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_hazard_successor();
DO $paths$ DECLARE signature TEXT;BEGIN
 FOREACH signature IN ARRAY ARRAY['game_npc_source_hazard_admit()','game_npc_primal_claim_admit()',
  'game_npc_source_hazard_selected_proof(uuid,smallint,smallint,smallint)',
  'game_npc_source_hazard_complete()','game_npc_source_hazard_child_required()','game_npc_source_hazard_successor()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',signature);
 END LOOP;
END $paths$;
