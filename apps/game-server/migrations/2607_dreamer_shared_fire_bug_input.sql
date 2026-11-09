-- M07: a retained Fire Bug proves custody without inventing personal progress.
CREATE TABLE game_quest_shared_input_proofs (
 character_id UUID NOT NULL,
 cause_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
 transition_key TEXT NOT NULL CHECK(transition_key='oteryn:quest-callback/dreamers-challenge-fire-bug-ignite'),
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 before_item JSONB NOT NULL,
 location JSONB NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('container','nested','backpack','equipment')),
 proof BYTEA NOT NULL CHECK(octet_length(proof) BETWEEN 1 AND 16384),
 proof_sha256 BYTEA NOT NULL CHECK(proof_sha256=sha256(proof)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
 FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
  REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
  DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_quest_shared_input_item ON game_quest_shared_input_proofs(item_instance_id);

CREATE FUNCTION game_quest_shared_input_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; observed JSONB; observed_kind TEXT; p JSONB;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 SELECT c.location,c.kind INTO observed,observed_kind
 FROM game_quest_callback_carried(NEW.character_id,NEW.world_id) c WHERE c.item=NEW.item_instance_id;
 p:=convert_from(NEW.proof,'UTF8')::jsonb;
 IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM NEW.before_item
  OR i.world_id<>NEW.world_id OR i.lifecycle<>1 OR i.quantity<1
  OR i.definition_family<>'Item' OR i.definition_production_key<>'oteryn:item.tibia.i5467'
  OR i.definition_revision_ref<>'definition-r1'
  OR observed IS DISTINCT FROM NEW.location OR observed_kind IS DISTINCT FROM NEW.kind
  OR p->'input' IS DISTINCT FROM NEW.before_item OR p->'location' IS DISTINCT FROM NEW.location
  OR p->>'recipe' IS DISTINCT FROM NEW.transition_key
  OR p->'source' IS DISTINCT FROM '[32849,32233,9,1]'::jsonb
  OR NOT EXISTS(SELECT 1 FROM game_content_activations a
    WHERE a.world_id=NEW.world_id AND a.channel_id=NEW.channel_id
      AND 'sha256:'||encode(a.server_artifact_digest,'hex')=p->'context'->>'content_revision'
      AND p->'context'->'placement_context'=(SELECT jsonb_agg(get_byte(a.frame_binding_digest,b.n) ORDER BY b.n)
        FROM generate_series(0,octet_length(a.frame_binding_digest)-1) AS b(n))
      AND a.activation_sequence=(SELECT max(b.activation_sequence) FROM game_content_activations b
        WHERE b.world_id=NEW.world_id AND b.channel_id=NEW.channel_id))
  OR coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=NEW.world_id),0)::text
       IS DISTINCT FROM p->'context'->>'reset_epoch'
 THEN RAISE EXCEPTION 'Dreamer ignition lacks current retained Fire Bug proof' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE TRIGGER game_quest_shared_input_admit BEFORE INSERT ON game_quest_shared_input_proofs
 FOR EACH ROW EXECUTE FUNCTION game_quest_shared_input_admit();
CREATE TRIGGER game_quest_shared_input_immutable BEFORE UPDATE OR DELETE ON game_quest_shared_input_proofs
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_quest_shared_input_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id()
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=NEW.item_instance_id
    AND to_jsonb(i)=NEW.before_item)
  OR NOT EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,NEW.world_id) c
    WHERE c.item=NEW.item_instance_id AND c.location=NEW.location AND c.kind=NEW.kind)
  OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r
    WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.cause_id
      AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
      AND r.quest_key='oteryn:quest.dreamer_s_challenge_quest' AND r.cause_kind='use'
      AND cardinality(r.track_keys)=0 AND NOT r.completes AND r.created_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'incomplete retained Dreamer input' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_shared_input_complete AFTER INSERT ON game_quest_shared_input_proofs
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_shared_input_complete();

-- Keep every existing effect-shape arm, adding only this exact empty-state action.
DO $migration$
DECLARE predicate TEXT; body TEXT;
 needle TEXT:='IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;';
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT predicate FROM pg_constraint
 WHERE conrelid='game_character_quest_receipts'::regclass AND conname='game_character_quest_receipt_effects_shape';
 ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
 EXECUTE 'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (('
  ||predicate||$shape$) OR (transition_key='oteryn:quest-callback/dreamers-challenge-fire-bug-ignite'
   AND quest_key='oteryn:quest.dreamer_s_challenge_quest' AND cause_kind='use' AND NOT completes
   AND cardinality(track_keys)=0 AND cardinality(values_before)=0 AND cardinality(values_after)=0))$shape$;
 body:=pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure);
 IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
  RAISE EXCEPTION 'missing or ambiguous canonical value-only quest guard';
 END IF;
 body:=replace(body,needle,needle||$arm$
 IF NEW.transition_key='oteryn:quest-callback/dreamers-challenge-fire-bug-ignite' THEN
  IF NEW.quest_key='oteryn:quest.dreamer_s_challenge_quest' AND NEW.cause_kind='use'
   AND NEW.created_xact_id=pg_current_xact_id()
   AND EXISTS(SELECT 1 FROM game_quest_shared_input_proofs p WHERE p.character_id=NEW.character_id
     AND p.cause_id=NEW.cause_id AND p.cause_ordinal=NEW.cause_ordinal AND p.transition_key=NEW.transition_key
     AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
  RAISE EXCEPTION 'Dreamer ignition lacks same-transaction retained input' USING ERRCODE='23514';
 END IF;
$arm$);
 EXECUTE body;
END $migration$;
REVOKE ALL ON game_quest_shared_input_proofs FROM PUBLIC;
REVOKE ALL ON FUNCTION game_quest_shared_input_admit(),game_quest_shared_input_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_shared_input_proofs TO oteryn_game_runtime;
GRANT SELECT ON game_quest_shared_input_proofs TO oteryn_game_control;
