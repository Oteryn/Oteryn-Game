-- An initial stream needs an explicitly accepted baseline. Installing this
-- migration accepts none. Runtime cannot manufacture or replace an approval.
CREATE TABLE game_journal_items_baselines (
 stream_key BYTEA PRIMARY KEY REFERENCES game_journal_streams(stream_key),
 approval_ref TEXT NOT NULL CHECK(approval_ref ~ '^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$'),
 accepted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
REVOKE ALL ON game_journal_items_baselines FROM PUBLIC;
GRANT SELECT ON game_journal_items_baselines TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_journal_items_baselines TO oteryn_game_control;

-- One live character/family cut per source generation. Preserve native IDs;
-- these indexed columns are derived only from the canonical stream binding.
ALTER TABLE game_journal_streams ADD COLUMN character_id UUID REFERENCES game_character_roots(character_id);
ALTER TABLE game_journal_streams ADD COLUMN family TEXT;
UPDATE game_journal_streams SET
 character_id=(convert_from(stream_key,'UTF8')::jsonb->>'character_id')::uuid,
 family=convert_from(stream_key,'UTF8')::jsonb->>'family';
ALTER TABLE game_journal_streams ALTER COLUMN family SET NOT NULL;
ALTER TABLE game_journal_streams ADD CHECK(family IN('ITEMS','BANK','MARKET','PLAYER_TRADE'));
ALTER TABLE game_journal_streams ADD CHECK((family='BANK')=(character_id IS NULL));
CREATE INDEX game_journal_stream_character ON game_journal_streams(character_id) WHERE character_id IS NOT NULL;
CREATE UNIQUE INDEX game_journal_active_character_cut ON game_journal_streams(source_authority,character_id,family,recovery_generation) WHERE active AND character_id IS NOT NULL;
CREATE FUNCTION game_journal_stream_binding() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE k JSONB;
BEGIN
 k:=convert_from(NEW.stream_key,'UTF8')::jsonb;
 NEW.character_id:=(k->>'character_id')::uuid;
 NEW.family:=k->>'family';
 -- Match native ownership/capture lock order: Character, generation, stream.
 IF NEW.character_id IS NOT NULL THEN
  PERFORM 1 FROM public.game_character_roots WHERE character_id=NEW.character_id FOR KEY SHARE;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_journal_stream_binding() FROM PUBLIC;
CREATE TRIGGER game_journal_00_stream_binding BEFORE INSERT ON game_journal_streams
 FOR EACH ROW EXECUTE FUNCTION game_journal_stream_binding();

-- Ownership/World cutoff fences queued exports in the same native transaction.
CREATE FUNCTION game_journal_owner_fence() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 UPDATE public.game_journal_streams SET active=false WHERE character_id=NEW.character_id AND active;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_journal_owner_fence() FROM PUBLIC;
CREATE TRIGGER game_journal_owner_fence AFTER UPDATE OF account_id,world_id ON game_character_roots
 FOR EACH ROW WHEN(OLD.account_id IS DISTINCT FROM NEW.account_id OR OLD.world_id IS DISTINCT FROM NEW.world_id)
 EXECUTE FUNCTION game_journal_owner_fence();
