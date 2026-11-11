-- WU26-06 Temple Teleport Scroll (docs/contracts/TEMPLE_TELEPORT_SCROLL_V1.md).
-- The native item-use writer burns one scroll as ItemUseCause 481 with one paired
-- immutable use row; the row's stamp applies the scroll's seven-day TIMED deadline
-- (Expire {Deadline}, RT-1c), which the daily claim fixes once on the immutable
-- delivery line. All prior item-use causes, shapes and daily line intervals stay
-- admitted unchanged.
--
-- Rollback: applied migrations are immutable. A successor stops new writes (the
-- dispatch branch and the INSERT grant) and keeps every row as evidence of a burn.

-- 1. The delivery line carries the deadline: delivery + 168 hours (604,800,000 ms).
DO $$
DECLARE
 previous_rule TEXT;
BEGIN
 SELECT pg_get_expr(c.conbin,c.conrelid) INTO STRICT previous_rule
   FROM pg_constraint c
  WHERE c.conrelid='game_character_daily_reward_lines'::regclass
    AND c.conname='daily_effect_interval' AND c.contype='c';
 EXECUTE format(
   'ALTER TABLE game_character_daily_reward_lines DROP CONSTRAINT daily_effect_interval, '
   || 'ADD CONSTRAINT daily_effect_interval CHECK ((%s) OR (kind=''TEMPLE_SCROLL'' '
   || 'AND status=''DELIVERED'' AND effect_started_at IS NOT NULL '
   || 'AND effect_expires_at=effect_started_at+interval ''168 hours''))',
   previous_rule);
END $$;
-- Lines delivered before this migration keep their null interval (no deadline).
ALTER TABLE game_character_daily_reward_lines ADD CONSTRAINT daily_temple_scroll_deadline
 CHECK (kind<>'TEMPLE_SCROLL' OR status<>'DELIVERED' OR effect_expires_at IS NOT NULL) NOT VALID;
CREATE INDEX game_character_daily_reward_lines_item
 ON game_character_daily_reward_lines(item_instance_id) WHERE item_instance_id IS NOT NULL;

-- 2. ItemUseCause 481 TempleScroll: a burn only.
DO $$
DECLARE
 constraint_name TEXT;
 previous_rule TEXT;
BEGIN
 FOREACH constraint_name IN ARRAY ARRAY[
   'game_item_use_records_cause_kind_check','game_item_use_cause_shape'
 ] LOOP
   SELECT pg_get_expr(c.conbin,c.conrelid) INTO STRICT previous_rule
     FROM pg_constraint c
    WHERE c.conrelid='game_item_use_records'::regclass
      AND c.conname=constraint_name AND c.contype='c';
   EXECUTE format(
     'ALTER TABLE game_item_use_records DROP CONSTRAINT %I, ADD CONSTRAINT %I CHECK ((%s) OR (cause_kind=481 AND shape=1))',
     constraint_name,constraint_name,previous_rule);
 END LOOP;
END $$;

-- 3. One use row per burn. The stamp, not the caller, reads the deadline and the clock.
CREATE TABLE game_temple_scroll_uses (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id))
   REFERENCES game_item_use_records(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 deadline_at TIMESTAMPTZ,
 observed_at TIMESTAMPTZ NOT NULL,
 expired BOOLEAN NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(expired=(deadline_at IS NOT NULL AND deadline_at<=observed_at))
);
CREATE INDEX game_temple_scroll_uses_character ON game_temple_scroll_uses(character_id);
CREATE TRIGGER game_temple_scroll_uses_immutable BEFORE UPDATE OR DELETE ON game_temple_scroll_uses
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_temple_scroll_uses_no_truncate BEFORE TRUNCATE ON game_temple_scroll_uses
 EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_temple_scroll_use_stamp() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
 NEW.observed_at:=public.game_daily_effect_now();
 NEW.deadline_at:=(SELECT min(l.effect_expires_at) FROM public.game_character_daily_reward_lines l
   WHERE l.item_instance_id=NEW.item_instance_id AND l.kind='TEMPLE_SCROLL' AND l.status='DELIVERED');
 NEW.expired:=coalesce(NEW.deadline_at<=NEW.observed_at,false);
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE TRIGGER game_temple_scroll_uses_stamp BEFORE INSERT ON game_temple_scroll_uses
 FOR EACH ROW EXECUTE FUNCTION game_temple_scroll_use_stamp();

-- 4. The burn and its row commit together: the same transaction, Character and scroll.
CREATE FUNCTION game_temple_scroll_use_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE s public.game_temple_scroll_uses%ROWTYPE;
BEGIN
 IF TG_TABLE_NAME='game_temple_scroll_uses' THEN s:=NEW;
 ELSE
   SELECT * INTO s FROM public.game_temple_scroll_uses WHERE transaction_id=NEW.transaction_id;
   IF s.transaction_id IS NULL THEN
     RAISE EXCEPTION 'temple scroll burn lacks its use row' USING ERRCODE='23514';
   END IF;
 END IF;
 IF NOT EXISTS (
   SELECT 1 FROM public.game_item_use_records u WHERE u.transaction_id=s.transaction_id
    AND u.cause_kind=481 AND u.shape=1 AND u.created_xact_id=s.created_xact_id
    AND u.character_id=s.character_id AND u.source_item_instance_id=s.item_instance_id
    AND u.source_definition_family='Item'
    AND u.source_definition_production_key='oteryn:item.tibia.i25718'
 ) THEN RAISE EXCEPTION 'temple scroll use row differs from its burn' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_temple_scroll_use_consistent AFTER INSERT ON game_temple_scroll_uses
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_temple_scroll_use_consistent();
CREATE CONSTRAINT TRIGGER game_temple_scroll_burn_consistent AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause_kind=481)
 EXECUTE FUNCTION game_temple_scroll_use_consistent();

REVOKE ALL ON game_temple_scroll_uses FROM PUBLIC;
REVOKE ALL ON FUNCTION game_temple_scroll_use_stamp() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_temple_scroll_use_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_temple_scroll_uses TO oteryn_game_runtime;
GRANT SELECT ON game_temple_scroll_uses TO oteryn_game_control;
