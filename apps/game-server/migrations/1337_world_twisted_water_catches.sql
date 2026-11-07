-- WCONTRIB: personal Shimmer Swimmer restriction joins the existing craft receipt.
CREATE TABLE game_twisted_water_catches (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    last_catch_at BIGINT NOT NULL,
    transaction_id UUID NOT NULL REFERENCES game_item_craft_records(transaction_id)
);
CREATE INDEX game_twisted_water_catches_transaction ON game_twisted_water_catches(transaction_id);
REVOKE ALL ON game_twisted_water_catches FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_twisted_water_catches TO oteryn_game_runtime;
GRANT SELECT ON game_twisted_water_catches TO oteryn_game_control;

CREATE FUNCTION game_twisted_water_catch_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='UPDATE' AND (
        NEW.character_id<>OLD.character_id
        OR NEW.last_catch_at::numeric<OLD.last_catch_at::numeric+72000000
    ) THEN RAISE EXCEPTION 'Shimmer Swimmer requires twenty elapsed hours' USING ERRCODE='23514'; END IF;
    IF NEW.last_catch_at < floor(extract(epoch FROM transaction_timestamp())*1000)::bigint
        OR NEW.last_catch_at > ceil(extract(epoch FROM clock_timestamp())*1000)::bigint
        OR NOT EXISTS (
            SELECT 1 FROM game_item_craft_records r
            WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id()
                AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id
                AND r.recipe='oteryn:gathering.crystal' AND r.consume_source
                AND EXISTS (
                    SELECT 1 FROM jsonb_array_elements(r.outputs) o
                    JOIN game_item_instances i ON i.item_instance_id=(o->>'item')::uuid
                    WHERE o->>'key'='oteryn:item.tibia.i12557' AND (o->>'count')::bigint=1
                        AND i.world_id=r.world_id AND i.lifecycle=1 AND i.quantity=1
                        AND i.definition_production_key='oteryn:item.tibia.i12557'
                        AND i.minted_transaction_id=r.transaction_id
                )
        ) THEN RAISE EXCEPTION 'Shimmer Swimmer cooldown requires current craft receipt and mint' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_twisted_water_catch_admit() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_twisted_water_catch_admit() TO oteryn_game_runtime;
CREATE TRIGGER game_twisted_water_catch_admission BEFORE INSERT OR UPDATE
 ON game_twisted_water_catches FOR EACH ROW EXECUTE FUNCTION game_twisted_water_catch_admit();

-- Source fishing.lua:106-126 permits a spent tile/worm with no yielded item.
-- Preserve the previous bounds and every existing output exception.
ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check
    CHECK (jsonb_typeof(outputs)='array'
        AND (jsonb_array_length(outputs) BETWEEN 0 AND 4
            OR (consume_source AND recipe IN (
                'oteryn:reward-bag.event_6506', 'oteryn:reward-bag.event_6507',
                'oteryn:reward-bag.event_6508', 'oteryn:reward-bag.event_39704',
                'oteryn:reward-bag.event_39705', 'oteryn:reward-bag.event_39706',
                'oteryn:reward-bag.event_39710')
                AND jsonb_array_length(outputs) BETWEEN 5 AND 14))
        AND (jsonb_array_length(outputs)>0
            OR (NOT consume_source AND recipe='oteryn:gathering.crystal')
            OR (consume_source AND recipe='oteryn:soulpit.entry')
            OR (consume_source AND recipe='oteryn:planting.crystal'
                AND before_item->>'definition_production_key'='oteryn:item.tibia.i13844')
            OR (consume_source AND recipe='oteryn:gathering.crystal'
                AND before_item->>'definition_production_key'='oteryn:item.tibia.i3492')));

DO $migration$
DECLARE body TEXT; begin_at INT; arm TEXT;
BEGIN
    body := pg_get_functiondef('game_item_craft_admit()'::regprocedure);
    begin_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected craft admission body'; END IF;
    arm := $arm$
    IF NEW.consume_source AND NEW.recipe='oteryn:gathering.crystal'
        AND jsonb_array_length(NEW.outputs)=0 AND NOT EXISTS (
            SELECT 1 FROM game_terrain_tool_receipts r
            WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
                AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id
                AND r.intent=NEW.intent AND r.transformed_item_id IN (12558,12559,12560)
        ) THEN RAISE EXCEPTION 'empty fishing yield requires dirty-water terrain receipt' USING ERRCODE='23514'; END IF;
$arm$;
    EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;
