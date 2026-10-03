-- LOCAL CANDIDATE: source barrier attribution on the real ItemInstance MINT.
-- Immutable historical properties are not a second Item/custody owner.
CREATE TABLE game_spell_item_source_descriptions (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
 transaction_id UUID NOT NULL,
 ordinal INTEGER NOT NULL,
 description TEXT NOT NULL CHECK(octet_length(description) BETWEEN 12 AND 1024),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(transaction_id,ordinal) REFERENCES game_spell_item_lines(transaction_id,ordinal),
 UNIQUE(transaction_id,ordinal)
);

CREATE FUNCTION game_spell_item_description_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE op JSONB;
BEGIN
 SELECT convert_from(r.intent,'UTF8')::jsonb->'operations'->(NEW.ordinal-1)
 INTO op FROM game_spell_item_receipts r
 JOIN game_spell_item_lines l ON l.transaction_id=r.transaction_id
 JOIN game_character_roots c ON c.character_id=r.character_id
 JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
 WHERE r.transaction_id=NEW.transaction_id AND l.ordinal=NEW.ordinal
 AND l.item_instance_id=NEW.item_instance_id AND l.operation_kind=1
 AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()
 AND NEW.created_xact_id=pg_current_xact_id()
 AND i.minted_transaction_id=r.transaction_id AND i.world_id=r.world_id
 AND NEW.description='Casted by: ' || c.name;
 IF op IS NULL OR op->>'kind'<>'mint_ground'
 OR op->'id' IS DISTINCT FROM to_jsonb(ARRAY(SELECT get_byte(uuid_send(NEW.item_instance_id),n) FROM generate_series(0,15) n))
 OR op->>'description' IS DISTINCT FROM NEW.description THEN
  RAISE EXCEPTION 'source description requires exact same-TX caster/MINT intent' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_item_description_proven
 AFTER INSERT ON game_spell_item_source_descriptions DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_spell_item_description_proven();

CREATE FUNCTION game_spell_item_description_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF EXISTS(
  SELECT 1 FROM jsonb_array_elements(convert_from(NEW.intent,'UTF8')::jsonb->'operations') WITH ORDINALITY AS o(op,ordinal)
  WHERE op->>'description' IS NOT NULL
  AND (op->>'kind'<>'mint_ground' OR NOT EXISTS(
   SELECT 1 FROM game_spell_item_source_descriptions d
   JOIN game_spell_item_lines l ON l.transaction_id=d.transaction_id AND l.ordinal=d.ordinal
   WHERE d.transaction_id=NEW.transaction_id AND d.ordinal=o.ordinal
   AND l.operation_kind=1 AND d.item_instance_id=l.item_instance_id
   AND d.description=op->>'description' AND d.created_xact_id=pg_current_xact_id()
  ))
 ) THEN
  RAISE EXCEPTION 'source MINT attribution omitted or changed' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_item_description_complete
 AFTER INSERT ON game_spell_item_receipts DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_spell_item_description_complete();

CREATE TRIGGER game_spell_item_description_immutable
 BEFORE UPDATE OR DELETE OR TRUNCATE ON game_spell_item_source_descriptions
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();
ALTER FUNCTION game_spell_item_description_proven() SET search_path FROM CURRENT;
ALTER FUNCTION game_spell_item_description_complete() SET search_path FROM CURRENT;
REVOKE ALL ON game_spell_item_source_descriptions FROM PUBLIC;
REVOKE ALL ON FUNCTION game_spell_item_description_proven(),game_spell_item_description_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_spell_item_source_descriptions TO oteryn_game_runtime;
GRANT SELECT ON game_spell_item_source_descriptions TO oteryn_game_control;
