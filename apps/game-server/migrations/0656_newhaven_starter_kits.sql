-- Extend STARTER-1 with atomic per-vocation equipment, backpack and quiver lines.
-- C19a's first-entry main-backpack grant and immutable completion remain unchanged.
CREATE TABLE game_character_starter_kit_receipts (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    template_key TEXT NOT NULL CHECK(template_key IN ('sorcerer','druid','paladin','knight','monk')),
    transaction_id UUID NOT NULL UNIQUE REFERENCES game_character_equipment_receipts(transaction_id),
    content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
    line_count SMALLINT NOT NULL CHECK(line_count BETWEEN 10 AND 14),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_character_starter_kit_lines (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id) DEFERRABLE INITIALLY DEFERRED,
    character_id UUID NOT NULL REFERENCES game_character_starter_kit_receipts(character_id),
    transaction_id UUID NOT NULL REFERENCES game_character_starter_kit_receipts(transaction_id),
    definition_production_key TEXT NOT NULL,
    quantity BIGINT NOT NULL CHECK(quantity BETWEEN 1 AND 100),
    destination SMALLINT NOT NULL CHECK(destination IN(1,2,3,4)),
    slot SMALLINT,
    parent_item_instance_id UUID REFERENCES game_item_instances(item_instance_id) DEFERRABLE INITIALLY DEFERRED,
    placement_ordinal NUMERIC(20,0),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK((destination=1 AND slot IN(1,4,5,6,7,8,10) AND parent_item_instance_id IS NULL AND placement_ordinal IS NULL)
       OR (destination IN(2,3) AND slot IS NULL AND parent_item_instance_id IS NOT NULL AND placement_ordinal>0)
       OR (destination=4 AND slot IS NULL AND parent_item_instance_id IS NULL AND placement_ordinal IS NULL))
);
CREATE INDEX game_character_starter_kit_lines_character ON game_character_starter_kit_lines(character_id);
CREATE INDEX game_character_starter_kit_lines_parent ON game_character_starter_kit_lines(parent_item_instance_id);
CREATE INDEX game_character_starter_kit_lines_transaction ON game_character_starter_kit_lines(transaction_id);
REVOKE ALL ON game_character_starter_kit_receipts,game_character_starter_kit_lines FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_starter_kit_receipts,game_character_starter_kit_lines TO oteryn_game_runtime;

-- New batch receipt operation, under the same equipment state/audit owner and FK.
DO $$ DECLARE c RECORD; BEGIN
    FOR c IN SELECT conname,pg_get_constraintdef(oid) AS definition FROM pg_constraint
        WHERE conrelid='game_character_equipment_receipts'::regclass AND contype='c'
    LOOP
        IF c.definition LIKE '%operation%' THEN
            EXECUTE format('ALTER TABLE game_character_equipment_receipts DROP CONSTRAINT %I',c.conname);
            EXECUTE format('ALTER TABLE game_character_equipment_receipts ADD CONSTRAINT %I %s',c.conname,
                'CHECK (operation=3 OR ' || substr(c.definition,7) || ')');
        END IF;
    END LOOP;
END $$;
ALTER TABLE game_character_equipment_receipts ADD CONSTRAINT game_newhaven_equipment_batch_shape CHECK(
    operation<>3 OR (item_instance_id IS NULL AND state_revision_before IS NULL
    AND from_slot IS NULL AND to_slot IS NULL AND backpack_item_instance_id IS NULL
    AND backpack_ordinal IS NULL AND swap_item_instance_id IS NULL
    AND mode_before IS NOT DISTINCT FROM mode_after));

CREATE FUNCTION game_item_newhaven_starter_current(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
    SELECT EXISTS(SELECT 1 FROM game_character_starter_kit_lines l
      JOIN game_character_starter_kit_receipts r USING(character_id,transaction_id)
      WHERE l.item_instance_id=p_item AND l.created_xact_id=pg_current_xact_id()
        AND r.created_xact_id=pg_current_xact_id())
$$;

CREATE FUNCTION game_newhaven_starter_recipe(p_vocation TEXT)
RETURNS TABLE(definition_production_key TEXT,quantity BIGINT,destination SMALLINT,slot SMALLINT)
LANGUAGE sql IMMUTABLE SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
    SELECT v.definition_production_key,v.quantity,v.destination,v.slot FROM (VALUES
('sorcerer','oteryn:item.tibia.i3355',1::bigint,1::smallint,1::smallint),
('sorcerer','oteryn:item.tibia.i3561',1::bigint,1::smallint,4::smallint),
('sorcerer','oteryn:item.tibia.i3559',1::bigint,1::smallint,7::smallint),
('sorcerer','oteryn:item.tibia.i3552',1::bigint,1::smallint,8::smallint),
('sorcerer','oteryn:item.tibia.i34017',1::bigint,1::smallint,10::smallint),
('sorcerer','oteryn:item.tibia.i21400',1::bigint,1::smallint,6::smallint),
('sorcerer','oteryn:item.tibia.i21348',1::bigint,1::smallint,5::smallint),
('sorcerer','oteryn:item.tibia.i268',10::bigint,2::smallint,NULL::smallint),
('sorcerer','oteryn:item.tibia.i266',2::bigint,2::smallint,NULL::smallint),
('sorcerer','oteryn:item.tibia.i3031',50::bigint,2::smallint,NULL::smallint),
('sorcerer','oteryn:item.tibia.i3578',1::bigint,2::smallint,NULL::smallint),
('sorcerer','oteryn:item.tibia.i21351',1::bigint,2::smallint,NULL::smallint),
('druid','oteryn:item.tibia.i3355',1::bigint,1::smallint,1::smallint),
('druid','oteryn:item.tibia.i3561',1::bigint,1::smallint,4::smallint),
('druid','oteryn:item.tibia.i3559',1::bigint,1::smallint,7::smallint),
('druid','oteryn:item.tibia.i3552',1::bigint,1::smallint,8::smallint),
('druid','oteryn:item.tibia.i34017',1::bigint,1::smallint,10::smallint),
('druid','oteryn:item.tibia.i21400',1::bigint,1::smallint,6::smallint),
('druid','oteryn:item.tibia.i21350',1::bigint,1::smallint,5::smallint),
('druid','oteryn:item.tibia.i268',10::bigint,2::smallint,NULL::smallint),
('druid','oteryn:item.tibia.i266',2::bigint,2::smallint,NULL::smallint),
('druid','oteryn:item.tibia.i3031',50::bigint,2::smallint,NULL::smallint),
('druid','oteryn:item.tibia.i3578',1::bigint,2::smallint,NULL::smallint),
('druid','oteryn:item.tibia.i21351',1::bigint,2::smallint,NULL::smallint),
('paladin','oteryn:item.tibia.i3355',1::bigint,1::smallint,1::smallint),
('paladin','oteryn:item.tibia.i3561',1::bigint,1::smallint,4::smallint),
('paladin','oteryn:item.tibia.i3559',1::bigint,1::smallint,7::smallint),
('paladin','oteryn:item.tibia.i3552',1::bigint,1::smallint,8::smallint),
('paladin','oteryn:item.tibia.i34017',1::bigint,1::smallint,10::smallint),
('paladin','oteryn:item.tibia.i3350',1::bigint,1::smallint,5::smallint),
('paladin','oteryn:item.tibia.i35562',1::bigint,1::smallint,6::smallint),
('paladin','oteryn:item.tibia.i268',7::bigint,2::smallint,NULL::smallint),
('paladin','oteryn:item.tibia.i266',5::bigint,2::smallint,NULL::smallint),
('paladin','oteryn:item.tibia.i3031',50::bigint,2::smallint,NULL::smallint),
('paladin','oteryn:item.tibia.i3578',1::bigint,2::smallint,NULL::smallint),
('paladin','oteryn:item.tibia.i3447',100::bigint,3::smallint,NULL::smallint),
('paladin','oteryn:item.tibia.i3447',100::bigint,3::smallint,NULL::smallint),
('knight','oteryn:item.tibia.i3355',1::bigint,1::smallint,1::smallint),
('knight','oteryn:item.tibia.i3561',1::bigint,1::smallint,4::smallint),
('knight','oteryn:item.tibia.i3559',1::bigint,1::smallint,7::smallint),
('knight','oteryn:item.tibia.i3552',1::bigint,1::smallint,8::smallint),
('knight','oteryn:item.tibia.i34017',1::bigint,1::smallint,10::smallint),
('knight','oteryn:item.tibia.i3272',1::bigint,1::smallint,5::smallint),
('knight','oteryn:item.tibia.i3412',1::bigint,1::smallint,6::smallint),
('knight','oteryn:item.tibia.i268',2::bigint,2::smallint,NULL::smallint),
('knight','oteryn:item.tibia.i266',10::bigint,2::smallint,NULL::smallint),
('knight','oteryn:item.tibia.i3031',50::bigint,2::smallint,NULL::smallint),
('knight','oteryn:item.tibia.i3578',1::bigint,2::smallint,NULL::smallint),
('monk','oteryn:item.tibia.i3355',1::bigint,1::smallint,1::smallint),
('monk','oteryn:item.tibia.i3561',1::bigint,1::smallint,4::smallint),
('monk','oteryn:item.tibia.i3559',1::bigint,1::smallint,7::smallint),
('monk','oteryn:item.tibia.i3552',1::bigint,1::smallint,8::smallint),
('monk','oteryn:item.tibia.i34017',1::bigint,1::smallint,10::smallint),
('monk','oteryn:item.tibia.i50166',1::bigint,1::smallint,5::smallint),
('monk','oteryn:item.tibia.i268',5::bigint,2::smallint,NULL::smallint),
('monk','oteryn:item.tibia.i266',7::bigint,2::smallint,NULL::smallint),
('monk','oteryn:item.tibia.i3031',50::bigint,2::smallint,NULL::smallint),
('monk','oteryn:item.tibia.i3578',1::bigint,2::smallint,NULL::smallint)) v(vocation,definition_production_key,quantity,destination,slot)
    WHERE v.vocation=p_vocation
$$;

CREATE FUNCTION game_newhaven_starter_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE r game_character_starter_kit_receipts%ROWTYPE; e game_character_equipment_receipts%ROWTYPE; l RECORD;
BEGIN
    IF TG_TABLE_NAME='game_character_equipment_receipts' THEN
        IF NEW.operation<>3 THEN RETURN NULL; END IF;
        SELECT * INTO r FROM game_character_starter_kit_receipts WHERE transaction_id=NEW.transaction_id;
    ELSE
        SELECT * INTO r FROM game_character_starter_kit_receipts WHERE character_id=NEW.character_id;
    END IF;
    SELECT * INTO e FROM game_character_equipment_receipts WHERE transaction_id=r.transaction_id;
    IF r.character_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
       OR e.operation IS DISTINCT FROM 3 OR e.character_id IS DISTINCT FROM r.character_id
       OR e.content_digest IS DISTINCT FROM r.content_digest
       OR NOT EXISTS(SELECT 1 FROM game_character_build_state b WHERE b.character_id=r.character_id AND b.vocation=r.template_key)
       OR (SELECT count(*) FROM game_character_starter_kit_lines WHERE character_id=r.character_id)<>r.line_count
    THEN RAISE EXCEPTION 'starter kit lacks exact batch receipt/build' USING ERRCODE='23514'; END IF;
    IF EXISTS(
        (SELECT definition_production_key,quantity,destination,slot FROM game_newhaven_starter_recipe(r.template_key)
         EXCEPT ALL SELECT definition_production_key,quantity,destination,slot FROM game_character_starter_kit_lines WHERE character_id=r.character_id AND destination<>4)
        UNION ALL
        (SELECT definition_production_key,quantity,destination,slot FROM game_character_starter_kit_lines WHERE character_id=r.character_id AND destination<>4
         EXCEPT ALL SELECT definition_production_key,quantity,destination,slot FROM game_newhaven_starter_recipe(r.template_key))
       ) OR EXISTS(SELECT 1 FROM game_character_starter_kit_lines WHERE character_id=r.character_id AND destination=4
           AND (definition_production_key<>'oteryn:item.tibia.i2854' OR quantity<>1))
       OR (SELECT count(*) FROM game_character_starter_kit_lines WHERE character_id=r.character_id AND destination=4)>1
    THEN RAISE EXCEPTION 'starter kit does not match its exact vocation recipe' USING ERRCODE='23514'; END IF;
    FOR l IN SELECT * FROM game_character_starter_kit_lines WHERE character_id=r.character_id LOOP
        IF l.transaction_id<>r.transaction_id OR l.created_xact_id<>pg_current_xact_id()
           OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=l.item_instance_id
                AND i.definition_family='Item' AND i.definition_production_key=l.definition_production_key
                AND i.definition_revision_ref='definition-r1' AND i.quantity=l.quantity AND i.lifecycle=1
                AND i.world_id=e.world_id AND i.minted_transaction_id=r.transaction_id
                AND i.last_transaction_id IS NULL AND i.state_revision=1)
           OR (l.destination=1 AND NOT EXISTS(SELECT 1 FROM game_character_equipment_slots s
                WHERE s.item_instance_id=l.item_instance_id AND s.character_id=r.character_id
                AND s.slot=l.slot AND s.world_id=e.world_id AND s.placed_transaction_id=r.transaction_id))
           OR (l.destination=2 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries s
                WHERE s.item_instance_id=l.item_instance_id AND s.character_id=r.character_id
                AND s.parent_item_instance_id=l.parent_item_instance_id AND s.placement_ordinal=l.placement_ordinal
                AND s.world_id=e.world_id AND s.placed_transaction_id=r.transaction_id))
           OR (l.destination=3 AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries s
                WHERE s.item_instance_id=l.item_instance_id AND s.parent_item_instance_id=l.parent_item_instance_id
                AND s.placement_ordinal=l.placement_ordinal AND s.world_id=e.world_id AND s.placed_transaction_id=r.transaction_id))
           OR (l.destination=4 AND NOT EXISTS(SELECT 1 FROM game_item_container_slots s
                WHERE s.item_instance_id=l.item_instance_id AND s.character_id=r.character_id
                AND s.world_id=e.world_id AND s.placed_transaction_id=r.transaction_id))
        THEN RAISE EXCEPTION 'starter kit line lacks exact fresh custody' USING ERRCODE='23514'; END IF;
    END LOOP;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_newhaven_starter_batch_proven AFTER INSERT ON game_character_equipment_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_newhaven_starter_proven();
CREATE CONSTRAINT TRIGGER game_newhaven_starter_proven AFTER INSERT ON game_character_starter_kit_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_newhaven_starter_proven();
CREATE CONSTRAINT TRIGGER game_newhaven_starter_line_proven AFTER INSERT ON game_character_starter_kit_lines
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_newhaven_starter_proven();

-- Preserve each current guard body in full and prepend only the exact same-xact kit arm.
-- The deferred kit proof above checks every skipped item's complete fresh custody.
DO $$ DECLARE n TEXT; body TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_equipment_slot_proven','game_item_nested_entry_proven','game_item_placement_proven'] LOOP
        SELECT pg_get_functiondef(to_regprocedure(n||'()')) INTO body;
        body := regexp_replace(body,'BEGIN',E'BEGIN\n    IF TG_OP=''INSERT'' AND game_item_newhaven_starter_current(NEW.item_instance_id) THEN RETURN NULL; END IF;', 'i');
        EXECUTE body;
    END LOOP;
    SELECT pg_get_functiondef('game_item_starter_grant_current(uuid)'::regprocedure) INTO body;
    body := replace(body,'SELECT EXISTS (','SELECT game_item_newhaven_starter_current(p_item) OR EXISTS (');
    EXECUTE body;
END $$;

CREATE TRIGGER game_starter_kit_receipts_immutable BEFORE UPDATE OR DELETE ON game_character_starter_kit_receipts
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_starter_kit_lines_immutable BEFORE UPDATE OR DELETE ON game_character_starter_kit_lines
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_starter_kit_receipts_no_truncate BEFORE TRUNCATE ON game_character_starter_kit_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_starter_kit_lines_no_truncate BEFORE TRUNCATE ON game_character_starter_kit_lines
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_newhaven_starter_current(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_newhaven_starter_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_item_newhaven_starter_current(UUID),game_newhaven_starter_proven(),game_newhaven_starter_recipe(TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_newhaven_starter_current(UUID) TO oteryn_game_runtime;
