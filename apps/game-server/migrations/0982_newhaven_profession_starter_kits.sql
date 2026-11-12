-- Owner-approved SRX-2 compatibility: only registered historical starter revisions
-- may differ; profile/rules/content and every session/authority fence remain current.
ALTER TABLE game_character_interpretations ADD CONSTRAINT game_character_interpretation_starter_identity
    UNIQUE(interpretation_revision,starter_template_revision);
CREATE TABLE game_world_registered_starter_revisions (
    world_id UUID NOT NULL,
    starter_template_revision TEXT NOT NULL,
    interpretation_revision BIGINT NOT NULL,
    PRIMARY KEY(world_id,starter_template_revision),
    FOREIGN KEY(interpretation_revision,starter_template_revision)
        REFERENCES game_character_interpretations(interpretation_revision,starter_template_revision)
);
CREATE INDEX game_world_registered_starter_revision_origin
    ON game_world_registered_starter_revisions(interpretation_revision,starter_template_revision);
REVOKE ALL ON game_world_registered_starter_revisions FROM PUBLIC;
GRANT SELECT ON game_world_registered_starter_revisions TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_world_registered_starter_revisions TO oteryn_game_control;

-- Never infer registration from arbitrary root revisions. The current trusted
-- interpretation is registered for existing World assignment identities only.
INSERT INTO game_world_registered_starter_revisions
SELECT DISTINCT a.world_id,i.starter_template_revision,i.interpretation_revision
FROM game_runtime_scope_assignments a CROSS JOIN LATERAL (
    SELECT starter_template_revision,interpretation_revision FROM game_character_interpretations
    ORDER BY interpretation_revision DESC LIMIT 1
) i WHERE a.scope_kind=1 ON CONFLICT DO NOTHING;

CREATE FUNCTION game_register_configured_starter_revisions() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    -- Both the new current token and the token it replaces are trusted configuration
    -- facts. Runtime cannot insert here or invoke a free-form registration API.
    INSERT INTO game_world_registered_starter_revisions
    SELECT DISTINCT a.world_id,i.starter_template_revision,i.interpretation_revision
    FROM game_runtime_scope_assignments a CROSS JOIN LATERAL (
        SELECT starter_template_revision,interpretation_revision FROM game_character_interpretations
        WHERE interpretation_revision<=NEW.interpretation_revision
        ORDER BY interpretation_revision DESC LIMIT 2
    ) i WHERE a.scope_kind=1 ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_register_configured_starter_revisions() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_register_configured_starter_revisions() FROM PUBLIC;
CREATE TRIGGER game_register_configured_starter_revisions
    AFTER INSERT ON game_character_interpretations
    FOR EACH ROW EXECUTE FUNCTION game_register_configured_starter_revisions();

-- C19b: extend the existing starter receipt owner to Newhaven profession kits.
-- destination 0 keeps the original backpack-only contract; 1 equipment, 2 backpack,
-- 3 nested quiver. Every mint is proved against its exact same-transaction destination.
ALTER TABLE game_character_starter_grants
    ADD COLUMN kit_destination SMALLINT NOT NULL DEFAULT 0 CHECK(kit_destination BETWEEN 0 AND 3),
    ADD COLUMN kit_slot SMALLINT CHECK(kit_slot IN(1,2,3,4,5,6,7,8,10)),
    ADD COLUMN kit_parent UUID REFERENCES game_item_instances(item_instance_id),
    ADD COLUMN kit_ordinal NUMERIC(20,0) CHECK(kit_ordinal BETWEEN 1 AND 18446744073709551615),
    ADD COLUMN kit_creation_revision TEXT,
    ADD COLUMN kit_equipment_revision_before NUMERIC(20,0) CHECK(kit_equipment_revision_before BETWEEN 1 AND 18446744073709551614),
    ADD CONSTRAINT newhaven_starter_destination CHECK(
        (kit_destination=0 AND kit_slot IS NULL AND kit_parent IS NULL AND kit_ordinal IS NULL AND kit_equipment_revision_before IS NULL)
        OR (outcome=1 AND kit_destination=1 AND kit_slot IS NOT NULL AND kit_parent IS NULL AND kit_ordinal IS NULL)
        OR (outcome=1 AND kit_destination IN(2,3) AND kit_slot IS NULL AND kit_parent IS NOT NULL AND kit_ordinal IS NOT NULL));
ALTER TABLE game_character_starter_grants ADD CONSTRAINT newhaven_starter_creation_revision CHECK(
    (kit_destination=0 AND kit_creation_revision IS NULL)
    OR (kit_destination>0 AND kit_creation_revision IS NOT NULL AND kit_creation_revision='newhaven-starter-r1'));
CREATE INDEX game_character_starter_grants_kit_parent ON game_character_starter_grants(kit_parent);

-- 0976 already replaced the old equipment-receipt FK with exact deferred proof
-- arms. Preserve that complete guard chain and add Newhaven's exact destination arms.

-- Extend the installed guard chains instead of replacing later owners' admitted arms.
DO $migration$
DECLARE definition TEXT; name TEXT; addition TEXT;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_equipment_slot_proven','game_item_placement_proven','game_item_nested_entry_proven'] LOOP
        SELECT pg_get_functiondef((name || '()')::regprocedure) INTO definition;
        addition:=CASE name
            WHEN 'game_equipment_slot_proven' THEN $arm$BEGIN
    IF TG_OP='INSERT' THEN
        IF EXISTS(SELECT 1 FROM game_character_starter_grants g
            WHERE g.kit_destination=1 AND g.item_instance_id=NEW.item_instance_id
              AND g.character_id=NEW.character_id AND g.kit_slot=NEW.slot
              AND g.world_id=NEW.world_id AND g.transaction_id=NEW.placed_transaction_id
              AND g.outcome=1 AND g.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    END IF;$arm$
            WHEN 'game_item_placement_proven' THEN $arm$BEGIN
    IF TG_OP='INSERT' AND TG_TABLE_NAME='game_item_container_entries' THEN
        IF EXISTS(SELECT 1 FROM game_character_starter_grants g
            WHERE g.kit_destination=2 AND g.item_instance_id=NEW.item_instance_id
              AND g.character_id=NEW.character_id AND g.kit_parent=NEW.parent_item_instance_id
              AND g.kit_ordinal=NEW.placement_ordinal AND g.world_id=NEW.world_id
              AND g.transaction_id=NEW.placed_transaction_id
              AND g.outcome=1 AND g.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    END IF;$arm$
            ELSE $arm$BEGIN
    IF TG_OP='INSERT' THEN
        IF EXISTS(SELECT 1 FROM game_character_starter_grants g
            WHERE g.kit_destination=3 AND g.item_instance_id=NEW.item_instance_id
              AND g.kit_parent=NEW.parent_item_instance_id AND g.kit_ordinal=NEW.placement_ordinal
              AND g.world_id=NEW.world_id AND g.transaction_id=NEW.placed_transaction_id
              AND g.outcome=1 AND g.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    END IF;$arm$ END;
        EXECUTE overlay(definition PLACING addition FROM position('BEGIN' IN definition) FOR 5);
    END LOOP;
    SELECT pg_get_functiondef('game_equipment_state_proven()'::regprocedure) INTO definition;
    addition:=$arm$BEGIN
    IF EXISTS(SELECT 1 FROM game_character_starter_grants g
        WHERE g.character_id=NEW.character_id AND g.transaction_id=NEW.last_transaction_id
          AND g.kit_destination>0 AND g.kit_equipment_revision_before=OLD.revision
          AND NEW.revision=OLD.revision+1
          AND NEW.combat_mode IS NOT DISTINCT FROM OLD.combat_mode
          AND g.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;$arm$;
    EXECUTE overlay(definition PLACING addition FROM position('BEGIN' IN definition) FOR 5);
    SELECT pg_get_functiondef('game_character_starter_grant_proven()'::regprocedure) INTO definition;
    addition:=$arm$BEGIN
    IF NEW.kit_destination>0 THEN
        IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
            SELECT 1 FROM game_item_instances kit_item JOIN game_character_roots c ON c.character_id=NEW.character_id
            WHERE kit_item.item_instance_id=NEW.item_instance_id AND c.world_id=NEW.world_id
              AND c.starter_template_revision=NEW.kit_creation_revision
              AND NEW.kit_creation_revision='newhaven-starter-r1'
              AND kit_item.world_id=NEW.world_id AND kit_item.lifecycle=1 AND kit_item.quantity=NEW.quantity
              AND kit_item.minted_transaction_id=NEW.transaction_id AND kit_item.last_transaction_id IS NULL AND kit_item.state_revision=1
              AND (kit_item.definition_family,kit_item.definition_production_key,kit_item.definition_revision_ref)=
                  (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)) THEN
            RAISE EXCEPTION 'Newhaven starter mint does not match its receipt' USING ERRCODE='23514';
        END IF;
        IF NOT ((NEW.kit_destination=1 AND EXISTS(SELECT 1 FROM game_character_equipment_slots s
            WHERE s.character_id=NEW.character_id AND s.slot=NEW.kit_slot AND s.item_instance_id=NEW.item_instance_id
              AND s.world_id=NEW.world_id AND s.placed_transaction_id=NEW.transaction_id))
          OR (NEW.kit_destination=2 AND EXISTS(SELECT 1 FROM game_item_container_entries e
            WHERE e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.kit_parent
              AND e.placement_ordinal=NEW.kit_ordinal AND e.item_instance_id=NEW.item_instance_id
              AND e.world_id=NEW.world_id AND e.placed_transaction_id=NEW.transaction_id))
          OR (NEW.kit_destination=3 AND EXISTS(SELECT 1 FROM game_item_nested_entries e
            CROSS JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) o
            WHERE o.owner_character=NEW.character_id AND e.parent_item_instance_id=NEW.kit_parent
              AND e.placement_ordinal=NEW.kit_ordinal AND e.item_instance_id=NEW.item_instance_id
              AND e.world_id=NEW.world_id AND e.placed_transaction_id=NEW.transaction_id))) THEN
            RAISE EXCEPTION 'Newhaven starter destination does not match its receipt' USING ERRCODE='23514';
        END IF;
        PERFORM game_item_location_exclusive(NEW.item_instance_id);
        RETURN NULL;
    END IF;$arm$;
    EXECUTE overlay(definition PLACING addition FROM position('BEGIN' IN definition) FOR 5);
END;
$migration$;
