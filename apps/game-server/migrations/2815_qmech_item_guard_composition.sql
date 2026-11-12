-- Compose pending QMECH owners after the accepted audit optimizations.
-- Earlier QMECH migrations create their objects but defer Item-guard changes
-- until here, so both fresh and accepted-main upgrades retain the exact
-- accepted2808/2809 history before this forward-only composition.
-- Every original fallback branch remains byte-identical; row images stay
-- invocation-local and the terminal shortcut retains all deferred proofs.
DO $migration$
DECLARE
    definition TEXT;
    original_source TEXT;
    source TEXT;
    original_oid OID;
    original_owner OID;
    original_security BOOLEAN;
    original_config TEXT[];
    original_acl ACLITEM[];
    anchor TEXT;
    replacement TEXT;
BEGIN
    SELECT p.oid, p.proowner, p.prosecdef, p.proconfig, p.proacl,
           pg_get_functiondef(p.oid), p.prosrc
      INTO original_oid, original_owner, original_security, original_config,
           original_acl, definition, original_source
      FROM pg_proc p WHERE p.oid='game_item_instance_guard()'::regprocedure;
    IF encode(sha256(convert_to(original_source,'UTF8')),'hex')
           <> '5cfab2acfadeac9485fb3c34030eaf736f516fd65ea243e2a0d16538df4a3b37'
       OR (length(definition)-length(replace(definition,original_source,'')))
              /length(original_source)<>1 THEN
        RAISE EXCEPTION 'unexpected accepted Item guard before QMECH composition';
    END IF;
    source := original_source;

    -- terminal receipt classification.
    anchor := $qmech_edit_1$        UNION ALL SELECT 1 FROM game_spell_field_temporal_receipts WHERE transaction_id=NEW.last_transaction_id$qmech_edit_1$;
    replacement := $qmech_edit_1$        -- QMECH retirement owners retain their exact before-image, same-XID
        -- and quantity checks in the untouched original branches below.
        UNION ALL SELECT 1 FROM game_elemental_arrival_receipts WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_elemental_loot_retire_receipts WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_elemental_altar_receipts WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_dreamer_ring_offerings WHERE transaction_id=NEW.last_transaction_id
        UNION ALL SELECT 1 FROM game_spell_field_temporal_receipts WHERE transaction_id=NEW.last_transaction_id$qmech_edit_1$;
    IF (length(source)-length(replace(source,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected QMECH composition anchor: terminal receipt classification';
    END IF;
    source := replace(source,anchor,replacement);

    -- QMECH owner scopes.
    anchor := $qmech_edit_2$

  IF TG_OP='UPDATE' AND OLD.definition_family='Item'
     AND OLD.definition_production_key='oteryn:item.tibia.i51442'$qmech_edit_2$;
    replacement := $qmech_edit_2$

    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_dreamer_ring_offerings r
     WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=OLD.item_instance_id
      AND r.created_xact_id=pg_current_xact_id() AND r.before_item=v_instance_guard_old_json) THEN
     IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR (v_instance_guard_new_json-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       (v_instance_guard_old_json-ARRAY['quantity','lifecycle','last_transaction_id'])
     THEN RAISE EXCEPTION 'Dreamer offering retires exactly one inactive ring' USING ERRCODE='23514'; END IF;
     NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
    END IF;


     IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_elemental_altar_receipts r,LATERAL jsonb_array_elements(r.inputs) v
       WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND v->'item'=v_instance_guard_old_json) THEN
       IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR (v_instance_guard_new_json-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
          (v_instance_guard_old_json-ARRAY['quantity','lifecycle','last_transaction_id']) THEN
         RAISE EXCEPTION 'altar retires whole exact corpse members' USING ERRCODE='23514'; END IF;
       NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
     END IF;


     IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_elemental_loot_retire_receipts r
       WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND r.item_before=v_instance_guard_old_json) THEN
       IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR (v_instance_guard_new_json-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
         (v_instance_guard_old_json-ARRAY['quantity','lifecycle','last_transaction_id']) THEN
         RAISE EXCEPTION 'elemental loot retires the exact whole entry' USING ERRCODE='23514'; END IF;
       NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
     END IF;


     IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_elemental_arrival_receipts r WHERE r.transaction_id=NEW.last_transaction_id
       AND r.created_xact_id=pg_current_xact_id() AND r.corpse_before->'item'=v_instance_guard_old_json) THEN
       IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR (v_instance_guard_new_json-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
         (v_instance_guard_old_json-ARRAY['quantity','lifecycle','last_transaction_id']) THEN RAISE EXCEPTION 'elemental arrival retires its exact whole corpse' USING ERRCODE='23514'; END IF;
       NEW.state_revision:=OLD.state_revision+1; RETURN NEW;
     END IF;

 IF TG_OP='UPDATE' THEN
  DECLARE candidate game_item_instances%ROWTYPE;
  BEGIN
   candidate:=NEW;
   candidate.state_revision:=OLD.state_revision+1;
   IF game_rotten_move_transform_before(OLD,candidate) OR game_w01_death_transform_before(OLD,candidate) THEN
    NEW:=candidate;
    INSERT INTO game_item_transfer_quantity_evidence(item_instance_id,transaction_id,quantity_before) VALUES(OLD.item_instance_id,NEW.last_transaction_id,OLD.quantity);
    RETURN NEW;
   END IF;
  END;
 END IF;

  IF TG_OP='UPDATE' AND OLD.definition_family='Item'
     AND OLD.definition_production_key='oteryn:item.tibia.i51442'$qmech_edit_2$;
    IF (length(source)-length(replace(source,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected QMECH composition anchor: QMECH owner scopes';
    END IF;
    source := replace(source,anchor,replacement);

    -- captured ingredient debit.
    anchor := $qmech_edit_3$AND r.ingredient_before_item=v_instance_guard_old_json AND r.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.quantity<>OLD.quantity-1$qmech_edit_3$;
    replacement := $qmech_edit_3$AND r.ingredient_before_item=v_instance_guard_old_json AND r.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.quantity<>OLD.quantity-(SELECT r.ingredient_count FROM game_item_craft_records r
        WHERE r.ingredient_item_instance_id=OLD.item_instance_id
          AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id())$qmech_edit_3$;
    IF (length(source)-length(replace(source,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected QMECH composition anchor: captured ingredient debit';
    END IF;
    source := replace(source,anchor,replacement);

    -- captured debit refusal text.
    anchor := $qmech_edit_4$craft decrements exactly one ingredient unit$qmech_edit_4$;
    replacement := $qmech_edit_4$craft ingredient debit differs from its receipt$qmech_edit_4$;
    IF (length(source)-length(replace(source,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected QMECH composition anchor: captured debit refusal text';
    END IF;
    source := replace(source,anchor,replacement);

    -- Pin the complete reviewed successor, including unchanged old owners.
    IF encode(sha256(convert_to(source,'UTF8')),'hex')
           <> '7980797b9d2a45234462062c4bd59e1f184cdbdc626f26713c4eacc1f8ca7bb0' THEN
        RAISE EXCEPTION 'unexpected composed QMECH Item guard';
    END IF;
    EXECUTE replace(definition,original_source,source);

    IF NOT EXISTS(
        SELECT 1 FROM pg_proc p
         WHERE p.oid=original_oid
           AND p.proowner=original_owner
           AND p.prosecdef IS NOT DISTINCT FROM original_security
           AND p.proconfig IS NOT DISTINCT FROM original_config
           AND p.proacl IS NOT DISTINCT FROM original_acl
           AND encode(sha256(convert_to(p.prosrc,'UTF8')),'hex')
               = '7980797b9d2a45234462062c4bd59e1f184cdbdc626f26713c4eacc1f8ca7bb0'
    ) THEN
        RAISE EXCEPTION 'QMECH Item guard metadata or body changed during composition';
    END IF;
END $migration$;
