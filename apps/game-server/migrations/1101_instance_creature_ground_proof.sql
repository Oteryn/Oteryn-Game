-- W02: Crystal src/creatures/creature.cpp:711-721,777-782 creates the
-- physical corpse at the killed creature's tile, also inside an Instance.
-- Extend only the existing Instance placement gate. Retain every other owner
-- arm and the mandatory deferred game_item_mint_consistent proof unchanged.
DO $migration$
DECLARE definition TEXT; anchor TEXT := ' IF NEW.scope_kind=2 THEN'; arm TEXT;
BEGIN
 definition:=pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
 IF strpos(definition,anchor)=0 THEN
  RAISE EXCEPTION 'Instance Ground placement gate changed';
 END IF;
 arm:=$arm$
  -- Reservations predate the commit pass. The receipt and its exact frozen
  -- audit bytes must belong to this physical transaction, never a prior mint.
  IF EXISTS(
   SELECT 1 FROM game_item_mint_receipts r
    JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
    JOIN game_item_audit_outbox a ON a.event_id=r.event_id
    JOIN game_item_mint_reservations v ON
     (v.death_world_id,v.death_scope_id,v.death_scope_ownership_generation,
      v.death_actor_local_id,v.death_actor_local_generation,
      v.loot_table_family,v.loot_table_production_key,v.loot_table_revision_ref,
      v.loot_purpose_key,v.draw_ordinal,v.intent_binding,v.transaction_id,
      v.event_id,v.item_instance_id,v.occurred_at)
     =(r.death_world_id,r.death_scope_id,r.death_scope_ownership_generation,
      r.death_actor_local_id,r.death_actor_local_generation,
      r.loot_table_family,r.loot_table_production_key,r.loot_table_revision_ref,
      r.loot_purpose_key,r.draw_ordinal,r.intent_binding,r.transaction_id,
      r.event_id,r.item_instance_id,r.occurred_at)
   WHERE r.item_instance_id=NEW.item_instance_id
    AND r.created_xact_id=pg_current_xact_id()
    AND r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.draw_ordinal=0
    AND r.destination_parent_item_instance_id IS NULL AND r.destination_ordinal IS NULL
    AND r.death_world_id=NEW.world_id AND r.death_channel_id IS NULL
    AND NEW.channel_id IS NULL AND r.death_instance_id=NEW.instance_id
    AND r.death_scope_ownership_generation=NEW.runtime_scope_ownership_generation
    AND i.world_id=NEW.world_id AND i.minted_transaction_id=r.transaction_id
    AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
    AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
       =(r.loot_table_family,r.loot_table_production_key,r.loot_table_revision_ref)
    AND a.created_xact_id=pg_current_xact_id()
    AND a.transaction_id=r.transaction_id AND a.item_instance_id=r.item_instance_id
    AND a.occurred_at=r.occurred_at AND a.envelope_sha256=r.envelope_sha256
    AND a.envelope=v.envelope AND sha256(v.envelope)=r.envelope_sha256
    AND NOT EXISTS(SELECT 1 FROM game_reward_claim_mint_receipts rr WHERE rr.item_instance_id=NEW.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=NEW.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.item_instance_id=NEW.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=NEW.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=NEW.item_instance_id)
  ) THEN RETURN NULL; END IF;
$arm$;
 EXECUTE replace(definition,anchor,anchor||E'\n'||arm);
END $migration$;
