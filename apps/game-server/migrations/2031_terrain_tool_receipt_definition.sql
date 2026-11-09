-- M01c: USE observers need the source definition at the committed native action,
-- not the tool's later mutable Item state. Historical receipts stay unknown.
ALTER TABLE game_terrain_tool_receipts
    ADD COLUMN tool_definition_family TEXT,
    ADD COLUMN tool_definition_production_key TEXT,
    ADD COLUMN tool_definition_revision_ref TEXT,
    ADD CONSTRAINT game_terrain_tool_definition_shape CHECK (
        (tool_definition_family IS NULL
            AND tool_definition_production_key IS NULL
            AND tool_definition_revision_ref IS NULL)
        OR (tool_definition_family IS NOT NULL
            AND tool_definition_production_key IS NOT NULL
            AND tool_definition_revision_ref IS NOT NULL
            AND octet_length(tool_definition_family) BETWEEN 1 AND 512
            AND octet_length(tool_definition_production_key) BETWEEN 1 AND 512
            AND octet_length(tool_definition_revision_ref) BETWEEN 1 AND 512)
    );

CREATE FUNCTION game_terrain_tool_capture_definition() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $body$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'terrain tool definition snapshot is immutable';
    END IF;
    IF NEW.tool_definition_family IS NOT NULL
        OR NEW.tool_definition_production_key IS NOT NULL
        OR NEW.tool_definition_revision_ref IS NOT NULL THEN
        RAISE EXCEPTION 'terrain tool definition snapshot is database-owned';
    END IF;
    SELECT i.definition_family, i.definition_production_key, i.definition_revision_ref
        INTO NEW.tool_definition_family, NEW.tool_definition_production_key,
             NEW.tool_definition_revision_ref
        FROM public.game_item_instances i
        WHERE i.item_instance_id = NEW.tool_item_instance_id
            AND i.world_id = NEW.world_id
        FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'terrain tool receipt has no source in its world';
    END IF;
    RETURN NEW;
END
$body$;

CREATE TRIGGER game_terrain_tool_definition_capture
    BEFORE INSERT ON game_terrain_tool_receipts
    FOR EACH ROW EXECUTE FUNCTION game_terrain_tool_capture_definition();
CREATE TRIGGER game_terrain_tool_definition_immutable
    BEFORE UPDATE OF tool_definition_family, tool_definition_production_key,
        tool_definition_revision_ref ON game_terrain_tool_receipts
    FOR EACH ROW EXECUTE FUNCTION game_terrain_tool_capture_definition();
