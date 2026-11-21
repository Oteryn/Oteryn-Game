-- Historical source integrity for one bounded Feroxa receipt. The common
-- verifier separately proves its hash, same-transaction lines/custody and audit.
-- A later batch, reset or ownership handoff must not invalidate this history;
-- no current assignment, live root or active reset is used as authority here.
SELECT EXISTS(
 SELECT 1 FROM game_spell_item_receipts r
 JOIN game_world_raid_firings f ON f.firing_id=r.feroxa_firing_id
 JOIN game_world_feroxa_ground_clears c ON c.firing_id=f.firing_id
  AND c.channel_id=r.channel_id AND c.phase=r.feroxa_phase
 CROSS JOIN LATERAL (SELECT convert_from(r.intent,'UTF8')::jsonb AS body) p
 WHERE r.transaction_id=$1::uuid AND r.cause_kind=5
  AND r.spell_family='WorldEvent' AND r.spell_production_key='oteryn:world-event.feroxa.map-clear'
  AND r.spell_revision='source-9f5a72c-r1'
  AND r.runtime_scope_kind=1 AND r.channel_id IS NOT NULL AND r.instance_id IS NULL
  AND r.swing_ordinal IS NULL AND r.game_session_id IS NULL AND r.command_id IS NULL
  AND r.character_id IS NULL AND r.cost IS NULL AND r.cost_binding IS NULL
  AND r.caster_lease_generation IS NULL AND r.caster_placement_digest IS NULL
  AND num_nonnulls(r.creature_placement,r.creature_actor_generation,r.creature_definition_key,
      r.creature_definition_revision,r.creature_occurrence,r.creature_map_digest,r.creature_frame_digest,
      r.creature_source_body_digest,r.creature_cast_digest)=0
  AND f.world_id=r.world_id AND f.reset_epoch=r.feroxa_reset_epoch AND f.raid_key='raid:edron.feroxa'
  AND r.feroxa_phase IN(1,2) AND r.operation_count BETWEEN 1 AND 500
  AND jsonb_typeof(p.body)='object'
  AND p.body-ARRAY['schema','firing','world','channel','generation','epoch','phase','removals']::text[]='{}'::jsonb
  AND p.body->>'schema' IN('OTERYN_FEROXA_MAP_CLEAR/v1','OTERYN_FEROXA_MAP_CLEAR/v2')
  AND p.body->>'firing'=r.feroxa_firing_id::text AND p.body->>'phase'=r.feroxa_phase::text
  AND p.body->>'world'=r.world_id::text AND p.body->>'channel'=r.channel_id::text
  AND p.body->>'generation'=r.ownership_generation::text AND p.body->>'epoch'=r.feroxa_reset_epoch::text
  AND jsonb_typeof(p.body->'removals')='array'
  AND jsonb_array_length(p.body->'removals')=r.operation_count
  AND NOT EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=r.transaction_id
   AND ((l.operation_kind=2 OR (p.body->>'schema'='OTERYN_FEROXA_MAP_CLEAR/v2' AND l.operation_kind=6))
    AND l.runtime_scope_kind=1 AND l.instance_id IS NULL
    AND (l.world_id,l.channel_id)=(r.world_id,r.channel_id)
    AND l.content_generation_digest=r.catalog_digest
    AND game_feroxa_clear_position(l.spatial_position)
    AND l.destination_parent_item_instance_id IS NULL AND l.quantity_after=0
    AND EXISTS(SELECT 1 FROM jsonb_array_elements(p.body->'removals') AS evidence(body)
     WHERE evidence.body->>'item_instance_id'=l.item_instance_id::text
      AND evidence.body->>'definition_family'=l.definition_family
      AND evidence.body->>'definition_production_key'=l.definition_production_key
      AND evidence.body->>'definition_revision_ref'=l.definition_revision
      AND evidence.body->>'quantity'=l.quantity_before::text
      AND evidence.body->>'state_revision'=l.state_revision_before::text
      AND evidence.body->>'stack_ordinal' IS NOT DISTINCT FROM l.source_stack_ordinal::text
      AND evidence.body->>'source_parent_item_instance_id' IS NOT DISTINCT FROM l.source_parent_item_instance_id::text
      AND evidence.body->>'source_placement_ordinal' IS NOT DISTINCT FROM l.source_placement_ordinal::text
      AND evidence.body->>'source_corpse_root' IS NOT DISTINCT FROM l.source_corpse_root::text
      AND evidence.body->>'source_entry_is_corpse' IS NOT DISTINCT FROM l.source_entry_is_corpse::text
      AND evidence.body->'spatial_position'=to_jsonb(l.spatial_position)
      AND evidence.body->>'map_revision'=l.map_revision AND evidence.body->>'content_revision'=l.content_revision
      AND evidence.body->'native_room_placement_context'=to_jsonb(l.placement_context))) IS NOT TRUE)
)
