-- Closed child; no second Character revision/transaction or post-commit writer.
CREATE TABLE game_npc_source_actor_effects (
 npc_transaction_id UUID PRIMARY KEY REFERENCES game_npc_cosmetic_receipts(transaction_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 source_key TEXT NOT NULL CHECK(source_key='oteryn:npc-cosmetic/ajax/barbarian-fire'),
 program_sha256 BYTEA NOT NULL CHECK(octet_length(program_sha256)=32),
 committed_revision NUMERIC(20,0) NOT NULL,
 observation JSONB NOT NULL CHECK(jsonb_typeof(observation)='object' AND octet_length(observation::text)<=65536),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id)
);
CREATE INDEX game_npc_source_actor_effects_character ON game_npc_source_actor_effects(character_id);
REVOKE ALL ON game_npc_source_actor_effects FROM PUBLIC;
GRANT SELECT,INSERT ON game_npc_source_actor_effects TO oteryn_game_runtime;
CREATE TABLE game_npc_source_actor_snapshot_proofs (
 backend_pid INTEGER PRIMARY KEY,transaction_xid XID8 NOT NULL,
 npc_transaction_id UUID NOT NULL,character_id UUID NOT NULL,
 conditions_sha256 BYTEA NOT NULL CHECK(octet_length(conditions_sha256)=32)
);
REVOKE ALL ON game_npc_source_actor_snapshot_proofs FROM PUBLIC,oteryn_game_runtime;
CREATE FUNCTION game_npc_source_actor_effect_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_npc_source_context%ROWTYPE;r game_npc_cosmetic_receipts%ROWTYPE;BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'immutable NPC actor effect'; END IF;
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 SELECT * INTO r FROM game_npc_cosmetic_receipts WHERE transaction_id=NEW.npc_transaction_id;
 IF c.backend_pid IS NULL OR c.captured IS DISTINCT FROM TRUE
  OR c.npc_key<>'oteryn:npc.ajax' OR c.source_key<>'oteryn:npc-cosmetic/ajax/barbarian-fire'
  OR NEW.npc_transaction_id IS DISTINCT FROM c.npc_transaction_id
  OR NEW.character_id IS DISTINCT FROM c.character_id OR NEW.game_session_id IS DISTINCT FROM c.game_session_id
  OR NEW.command_id IS DISTINCT FROM c.command_id OR NEW.program_sha256 IS DISTINCT FROM c.program_sha256
  OR NEW.source_key IS DISTINCT FROM c.source_key OR NEW.committed_revision IS DISTINCT FROM c.original_revision+1
  OR NEW.observation IS DISTINCT FROM c.before_state->'actor_effect'
  OR NEW.observation->>'kind' IS DISTINCT FROM 'ajax_fire'
  OR game_npc_source_parent_current(NEW.npc_transaction_id,NEW.source_key) IS DISTINCT FROM TRUE
  OR game_npc_source_effect_current(NEW.npc_transaction_id,NEW.source_key,'{"kind":"condition_damage","condition":"FIRE","delayed":true,"ticks":10,"interval_ms":2000,"damage_per_tick":10}'::jsonb) IS DISTINCT FROM TRUE
  OR NEW.observation->>'lease_generation' IS DISTINCT FROM c.lease_generation::text
  OR r.character_id IS DISTINCT FROM NEW.character_id OR r.game_session_id IS DISTINCT FROM NEW.game_session_id
  OR r.command_id IS DISTINCT FROM NEW.command_id OR r.source_key IS DISTINCT FROM NEW.source_key
  OR r.program_sha256 IS DISTINCT FROM NEW.program_sha256 OR r.created_xact_id IS DISTINCT FROM pg_current_xact_id()
  OR r.committed_character_revision IS DISTINCT FROM NEW.committed_revision
  OR NOT ((c.expected_plan->'effects') @> '[{"kind":"condition_damage","condition":"FIRE","delayed":true,"ticks":10,"interval_ms":2000,"damage_per_tick":10}]'::jsonb)
  OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=NEW.character_id AND character_revision=NEW.committed_revision)
 THEN RAISE EXCEPTION 'NPC actor effect lacks exact source/Node/actor/parent proof';END IF;
 NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_npc_source_actor_effect_admit BEFORE INSERT OR UPDATE OR DELETE ON game_npc_source_actor_effects FOR EACH ROW EXECUTE FUNCTION game_npc_source_actor_effect_admit();
CREATE FUNCTION game_npc_source_actor_snapshot_capture() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_npc_source_context%ROWTYPE;o JSONB;BEGIN
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF c.backend_pid IS NULL OR c.before_state->'actor_effect' IS NULL OR c.before_state->'actor_effect'='null'::jsonb THEN RETURN NEW;END IF;
 o:=c.before_state->'actor_effect';
 IF NEW.character_id IS DISTINCT FROM c.character_id OR NEW.character_lease_generation IS DISTINCT FROM c.lease_generation
  OR NEW.snapshot IS DISTINCT FROM o->'conditions_after' OR encode(NEW.content_digest,'hex') IS DISTINCT FROM o->>'content_digest'
 THEN RAISE EXCEPTION 'NPC source actor snapshot differs from sealed native after';END IF;
 INSERT INTO game_npc_source_actor_snapshot_proofs VALUES(pg_backend_pid(),pg_current_xact_id(),c.npc_transaction_id,c.character_id,sha256(convert_to(NEW.snapshot::text,'UTF8')))
 ON CONFLICT(backend_pid) DO UPDATE SET transaction_xid=EXCLUDED.transaction_xid,npc_transaction_id=EXCLUDED.npc_transaction_id,character_id=EXCLUDED.character_id,conditions_sha256=EXCLUDED.conditions_sha256;
 RETURN NEW;
END $$;
CREATE TRIGGER game_npc_source_actor_snapshot_capture AFTER INSERT OR UPDATE ON game_character_condition_snapshots FOR EACH ROW EXECUTE FUNCTION game_npc_source_actor_snapshot_capture();
CREATE FUNCTION game_npc_source_actor_effect_complete() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_npc_source_actor_snapshot_proofs p JOIN game_character_condition_snapshots s ON s.character_id=p.character_id
 WHERE p.backend_pid=pg_backend_pid() AND p.transaction_xid=pg_current_xact_id() AND p.npc_transaction_id=NEW.npc_transaction_id
 AND p.character_id=NEW.character_id AND p.conditions_sha256=sha256(convert_to(s.snapshot::text,'UTF8'))
 AND s.snapshot=NEW.observation->'conditions_after' AND encode(s.content_digest,'hex')=NEW.observation->>'content_digest'
 AND s.character_lease_generation=(NEW.observation->>'lease_generation')::numeric)
 THEN RAISE EXCEPTION 'NPC actor effect requires same-XID exact condition snapshot';END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_source_actor_effect_complete AFTER INSERT ON game_npc_source_actor_effects DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_actor_effect_complete();

CREATE FUNCTION game_npc_source_ajax_child_required() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.source_key='oteryn:npc-cosmetic/ajax/barbarian-fire' AND NOT EXISTS(SELECT 1 FROM game_npc_source_actor_effects a WHERE a.npc_transaction_id=NEW.transaction_id AND a.created_xact_id=NEW.created_xact_id)
 THEN RAISE EXCEPTION 'Ajax FIRE source parent requires native actor child';END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_source_ajax_child_required AFTER INSERT ON game_npc_cosmetic_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_ajax_child_required();

-- Recheck after every later snapshot mutation, including DELETE, so an early
-- SET CONSTRAINTS cannot validate a child then remove its successor snapshot.
CREATE FUNCTION game_npc_source_actor_snapshot_complete() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE char_id UUID; a game_npc_source_actor_effects%ROWTYPE;
BEGIN
 char_id:=CASE WHEN TG_OP='DELETE' THEN OLD.character_id ELSE NEW.character_id END;
 FOR a IN SELECT * FROM game_npc_source_actor_effects WHERE character_id=char_id AND created_xact_id=pg_current_xact_id()
 LOOP
  IF NOT EXISTS(SELECT 1 FROM game_character_condition_snapshots s
    WHERE s.character_id=a.character_id AND s.snapshot=a.observation->'conditions_after'
    AND encode(s.content_digest,'hex')=a.observation->>'content_digest'
    AND s.character_lease_generation=(a.observation->>'lease_generation')::numeric)
  THEN RAISE EXCEPTION 'NPC actor child lost its exact condition snapshot';END IF;
 END LOOP;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_source_actor_snapshot_complete AFTER INSERT OR UPDATE OR DELETE ON game_character_condition_snapshots DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_actor_snapshot_complete();

DO $actor_effect_paths$ DECLARE signature TEXT; BEGIN
 FOREACH signature IN ARRAY ARRAY['game_npc_source_actor_effect_admit()',
 'game_npc_source_actor_snapshot_capture()','game_npc_source_actor_effect_complete()',
 'game_npc_source_ajax_child_required()','game_npc_source_actor_snapshot_complete()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime',signature);
 END LOOP;
END $actor_effect_paths$;
