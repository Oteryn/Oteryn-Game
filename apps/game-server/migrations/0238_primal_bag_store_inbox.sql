-- PRIMALBAG: narrow, audited Store Inbox delivery for reward-bag craft outputs.
INSERT INTO game_character_inbox_cause_kinds(cause_kind)
VALUES ('ITEM_CRAFT_REWARD') ON CONFLICT DO NOTHING;

CREATE FUNCTION game_item_craft_inbox_deliver(
    p_item UUID, p_world UUID, p_character UUID, p_transaction UUID
) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE
    valid_output BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1
          FROM game_item_craft_records r
          CROSS JOIN LATERAL jsonb_array_elements(r.outputs) output
         WHERE r.transaction_id = p_transaction
           AND r.world_id = p_world
           AND r.character_id = p_character
           AND r.created_xact_id = pg_current_xact_id()
           AND r.recipe LIKE 'oteryn:reward-bag.%'
           AND (output->>'item')::uuid = p_item
           AND output->>'destination' = 'store_inbox'
    ) INTO valid_output;
    IF NOT valid_output THEN
        RAISE EXCEPTION 'item craft Inbox delivery lacks its current reward-bag record'
            USING ERRCODE = '23514';
    END IF;

    PERFORM game_character_inbox_deliver(
        p_item, p_character, p_world, 'ITEM_CRAFT_REWARD', p_transaction::text
    );
END;
$$;

DO $$ BEGIN
    EXECUTE format(
        'ALTER FUNCTION game_item_craft_inbox_deliver(UUID,UUID,UUID,UUID) SET search_path = pg_catalog, %I, pg_temp',
        current_schema()
    );
END $$;
REVOKE ALL ON FUNCTION game_item_craft_inbox_deliver(UUID,UUID,UUID,UUID)
 FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_item_craft_inbox_deliver(UUID,UUID,UUID,UUID)
 TO oteryn_game_runtime;

-- Extend CRAFT-1's completion proof: every declared output still has exactly
-- one custody location, selected by its audit-bound destination.
CREATE OR REPLACE FUNCTION game_item_craft_complete(p_transaction UUID) RETURNS void
LANGUAGE plpgsql AS $$
DECLARE r game_item_craft_records%ROWTYPE; i game_item_instances%ROWTYPE; o JSONB;
BEGIN
 SELECT * INTO r FROM game_item_craft_records WHERE transaction_id=p_transaction;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
    OR i.quantity<>(r.before_item->>'quantity')::bigint-1
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
    OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       (r.before_item-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
    OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id AND character_id=r.character_id AND parent_item_instance_id=r.parent_item_instance_id AND placement_ordinal=r.source_ordinal))
    OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>jsonb_array_length(r.outputs)
    OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1
    OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>20
    OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
    OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id AND item_instance_id=r.source_item_instance_id
                   AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256 AND created_xact_id=pg_current_xact_id()
                   AND transaction_ordinal=1 AND transaction_count=1 AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'incomplete atomic craft' USING ERRCODE='23514'; END IF;
 FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  IF o->>'destination' NOT IN ('backpack','store_inbox') OR NOT EXISTS(
      SELECT 1 FROM game_item_instances n
       WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id AND n.lifecycle=1
         AND n.quantity=(o->>'count')::bigint AND n.definition_family='Item'
         AND n.definition_production_key=o->>'key' AND n.definition_revision_ref=o->>'revision'
         AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL)
     OR (o->>'destination'='backpack' AND NOT EXISTS(
      SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=(o->>'item')::uuid
       AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.parent_item_instance_id
       AND e.placement_ordinal=(o->>'ordinal')::numeric AND e.placed_transaction_id=r.transaction_id))
     OR (o->>'destination'='store_inbox' AND NOT EXISTS(
      SELECT 1 FROM game_character_inbox_deliveries d WHERE d.item_instance_id=(o->>'item')::uuid
       AND d.world_id=r.world_id AND d.character_id=r.character_id AND d.cause_kind='ITEM_CRAFT_REWARD'
       AND d.cause_ref=r.transaction_id::text AND d.created_xact_id=pg_current_xact_id()))
  THEN RAISE EXCEPTION 'craft output or custody mismatch' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_item_craft_complete(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
