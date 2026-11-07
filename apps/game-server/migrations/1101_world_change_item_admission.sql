-- WVAR: extend existing craft/terrain/contribution owners; no new value ledger.
ALTER TABLE game_world_change_tallies DROP CONSTRAINT game_world_change_tallies_resource_check;
ALTER TABLE game_world_change_tallies ADD CONSTRAINT game_world_change_tallies_resource_check
    CHECK(resource IN ('coal','medicine','votes','awash_coal','thornfire_extinguished'));
ALTER TABLE game_world_change_contributions DROP CONSTRAINT game_world_change_contributions_resource_check;
ALTER TABLE game_world_change_contributions ADD CONSTRAINT game_world_change_contributions_resource_check
    CHECK(resource IN ('coal','medicine','votes','awash_coal','thornfire_extinguished'));
ALTER TABLE game_world_change_contributions ADD COLUMN created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id();
-- Historical receipts stay immutable and unqualified for the new admission arms.
ALTER TABLE game_terrain_tool_receipts ADD COLUMN before_item_id BIGINT
    CHECK(before_item_id BETWEEN 0 AND 4294967295);
DO $migration$
DECLARE body TEXT; before TEXT; after TEXT;
BEGIN
    body:=pg_get_functiondef('game_apply_terrain_tool_transform(uuid,numeric,uuid,uuid,bytea,uuid,uuid,bytea,text,bytea,bigint,bigint,bigint,bigint,bigint[],bigint[])'::regprocedure);
    before:='placement,reset_epoch,transformed_item_id';
    after:='placement,reset_epoch,transformed_item_id,before_item_id';
    IF length(body)-length(replace(body,before,''))<>length(before) THEN
        RAISE EXCEPTION 'expected exactly one terrain receipt column list'; END IF;
    body:=replace(body,before,after);
    before:='p_placement,p_reset_epoch,p_after_item_id';
    after:='p_placement,p_reset_epoch,p_after_item_id,p_before_item_id';
    IF length(body)-length(replace(body,before,''))<>length(before) THEN
        RAISE EXCEPTION 'expected exactly one terrain receipt value list'; END IF;
    EXECUTE replace(body,before,after);
END $migration$;

-- Exact counterpart of the private item-craft operation key. The input is UUID
-- bytes followed by canonical integer ASCII, not locale-formatted UUID/numeric text.
CREATE FUNCTION game_world_change_craft_operation_id(p_session UUID,p_command NUMERIC)
RETURNS BYTEA LANGUAGE SQL IMMUTABLE STRICT SET search_path=pg_catalog AS $$
 SELECT CASE WHEN p_command BETWEEN 1 AND 18446744073709551615 AND p_command=trunc(p_command)
 THEN substring(sha256(convert_to('oteryn.world-change.station.v1','UTF8') || decode('00','hex')
        || uuid_send(p_session) || convert_to(p_command::numeric(20,0)::text,'UTF8')) FROM 1 FOR 16) END
$$;
CREATE INDEX game_world_change_craft_operation
    ON game_item_craft_records(world_id,game_world_change_craft_operation_id(game_session_id,command_id))
    WHERE recipe='oteryn:world-change.contribution';

-- This predicate is shared by BOTH receipt directions. Existing craft completion
-- still proves the actual carried debit and each fresh output after these checks.
CREATE FUNCTION game_world_change_craft_pair(
    c game_item_craft_records,r game_terrain_tool_receipts,w game_world_change_contributions)
RETURNS BOOLEAN LANGUAGE SQL STABLE AS $$
 SELECT c.recipe='oteryn:world-change.contribution' AND c.consume_source
    AND c.created_xact_id=pg_current_xact_id()
    AND r.created_xact_id=pg_current_xact_id() AND w.created_xact_id=pg_current_xact_id()
    AND c.world_id=r.world_id AND w.world_id=r.world_id AND w.reset_epoch=r.reset_epoch
    AND c.game_session_id=r.game_session_id AND c.command_id=r.command_id
    AND c.character_id=r.character_id AND c.intent=r.intent
    AND c.source_item_instance_id=r.tool_item_instance_id AND w.amount=1
    AND w.operation_id=game_world_change_craft_operation_id(c.game_session_id,c.command_id)
    AND octet_length(r.placement)=40
    AND CASE w.resource
      WHEN 'awash_coal' THEN
        c.before_item->>'definition_production_key'='oteryn:item.tibia.i12600' AND c.outputs='[]'::jsonb
        AND r.before_item_id=r.transformed_item_id AND r.transformed_item_id IN (12666,12667,12668)
        -- Exact retained source pumps, ordinal1 after their native ground entry.
        AND encode(substring(r.placement FROM 33 FOR 8),'hex') IN
            ('00007f2e7d070901','00007f307d0e0901','00007f377d180901')
      WHEN 'thornfire_extinguished' THEN
        c.before_item->>'definition_production_key'='oteryn:item.tibia.i12722'
        AND jsonb_array_length(c.outputs)=1
        AND c.outputs->0->>'key'='oteryn:item.tibia.i12819'
        AND c.outputs->0->>'count'='1' AND r.transformed_item_id=0
        AND r.before_item_id IN (232,5061,5062,5063,5064,5065,5066,5067,6288,6289,7803,19096,32021)
      ELSE FALSE END
$$;

-- Preserve the complete previous expression, including every event-container,
-- soulpit and planting arm. New zero-output shapes also require the guards below.
DO $migration$
DECLARE previous TEXT;
BEGIN
    SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
        WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_records_outputs_check';
    ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
    EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ((%s) OR (jsonb_typeof(outputs)=''array'' AND outputs=''[]''::jsonb AND consume_source AND (recipe=''oteryn:world-change.contribution'' OR (recipe=''oteryn:gathering.crystal'' AND before_item->>''definition_production_key''=''oteryn:item.tibia.i3492''))))',previous);
END $migration$;

DO $migration$
DECLARE body TEXT; begin_at INT; arm TEXT;
BEGIN
    body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected craft admission body'; END IF;
    arm:=$arm$
    IF NEW.recipe='oteryn:world-change.contribution' THEN
      IF NOT EXISTS (
        SELECT 1 FROM game_terrain_tool_receipts r
        JOIN game_terrain_tool_transforms t ON (t.world_id,t.channel_id,t.placement,t.reset_epoch)
            =(r.world_id,r.channel_id,r.placement,r.reset_epoch)
        JOIN game_world_change_contributions w ON w.world_id=r.world_id AND w.reset_epoch=r.reset_epoch
            AND w.operation_id=game_world_change_craft_operation_id(NEW.game_session_id,NEW.command_id)
        WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
            AND game_world_change_craft_pair(NEW,r,w)
            AND t.state_item_id=r.transformed_item_id
            AND t.decay_states='{}'::bigint[] AND t.decay_delays='{}'::bigint[])
      THEN RAISE EXCEPTION 'world station requires atomic terrain and contribution receipts' USING ERRCODE='23514'; END IF;
    END IF;
    IF NEW.recipe='oteryn:gathering.crystal' AND NEW.consume_source AND NEW.outputs='[]'::jsonb THEN
      IF NEW.before_item->>'definition_production_key' IS DISTINCT FROM 'oteryn:item.tibia.i3492'
        OR NOT EXISTS (
          SELECT 1 FROM game_terrain_tool_receipts r
          JOIN game_terrain_tool_transforms t ON (t.world_id,t.channel_id,t.placement,t.reset_epoch)
              =(r.world_id,r.channel_id,r.placement,r.reset_epoch)
          WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
              AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id AND r.intent=NEW.intent
              AND r.created_xact_id=pg_current_xact_id() AND r.before_item_id=13989 AND r.transformed_item_id=13988
              AND t.state_item_id=13988 AND t.decay_states=ARRAY[13989]::bigint[]
              AND t.decay_delays=ARRAY[1200000]::bigint[])
      THEN RAISE EXCEPTION 'empty desert catch requires its exact spent-water receipt' USING ERRCODE='23514'; END IF;
    END IF;
$arm$;
    -- Append checks at the start of the EXISTING chain. Rod/tool/custody/input,
    -- event-container, enchanting, soulpit and planting guards remain intact.
    EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

CREATE FUNCTION game_world_change_craft_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_craft_records c
        JOIN game_terrain_tool_receipts r ON (r.game_session_id,r.command_id)=(c.game_session_id,c.command_id)
        WHERE c.world_id=NEW.world_id AND c.recipe='oteryn:world-change.contribution'
            AND game_world_change_craft_operation_id(c.game_session_id,c.command_id)=NEW.operation_id
            AND game_world_change_craft_pair(c,r,NEW))
    THEN RAISE EXCEPTION 'world station contribution requires its atomic carried debit' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_world_change_craft_complete AFTER INSERT ON game_world_change_contributions
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    WHEN (NEW.resource IN ('awash_coal','thornfire_extinguished'))
    EXECUTE FUNCTION game_world_change_craft_complete();

DO $migration$
DECLARE f TEXT;
BEGIN
    FOREACH f IN ARRAY ARRAY['game_world_change_craft_operation_id(uuid,numeric)',
        'game_world_change_craft_pair(game_item_craft_records,game_terrain_tool_receipts,game_world_change_contributions)',
        'game_world_change_craft_complete()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',f,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
    END LOOP;
END $migration$;
GRANT EXECUTE ON FUNCTION game_world_change_craft_operation_id(UUID,NUMERIC),
    game_world_change_craft_pair(game_item_craft_records,game_terrain_tool_receipts,game_world_change_contributions)
    TO oteryn_game_runtime;
