-- An ordinary PvP swing can spend several worn Item charges in its own transaction.
ALTER TABLE game_character_pvp_harm ADD COLUMN IF NOT EXISTS created_xact_id XID8;
ALTER TABLE game_character_pvp_harm ALTER COLUMN created_xact_id SET DEFAULT pg_current_xact_id();

CREATE TABLE IF NOT EXISTS game_pvp_swing_charge_writes (
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL,
 ordinal INTEGER NOT NULL,
 victim UUID NOT NULL,
 victim_session UUID NOT NULL,
 item_instance_id UUID NOT NULL,
 expected_revision NUMERIC(20,0) NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(item_instance_id,expected_revision),
 UNIQUE(game_session_id,command_id,ordinal,victim,item_instance_id),
 FOREIGN KEY(item_instance_id,expected_revision) REFERENCES game_item_timed_state_writes(item_instance_id,expected_revision) DEFERRABLE INITIALLY DEFERRED
);

CREATE OR REPLACE FUNCTION game_pvp_swing_charge_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS (
  SELECT 1
  FROM game_character_pvp_harm h
  JOIN game_item_timed_state_writes w ON (w.item_instance_id,w.expected_revision)=(NEW.item_instance_id,NEW.expected_revision)
  JOIN game_durability_reconnect_sessions s ON s.game_session_id=NEW.victim_session AND s.character_id=NEW.victim
  WHERE (h.game_session_id,h.command_id,h.ordinal,h.victim)=(NEW.game_session_id,NEW.command_id,NEW.ordinal,NEW.victim)
   AND h.field_occurrence=''::bytea
   AND h.created_xact_id=pg_current_xact_id()
   AND w.created_xact_id=pg_current_xact_id()
   AND w.holder_character_id=NEW.victim
   AND w.full_remaining_ms IS NULL
   AND ((w.cause=1 AND w.charges_after=w.charges_before-1)
        OR (w.cause=3 AND w.charges_before=1))
 ) THEN
  RAISE EXCEPTION 'a protective charge must belong to the same committed swing and victim' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END;
$$;
DROP TRIGGER IF EXISTS game_pvp_swing_charge_proven ON game_pvp_swing_charge_writes;
CREATE CONSTRAINT TRIGGER game_pvp_swing_charge_proven AFTER INSERT ON game_pvp_swing_charge_writes
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_pvp_swing_charge_proven();
DROP TRIGGER IF EXISTS game_pvp_swing_charge_writes_immutable ON game_pvp_swing_charge_writes;
CREATE TRIGGER game_pvp_swing_charge_writes_immutable BEFORE UPDATE OR DELETE ON game_pvp_swing_charge_writes
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
DROP TRIGGER IF EXISTS game_pvp_swing_charge_writes_no_truncate ON game_pvp_swing_charge_writes;
CREATE TRIGGER game_pvp_swing_charge_writes_no_truncate BEFORE TRUNCATE ON game_pvp_swing_charge_writes
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();

REVOKE ALL ON game_pvp_swing_charge_writes FROM PUBLIC;
REVOKE ALL ON FUNCTION game_pvp_swing_charge_proven() FROM PUBLIC;
GRANT SELECT,INSERT ON game_pvp_swing_charge_writes TO oteryn_game_runtime;
GRANT SELECT ON game_pvp_swing_charge_writes TO oteryn_game_control;

DO $$
DECLARE body TEXT; needle TEXT;
BEGIN
 SELECT pg_get_functiondef('game_item_timed_state_write_consistency_guard()'::regprocedure) INTO body;
 IF position('FROM game_pvp_swing_charge_writes a' IN body)=0 THEN
  needle:=E'AND (o.item_instance_id, o.expected_revision)\n                  <> (NEW.item_instance_id, NEW.expected_revision)';
  IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing timed single-write proof'; END IF;
  body:=replace(body,needle,needle||$arm$
  AND NOT EXISTS (
   SELECT 1 FROM game_pvp_swing_charge_writes a
   JOIN game_pvp_swing_charge_writes b USING(game_session_id,command_id,ordinal,victim,victim_session)
   WHERE (a.item_instance_id,a.expected_revision)=(NEW.item_instance_id,NEW.expected_revision)
     AND (b.item_instance_id,b.expected_revision)=(o.item_instance_id,o.expected_revision)
     AND a.created_xact_id=pg_current_xact_id() AND b.created_xact_id=pg_current_xact_id())
 $arm$);
  EXECUTE body;
 END IF;
 EXECUTE format('ALTER FUNCTION game_pvp_swing_charge_proven() SET search_path=%I,pg_temp',current_schema());
END;
$$;
