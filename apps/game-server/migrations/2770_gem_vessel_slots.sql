-- Gem Vessel slots (Winter Update 2026): gems carry no Wheel domain and Vessel Resonance is gone.
-- Twelve Gem Vessel slots, each of one fixed gem quality, replace the four domain cells.
-- Stored vessel vectors are converted in place: every legacy socketed gem keeps its socket and
-- moves to the first free slot of its quality, in legacy cell order. Gem domains already stored
-- in Atelier state are inert history and are left untouched. Reapplication is a no-op.

-- Slot qualities in slot order: Lesser, Regular, Greater, four times.
CREATE OR REPLACE FUNCTION game_wheel_gem_vessel_quality(slot_index INT) RETURNS TEXT
 LANGUAGE sql IMMUTABLE AS $$
 SELECT (ARRAY['Lesser','Regular','Greater'])[slot_index%3+1] WHERE slot_index BETWEEN 0 AND 11
$$;

-- The Atelier state key of a sixteen-byte gem identity, or NULL when the identity is malformed.
CREATE OR REPLACE FUNCTION game_wheel_gem_key(identity JSONB) RETURNS TEXT
 LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE gem_key NUMERIC:=0; byte JSONB;
BEGIN
 IF jsonb_typeof(identity) IS DISTINCT FROM 'array' OR jsonb_array_length(identity)<>16 THEN
  RETURN NULL;
 END IF;
 FOR byte IN SELECT value FROM jsonb_array_elements(identity) LOOP
  IF jsonb_typeof(byte)<>'number' OR (byte#>>'{}')::numeric NOT BETWEEN 0 AND 255
    OR trunc((byte#>>'{}')::numeric)<>(byte#>>'{}')::numeric THEN
   RETURN NULL;
  END IF;
  gem_key:=gem_key*256+(byte#>>'{}')::numeric;
 END LOOP;
 RETURN gem_key::text;
END $$;

-- Converts four legacy domain cells into twelve Gem Vessel slots. Fails instead of dropping a gem.
CREATE OR REPLACE FUNCTION game_wheel_gem_vessels_from_legacy(cells JSONB, atelier_state JSONB) RETURNS JSONB
 LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE slots JSONB:='[null,null,null,null,null,null,null,null,null,null,null,null]';
 cell JSONB; quality TEXT; cell_index INT; slot_index INT; placed BOOLEAN;
BEGIN
 IF jsonb_typeof(cells) IS DISTINCT FROM 'array' OR jsonb_array_length(cells)<>4 THEN
  RAISE EXCEPTION 'legacy Wheel vessels must hold four cells' USING ERRCODE='23514';
 END IF;
 FOR cell_index IN 0..3 LOOP
  cell:=cells->cell_index;
  CONTINUE WHEN cell='null'::jsonb;
  IF EXISTS(SELECT 1 FROM jsonb_array_elements(slots) e WHERE e.value=cell) THEN
   RAISE EXCEPTION 'legacy Wheel vessels repeat a gem' USING ERRCODE='23514';
  END IF;
  quality:=atelier_state->'gems'->game_wheel_gem_key(cell)->>'quality';
  IF quality IS NULL THEN
   RAISE EXCEPTION 'legacy Wheel vessel gem has no Atelier state' USING ERRCODE='23514';
  END IF;
  placed:=false;
  FOR slot_index IN 0..11 LOOP
   IF game_wheel_gem_vessel_quality(slot_index)=quality AND slots->slot_index='null'::jsonb THEN
    slots:=jsonb_set(slots,ARRAY[slot_index::text],cell);
    placed:=true;
    EXIT;
   END IF;
  END LOOP;
  IF NOT placed THEN
   RAISE EXCEPTION 'legacy Wheel vessel gem has no free slot of its quality' USING ERRCODE='23514';
  END IF;
 END LOOP;
 RETURN slots;
END $$;

-- Receipts carry the twelve-slot vectors in their own columns: the legacy four-cell columns keep
-- their history and shape, and stay at their empty default on new receipts.
ALTER TABLE game_character_wheel_receipts
 ADD COLUMN IF NOT EXISTS gem_vessels_before JSONB,
 ADD COLUMN IF NOT EXISTS gem_vessels_after JSONB;
ALTER TABLE game_character_wheel_receipts
 ALTER COLUMN gem_vessels_before SET DEFAULT '[null,null,null,null,null,null,null,null,null,null,null,null]',
 ALTER COLUMN gem_vessels_after SET DEFAULT '[null,null,null,null,null,null,null,null,null,null,null,null]';
ALTER TABLE game_character_wheel_receipts DROP CONSTRAINT IF EXISTS game_wheel_gem_vessel_vectors;
ALTER TABLE game_character_wheel_receipts ADD CONSTRAINT game_wheel_gem_vessel_vectors CHECK(
 (gem_vessels_before IS NULL AND gem_vessels_after IS NULL)
 OR (jsonb_typeof(gem_vessels_before)='array' AND jsonb_array_length(gem_vessels_before)=12
  AND jsonb_typeof(gem_vessels_after)='array' AND jsonb_array_length(gem_vessels_after)=12));
DO $$ DECLARE body TEXT; widened TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_character_wheel_receipts'::regclass AND conname='game_character_wheel_receipt_kind_shape';
 IF position('gem_vessels_before' IN body)=0 THEN
  widened:=replace(body,'(vessels_before <> vessels_after)',
   '(vessels_before <> vessels_after) OR (gem_vessels_before IS DISTINCT FROM gem_vessels_after)');
  IF widened=body THEN
   RAISE EXCEPTION 'missing Wheel vessel change guard';
  END IF;
  ALTER TABLE game_character_wheel_receipts DROP CONSTRAINT game_character_wheel_receipt_kind_shape;
  EXECUTE 'ALTER TABLE game_character_wheel_receipts ADD CONSTRAINT game_character_wheel_receipt_kind_shape '||widened;
 END IF;
END $$;

-- Convert the current vessel vectors. The deferred guard demands a same-transaction Wheel receipt
-- for every vessel write, so it is suspended for this one conversion only.
DO $$ DECLARE c TEXT; BEGIN
 FOR c IN SELECT conname FROM pg_constraint
   WHERE conrelid='game_character_wheel_vessels'::regclass AND contype='c' AND conname<>'game_wheel_vessels_slots' LOOP
  EXECUTE format('ALTER TABLE game_character_wheel_vessels DROP CONSTRAINT %I',c);
 END LOOP;
END $$;
ALTER TABLE game_character_wheel_vessels DISABLE TRIGGER game_wheel_vessels_consistent;
UPDATE game_character_wheel_vessels v
   SET vessels=game_wheel_gem_vessels_from_legacy(v.vessels,
     (SELECT s.state_json FROM game_character_atelier_state s WHERE s.character_id=v.character_id))
 WHERE jsonb_array_length(v.vessels)=4;
ALTER TABLE game_character_wheel_vessels ENABLE TRIGGER game_wheel_vessels_consistent;
ALTER TABLE game_character_wheel_vessels DROP CONSTRAINT IF EXISTS game_wheel_vessels_slots;
ALTER TABLE game_character_wheel_vessels ADD CONSTRAINT game_wheel_vessels_slots
 CHECK(jsonb_typeof(vessels)='array' AND jsonb_array_length(vessels)=12);

-- Chain, identity, slot quality, distinctness and ruleset of every newly socketed gem. A legacy
-- previous receipt is compared through the same conversion the stored vector went through.
CREATE OR REPLACE FUNCTION game_wheel_vessels_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE current_vessels JSONB; previous JSONB; atelier_state JSONB; vessel JSONB; slot_index INT; gem JSONB;
 empty CONSTANT JSONB:='[null,null,null,null,null,null,null,null,null,null,null,null]';
BEGIN
 SELECT vessels INTO current_vessels FROM game_character_wheel_vessels WHERE character_id=NEW.character_id;
 IF TG_TABLE_NAME='game_character_wheel_receipts' THEN
  SELECT state_json INTO atelier_state FROM game_character_atelier_state WHERE character_id=NEW.character_id;
  SELECT coalesce(r.gem_vessels_after,game_wheel_gem_vessels_from_legacy(r.vessels_after,atelier_state))
    INTO previous FROM game_character_wheel_receipts r
   WHERE r.character_id=NEW.character_id AND r.after_wheel_revision=NEW.before_wheel_revision;
  IF NEW.gem_vessels_before IS DISTINCT FROM coalesce(previous,empty)
    OR coalesce(current_vessels,empty) IS DISTINCT FROM NEW.gem_vessels_after THEN
   RAISE EXCEPTION 'Wheel vessel receipt chain mismatch' USING ERRCODE='23514';
  END IF;
  IF (SELECT count(*) FROM jsonb_array_elements(NEW.gem_vessels_after) e WHERE e.value<>'null'::jsonb)
    <>(SELECT count(DISTINCT e.value) FROM jsonb_array_elements(NEW.gem_vessels_after) e WHERE e.value<>'null'::jsonb) THEN
   RAISE EXCEPTION 'a gem fills at most one Gem Vessel' USING ERRCODE='23514';
  END IF;
  FOR slot_index IN 0..11 LOOP
   vessel:=NEW.gem_vessels_after->slot_index;
   IF vessel<>'null'::jsonb AND vessel IS DISTINCT FROM NEW.gem_vessels_before->slot_index THEN
    IF game_wheel_gem_key(vessel) IS NULL THEN
     RAISE EXCEPTION 'invalid vessel identity' USING ERRCODE='23514';
    END IF;
    gem:=atelier_state->'gems'->game_wheel_gem_key(vessel);
    IF gem IS NULL OR gem->>'quality' IS DISTINCT FROM game_wheel_gem_vessel_quality(slot_index)
      OR gem->>'ruleset_revision' IS DISTINCT FROM NEW.wheel_ruleset_revision THEN
     RAISE EXCEPTION 'vessel gem must belong to this Character, slot quality and ruleset' USING ERRCODE='23514';
    END IF;
   END IF;
  END LOOP;
 ELSE
  IF NOT EXISTS(SELECT 1 FROM game_character_wheel_receipts r WHERE r.character_id=NEW.character_id
    AND r.wheel_occurrence_id=NEW.last_wheel_occurrence_id AND r.gem_vessels_after=NEW.vessels
    AND r.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'vessels require same-transaction Wheel receipt' USING ERRCODE='23514';
  END IF;
 END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_wheel_vessels_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_wheel_gem_vessel_quality(INT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_wheel_gem_key(JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_wheel_gem_vessels_from_legacy(JSONB,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_wheel_vessels_guard(),game_wheel_gem_vessel_quality(INT),game_wheel_gem_key(JSONB),
 game_wheel_gem_vessels_from_legacy(JSONB,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_wheel_gem_vessel_quality(INT),game_wheel_gem_key(JSONB),
 game_wheel_gem_vessels_from_legacy(JSONB,JSONB) TO oteryn_game_runtime;
