-- Closed historical successors of the original onMove MINT. No Item identity changes.
WITH RECURSIVE source_move_paths AS (
 SELECT m.item_instance_id,m.transaction_id AS mint_transaction_id,
  m.definition_family,m.definition_production_key,m.definition_revision,
  m.state_revision_before+1 AS state_revision,m.transaction_id AS last_transaction_id,
  ARRAY[m.transaction_id] AS transactions,0 AS depth,NULL::bigint AS stack_ordinal
 FROM game_spell_item_lines m JOIN game_spell_item_receipts r USING(transaction_id)
 JOIN game_item_instances i ON i.item_instance_id=m.item_instance_id AND i.minted_transaction_id=m.transaction_id
 JOIN game_spell_item_audit_outbox a USING(transaction_id)
 WHERE m.operation_kind=1 AND m.ordinal=1 AND r.operation_count=1 AND r.cause_kind=3
  AND r.runtime_scope_kind=2 AND r.channel_id IS NULL AND m.runtime_scope_kind=2 AND m.channel_id IS NULL
  AND (m.world_id,m.instance_id)=(r.world_id,r.instance_id) AND i.world_id=m.world_id
  AND m.quantity_before=0 AND m.quantity_after=1 AND m.state_revision_before=0 AND m.source_stack_ordinal IS NULL
  AND m.definition_family='Item' AND m.definition_revision='definition-r1'
  AND (m.definition_production_key='oteryn:item.tibia.i43589'
   OR (r.creature_definition_key='oteryn:creature.bakragore' AND m.definition_production_key='oteryn:item.tibia.i43294'))
  AND r.creature_definition_key IN('oteryn:creature.vemiath','oteryn:creature.chagorz','oteryn:creature.bakragore')
  AND r.spell_family='Creature' AND r.spell_production_key=r.creature_definition_key
  AND r.spell_revision='definition-r1' AND r.creature_definition_revision='definition-r1'
  AND r.binding=sha256(r.intent) AND a.envelope=r.intent AND a.envelope_sha256=r.binding
  AND a.event_id=r.event_id AND a.created_xact_id=r.created_xact_id AND m.created_xact_id=r.created_xact_id
  AND a.occurred_at_unix_ms=r.occurred_at_unix_ms
  AND convert_from(r.intent,'UTF8')::jsonb->>'schema'='OTERYN_CREATURE_EVENT_GROUND_CAUSE/v1'
  AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='rotten-blood.footprint.create'
 UNION ALL
 SELECT n.item_instance_id,p.mint_transaction_id,n.definition_family,n.definition_production_key,n.definition_revision,
  n.state_revision_before+1,n.transaction_id,p.transactions||n.transaction_id,p.depth+1,n.source_stack_ordinal
 FROM source_move_paths p
 JOIN game_spell_item_lines m ON m.transaction_id=p.mint_transaction_id AND m.item_instance_id=p.item_instance_id AND m.operation_kind=1
 JOIN game_spell_item_receipts root ON root.transaction_id=m.transaction_id
 JOIN game_spell_item_lines n ON n.item_instance_id=p.item_instance_id AND n.operation_kind=7 AND n.state_revision_before=p.state_revision
 JOIN game_spell_item_receipts r ON r.transaction_id=n.transaction_id
 JOIN game_spell_item_audit_outbox a ON a.transaction_id=r.transaction_id
 JOIN game_item_transfer_quantity_evidence q ON q.transaction_id=n.transaction_id AND q.item_instance_id=n.item_instance_id AND q.quantity_before=1
 WHERE p.depth<2 AND n.source_stack_ordinal>0 AND (p.depth=0 OR n.source_stack_ordinal=p.stack_ordinal) AND n.ordinal=1 AND r.operation_count=1 AND r.cause_kind=3
  AND n.quantity_before=1 AND n.quantity_after=1 AND n.definition_family='Item' AND n.definition_revision='definition-r1'
  AND (n.world_id,n.runtime_scope_kind,n.channel_id,n.instance_id,n.spatial_position,n.map_revision,n.content_revision,n.placement_context,n.content_generation_digest)
   IS NOT DISTINCT FROM (m.world_id,m.runtime_scope_kind,m.channel_id,m.instance_id,m.spatial_position,m.map_revision,m.content_revision,m.placement_context,m.content_generation_digest)
  AND (r.world_id,r.runtime_scope_kind,r.channel_id,r.instance_id,r.ownership_generation,r.catalog_digest,r.creature_definition_key,r.creature_definition_revision,r.creature_map_digest,r.creature_frame_digest,r.creature_source_body_digest)
   IS NOT DISTINCT FROM (root.world_id,root.runtime_scope_kind,root.channel_id,root.instance_id,root.ownership_generation,root.catalog_digest,root.creature_definition_key,root.creature_definition_revision,root.creature_map_digest,root.creature_frame_digest,root.creature_source_body_digest)
  AND r.spell_family='Creature' AND r.spell_production_key=r.creature_definition_key AND r.spell_revision='definition-r1'
  AND r.binding=sha256(r.intent) AND a.envelope=r.intent AND a.envelope_sha256=r.binding
  AND a.event_id=r.event_id AND a.created_xact_id=r.created_xact_id AND n.created_xact_id=r.created_xact_id
  AND a.occurred_at_unix_ms=r.occurred_at_unix_ms
  AND convert_from(r.intent,'UTF8')::jsonb->>'schema'='OTERYN_CREATURE_EVENT_GROUND_CAUSE/v1'
  AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='rotten-blood.footprint.transform'
  AND convert_from(r.intent,'UTF8')::jsonb->'operations'->0->>'kind'='transform_ground'
  AND convert_from(r.intent,'UTF8')::jsonb->'operations'->0->'before'=jsonb_build_object('family',p.definition_family,'key',p.definition_production_key,'revision',p.definition_revision)
  AND convert_from(r.intent,'UTF8')::jsonb->'operations'->0->'definition'=jsonb_build_object('family',n.definition_family,'key',n.definition_production_key,'revision',n.definition_revision)
  AND (convert_from(r.intent,'UTF8')::jsonb->'operations'->0->>'revision')=n.state_revision_before::text
  AND (convert_from(r.intent,'UTF8')::jsonb->'operations'->0->>'ordinal')=n.source_stack_ordinal::text
  AND convert_from(r.intent,'UTF8')::jsonb->'operations'->0->>'quantity'='1'
  AND ((p.definition_production_key='oteryn:item.tibia.i43589' AND n.definition_production_key='oteryn:item.tibia.i43590')
   OR (p.definition_production_key='oteryn:item.tibia.i43590' AND n.definition_production_key=CASE WHEN r.creature_definition_key='oteryn:creature.chagorz' THEN 'oteryn:item.tibia.i43625' ELSE 'oteryn:item.tibia.i43297' END)
   OR (r.creature_definition_key='oteryn:creature.bakragore' AND p.definition_production_key='oteryn:item.tibia.i43294' AND n.definition_production_key='oteryn:item.tibia.i43295'))
), source_move_current AS (
 SELECT p.* FROM source_move_paths p JOIN game_item_instances i USING(item_instance_id)
 JOIN game_spell_item_lines m ON m.transaction_id=p.mint_transaction_id AND m.item_instance_id=p.item_instance_id AND m.operation_kind=1
 JOIN game_spell_item_receipts root ON root.transaction_id=p.mint_transaction_id
 WHERE p.depth>0 AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=(p.definition_family,p.definition_production_key,p.definition_revision)
  AND i.minted_transaction_id=p.mint_transaction_id
  AND ((i.lifecycle=1 AND i.quantity=1 AND i.state_revision=p.state_revision AND i.last_transaction_id=p.last_transaction_id
   AND EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id
    AND (g.world_id,g.scope_kind,g.channel_id,g.instance_id,g.runtime_scope_ownership_generation,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context,g.stack_ordinal)
     IS NOT DISTINCT FROM (m.world_id,m.runtime_scope_kind,m.channel_id,m.instance_id,root.ownership_generation,m.spatial_position,m.map_revision,m.content_revision,m.placement_context,p.stack_ordinal)))
  OR (i.lifecycle=2 AND i.quantity=0 AND i.state_revision=p.state_revision+1 AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)
   AND EXISTS(SELECT 1 FROM game_spell_item_lines n JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_spell_item_audit_outbox a USING(transaction_id)
    WHERE n.item_instance_id=i.item_instance_id AND n.transaction_id=i.last_transaction_id AND n.operation_kind=2
     AND n.ordinal=1 AND r.operation_count=1 AND r.cause_kind=3 AND n.source_stack_ordinal=p.stack_ordinal AND n.state_revision_before=p.state_revision AND n.quantity_before=1 AND n.quantity_after=0
     AND (n.definition_family,n.definition_production_key,n.definition_revision)=(p.definition_family,p.definition_production_key,p.definition_revision)
     AND (n.world_id,n.runtime_scope_kind,n.channel_id,n.instance_id,n.spatial_position,n.map_revision,n.content_revision,n.placement_context,n.content_generation_digest)
      IS NOT DISTINCT FROM (m.world_id,m.runtime_scope_kind,m.channel_id,m.instance_id,m.spatial_position,m.map_revision,m.content_revision,m.placement_context,m.content_generation_digest)
     AND (r.world_id,r.runtime_scope_kind,r.channel_id,r.instance_id,r.ownership_generation,r.catalog_digest,r.creature_definition_key,r.creature_definition_revision,r.creature_map_digest,r.creature_frame_digest,r.creature_source_body_digest)
      IS NOT DISTINCT FROM (root.world_id,root.runtime_scope_kind,root.channel_id,root.instance_id,root.ownership_generation,root.catalog_digest,root.creature_definition_key,root.creature_definition_revision,root.creature_map_digest,root.creature_frame_digest,root.creature_source_body_digest)
     AND r.binding=sha256(r.intent) AND a.envelope=r.intent AND a.envelope_sha256=r.binding AND a.event_id=r.event_id
     AND a.created_xact_id=r.created_xact_id AND n.created_xact_id=r.created_xact_id AND a.occurred_at_unix_ms=r.occurred_at_unix_ms
     AND convert_from(r.intent,'UTF8')::jsonb->>'schema'='OTERYN_CREATURE_EVENT_GROUND_CAUSE/v1'
     AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='rotten-blood.footprint.remove')))
)
