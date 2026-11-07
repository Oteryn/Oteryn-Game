-- Static map fields share the spell cost transaction and existing terrain CAS.
-- Keep every existing physical Item line/outbox completeness guard unchanged.
-- A map placement is not an ItemInstance and has its own immutable CAS receipt.
CREATE FUNCTION game_spell_map_fields_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE body JSONB; fields JSONB; field JSONB;
BEGIN
    body := convert_from(NEW.intent, 'UTF8')::jsonb;
    IF NOT (body ? 'map_fields') THEN RETURN NULL; END IF;
    fields := body->'map_fields';
    IF jsonb_typeof(fields) IS DISTINCT FROM 'array'
       OR jsonb_array_length(fields) <> 1
       OR NEW.created_xact_id <> pg_current_xact_id()
       OR NEW.cause_kind <> 0 OR NEW.runtime_scope_kind <> 1 THEN
        RAISE EXCEPTION 'static map field requires one current player spell placement' USING ERRCODE='23514';
    END IF;
    field := fields->0;
    IF field->>'kind' IS DISTINCT FROM 'remove_map_field'
       OR field->'definition'->>'family' IS DISTINCT FROM 'Item'
       OR field->'definition'->>'key' IS DISTINCT FROM 'oteryn:item.tibia.i' || (field->>'appearance')
       OR COALESCE(field->'definition'->>'revision','') = ''
       OR NOT EXISTS (
            SELECT 1 FROM game_terrain_tool_receipts m
            JOIN game_terrain_tool_transforms t
              ON (t.world_id,t.channel_id,t.placement,t.reset_epoch)
               = (m.world_id,m.channel_id,m.placement,m.reset_epoch)
            WHERE (m.game_session_id,m.command_id,m.character_id,m.world_id,m.channel_id)
                = (NEW.game_session_id,NEW.command_id,NEW.character_id,NEW.world_id,NEW.channel_id)
              AND m.created_xact_id=pg_current_xact_id()
              AND m.tool_item_instance_id IS NULL AND m.transformed_item_id=0
              AND m.intent=NEW.binding
              AND field->'placement'=to_jsonb(ARRAY(SELECT get_byte(m.placement,n) FROM generate_series(0,39)n))
              AND (field->>'reset_epoch')::numeric=m.reset_epoch
              AND (field->>'revision')::numeric+1=t.object_revision
              AND t.state_item_id=0 AND field->>'map_revision'=t.map_revision
              AND field->'content_generation'=to_jsonb(ARRAY(SELECT get_byte(t.content_generation,n) FROM generate_series(0,31)n))
       ) THEN
        RAISE EXCEPTION 'static field removal must commit its exact map CAS with the spell receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_spell_map_fields_proven() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_spell_map_fields_proven AFTER INSERT ON game_spell_item_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_map_fields_proven();
