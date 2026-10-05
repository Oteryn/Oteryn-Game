-- Candidate native source-map Item initialization. No player/death/spell cause.
-- Current assignment/node/recovery + active map proof are checked by the owner.
CREATE TABLE game_native_map_item_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID UNIQUE NOT NULL CHECK(game_character_is_uuid_v7(event_id)),
 item_instance_id UUID UNIQUE NOT NULL CHECK(game_character_is_uuid_v7(item_instance_id)),
 world_id UUID NOT NULL,channel_id UUID NOT NULL,
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
 holder_node_id UUID NOT NULL,holder_registration_revision NUMERIC(20,0) NOT NULL CHECK(holder_registration_revision>0),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 map_digest BYTEA NOT NULL CHECK(octet_length(map_digest)=32),
 frame_digest BYTEA NOT NULL CHECK(octet_length(frame_digest)=32),
 placement_key TEXT NOT NULL CHECK(octet_length(placement_key) BETWEEN 1 AND 512),
 definition_family TEXT NOT NULL CHECK(definition_family='Item'),
 definition_key TEXT NOT NULL CHECK(octet_length(definition_key) BETWEEN 1 AND 512),
 definition_revision TEXT NOT NULL CHECK(octet_length(definition_revision) BETWEEN 1 AND 512),
 quantity BIGINT NOT NULL CHECK(quantity BETWEEN 1 AND 4294967295),
 spatial_position BYTEA NOT NULL CHECK(octet_length(spatial_position)=10),
 map_revision TEXT NOT NULL CHECK(map_revision='sha256:'||encode(map_digest,'hex')),
 content_revision TEXT NOT NULL CHECK(content_revision='sha256:'||encode(content_digest,'hex')),
 placement_context BYTEA NOT NULL CHECK(placement_context=frame_digest),
 source_binding BYTEA NOT NULL CHECK(octet_length(source_binding) BETWEEN 1 AND 8192),
 source_attributes JSONB NOT NULL CHECK(jsonb_typeof(source_attributes)='object' AND octet_length(source_attributes::text)<=8192),
 blocks_movement BOOLEAN NOT NULL,blocks_projectile BOOLEAN NOT NULL,immovable_block_solid BOOLEAN NOT NULL,
 owner_kind TEXT CHECK(owner_kind IN('door','bed','teleport','container','field')),
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32),
 source_intent BYTEA NOT NULL CHECK(binding=sha256(source_intent) AND octet_length(source_intent) BETWEEN 1 AND 32768),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(world_id,channel_id,content_digest,placement_key)
);
CREATE TABLE game_native_map_item_audit (
 event_id UUID PRIMARY KEY REFERENCES game_native_map_item_receipts(event_id),
 transaction_id UUID UNIQUE NOT NULL REFERENCES game_native_map_item_receipts(transaction_id),
 item_instance_id UUID UNIQUE NOT NULL REFERENCES game_item_instances(item_instance_id),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 32768),
 envelope_digest BYTEA NOT NULL CHECK(envelope_digest=sha256(envelope)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_native_map_item_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE j JSONB;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 IF TG_TABLE_NAME='game_native_map_item_receipts' THEN
  j:=convert_from(NEW.source_intent,'UTF8')::jsonb;
  IF j IS DISTINCT FROM jsonb_build_object('schema','OTERYN_NATIVE_MAP_ITEM_INIT/v1',
    'world',encode(uuid_send(NEW.world_id),'hex'),'channel',encode(uuid_send(NEW.channel_id),'hex'),
    'scope_generation',NEW.scope_generation::text,'content_digest',encode(NEW.content_digest,'hex'),
    'map_digest',encode(NEW.map_digest,'hex'),'frame_digest',encode(NEW.frame_digest,'hex'),
    'placement_key',NEW.placement_key,'definition_family',NEW.definition_family,'definition_key',NEW.definition_key,
    'definition_revision',NEW.definition_revision,'quantity',NEW.quantity,
    'spatial_position',encode(NEW.spatial_position,'hex'),'source_binding',encode(NEW.source_binding,'hex'),
    'source_attributes',NEW.source_attributes,'blocks_movement',NEW.blocks_movement,
    'blocks_projectile',NEW.blocks_projectile,'immovable_block_solid',NEW.immovable_block_solid,'owner_kind',NEW.owner_kind) THEN
   RAISE EXCEPTION 'map source intent does not bind exact receipt' USING ERRCODE='23514';
  END IF;
  IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments s JOIN game_durability_admission_runtime_guards g USING(scope_key)
    WHERE s.world_id=NEW.world_id AND s.channel_id=NEW.channel_id AND s.state=1
     AND s.ownership_generation=NEW.scope_generation AND s.holder_node_id=NEW.holder_node_id
     AND s.holder_registration_revision=NEW.holder_registration_revision AND g.ready AND g.ownership_generation=s.ownership_generation) THEN
   RAISE EXCEPTION 'map source has no current ready assignment' USING ERRCODE='23514';
  END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_native_map_item_stamp BEFORE INSERT ON game_native_map_item_receipts FOR EACH ROW EXECUTE FUNCTION game_native_map_item_stamp();
CREATE TRIGGER game_native_map_audit_stamp BEFORE INSERT ON game_native_map_item_audit FOR EACH ROW EXECUTE FUNCTION game_native_map_item_stamp();
CREATE FUNCTION game_native_map_item_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'source map initialization receipts/audit are immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_native_map_receipt_immutable BEFORE UPDATE OR DELETE ON game_native_map_item_receipts FOR EACH ROW EXECUTE FUNCTION game_native_map_item_immutable();
CREATE TRIGGER game_native_map_audit_immutable BEFORE UPDATE OR DELETE ON game_native_map_item_audit FOR EACH ROW EXECUTE FUNCTION game_native_map_item_immutable();
CREATE FUNCTION game_native_map_item_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_native_map_item_audit a USING(item_instance_id)
  WHERE i.item_instance_id=NEW.item_instance_id AND i.world_id=NEW.world_id AND i.definition_family=NEW.definition_family
   AND i.definition_production_key=NEW.definition_key AND i.definition_revision_ref=NEW.definition_revision
   AND i.quantity=NEW.quantity AND i.lifecycle=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
   AND i.minted_transaction_id=NEW.transaction_id AND g.world_id=NEW.world_id AND g.channel_id=NEW.channel_id
   AND g.runtime_scope_ownership_generation=NEW.scope_generation AND g.spatial_position=NEW.spatial_position
   AND g.map_revision=NEW.map_revision AND g.content_revision=NEW.content_revision AND g.native_room_placement_context=NEW.placement_context
   AND a.transaction_id=NEW.transaction_id AND a.event_id=NEW.event_id AND a.envelope=NEW.source_intent
   AND a.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'source map initialization needs exact same-transaction Item/Ground/audit' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_native_map_item_proven AFTER INSERT ON game_native_map_item_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_native_map_item_proven();
-- Preserve exact0033 existing owner branches; add only genuine map source.
CREATE OR REPLACE FUNCTION game_item_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN

    IF EXISTS(SELECT 1 FROM game_native_map_item_receipts r JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_native_map_item_audit a USING(item_instance_id)
      WHERE r.item_instance_id=NEW.item_instance_id AND r.transaction_id=NEW.minted_transaction_id
      AND r.world_id=NEW.world_id AND r.definition_family=NEW.definition_family AND r.definition_key=NEW.definition_production_key
      AND r.definition_revision=NEW.definition_revision_ref AND r.quantity=NEW.quantity
      AND g.world_id=r.world_id AND g.channel_id=r.channel_id AND g.runtime_scope_ownership_generation=r.scope_generation
      AND g.spatial_position=r.spatial_position AND g.map_revision=r.map_revision AND g.content_revision=r.content_revision
      AND g.native_room_placement_context=r.placement_context AND a.event_id=r.event_id AND a.transaction_id=r.transaction_id
      AND a.envelope=r.source_intent AND a.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.item_instance_id=NEW.item_instance_id AND l.operation_kind=1
         AND l.transaction_id=NEW.minted_transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS (
        SELECT 1
          FROM game_item_mint_receipts r
          JOIN game_item_ground_locations g ON g.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.death_world_id = NEW.world_id
           AND r.destination_parent_item_instance_id IS NULL
           AND g.world_id = NEW.world_id
           AND g.channel_id = r.death_channel_id
           AND g.runtime_scope_ownership_generation = r.death_scope_ownership_generation
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.occurred_at = r.occurred_at
           AND a.envelope_sha256 = r.envelope_sha256
           AND EXISTS (
               SELECT 1 FROM game_item_mint_reservations v
                WHERE (v.death_world_id, v.death_channel_id, v.death_scope_ownership_generation,
                       v.death_actor_local_id, v.death_actor_local_generation,
                       v.loot_table_family, v.loot_table_production_key,
                       v.loot_table_revision_ref, v.loot_purpose_key, v.draw_ordinal,
                       v.intent_binding, v.transaction_id, v.event_id, v.item_instance_id,
                       v.occurred_at)
                    = (r.death_world_id, r.death_channel_id, r.death_scope_ownership_generation,
                       r.death_actor_local_id, r.death_actor_local_generation,
                       r.loot_table_family, r.loot_table_production_key,
                       r.loot_table_revision_ref, r.loot_purpose_key, r.draw_ordinal,
                       r.intent_binding, r.transaction_id, r.event_id, r.item_instance_id,
                       r.occurred_at)
                  AND sha256(v.envelope) = r.envelope_sha256)) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_reward_claim_mint_receipts r
          JOIN game_item_container_entries e ON e.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.created_xact_id = pg_current_xact_id()
           AND e.placed_transaction_id = r.transaction_id
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_item_mint_receipts r
          JOIN game_item_corpse_container_entries e ON e.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.death_world_id = NEW.world_id
           AND r.destination_parent_item_instance_id = e.parent_item_instance_id
           AND r.destination_ordinal = e.placement_ordinal
           AND e.world_id = NEW.world_id
           AND e.placed_transaction_id = r.transaction_id
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.occurred_at = r.occurred_at
           AND a.envelope_sha256 = r.envelope_sha256
           AND EXISTS (
               SELECT 1 FROM game_item_mint_reservations v
                WHERE (v.death_world_id, v.death_channel_id, v.death_scope_ownership_generation,
                       v.death_actor_local_id, v.death_actor_local_generation,
                       v.loot_table_family, v.loot_table_production_key,
                       v.loot_table_revision_ref, v.loot_purpose_key, v.draw_ordinal,
                       v.intent_binding, v.transaction_id, v.event_id, v.item_instance_id,
                       v.occurred_at)
                    = (r.death_world_id, r.death_channel_id, r.death_scope_ownership_generation,
                       r.death_actor_local_id, r.death_actor_local_generation,
                       r.loot_table_family, r.loot_table_production_key,
                       r.loot_table_revision_ref, r.loot_purpose_key, r.draw_ordinal,
                       r.intent_binding, r.transaction_id, r.event_id, r.item_instance_id,
                       r.occurred_at)
                  AND sha256(v.envelope) = r.envelope_sha256)
           -- The parent must be a LIVE corpse of the SAME death (§39.4): its
           -- own CORPSE_MATERIALIZATION receipt, same full death tuple, still
           -- live on Ground (never yet DECAY_RETIREd).
           AND EXISTS (
               SELECT 1 FROM game_item_mint_receipts cr
                 JOIN game_item_ground_locations cg ON cg.item_instance_id = cr.item_instance_id
                 JOIN game_item_instances ci ON ci.item_instance_id = cr.item_instance_id
                WHERE cr.item_instance_id = e.parent_item_instance_id
                  AND cr.loot_purpose_key = 'CORPSE_MATERIALIZATION'
                  AND cr.death_world_id = r.death_world_id
                  AND cr.death_channel_id = r.death_channel_id
                  AND cr.death_scope_ownership_generation = r.death_scope_ownership_generation
                  AND cr.death_actor_local_id = r.death_actor_local_id
                  AND cr.death_actor_local_generation = r.death_actor_local_generation
                  AND ci.lifecycle = 1)) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_item_fee_burns f
          JOIN game_item_container_entries e ON e.item_instance_id = NEW.item_instance_id
         WHERE f.transaction_id = NEW.minted_transaction_id
           AND f.created_xact_id = pg_current_xact_id()
           AND NEW.item_instance_id IN (f.change_platinum_item_instance_id,
                                        f.change_gold_item_instance_id)
           AND e.placed_transaction_id = f.transaction_id) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'item MINT must commit its location, cause receipt and audit event together as its reserved transaction'
        USING ERRCODE = '23514';
END;
$$;

-- Preserve exact0033 existing owner branches; add only genuine map source.
CREATE OR REPLACE FUNCTION game_item_ground_insertion_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN

    IF EXISTS(SELECT 1 FROM game_native_map_item_receipts r JOIN game_item_instances i USING(item_instance_id) JOIN game_native_map_item_audit a USING(item_instance_id)
      WHERE r.item_instance_id=NEW.item_instance_id AND r.transaction_id=i.minted_transaction_id
      AND i.world_id=NEW.world_id AND i.quantity=r.quantity AND i.lifecycle=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
      AND i.definition_family=r.definition_family AND i.definition_production_key=r.definition_key AND i.definition_revision_ref=r.definition_revision
      AND NEW.world_id=r.world_id AND NEW.channel_id=r.channel_id AND NEW.runtime_scope_ownership_generation=r.scope_generation
      AND NEW.spatial_position=r.spatial_position AND NEW.map_revision=r.map_revision AND NEW.content_revision=r.content_revision
      AND NEW.native_room_placement_context=r.placement_context AND a.event_id=r.event_id AND a.transaction_id=r.transaction_id
      AND a.envelope=r.source_intent AND a.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
       WHERE l.item_instance_id=NEW.item_instance_id AND l.operation_kind=1
         AND i.lifecycle=1 AND i.last_transaction_id IS NULL AND i.minted_transaction_id=l.transaction_id
         AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()
         AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)
         AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=i.item_instance_id)
         AND NOT EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.item_instance_id=i.item_instance_id)) THEN RETURN NULL; END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_instances i
         WHERE i.item_instance_id = NEW.item_instance_id
           AND i.world_id = NEW.world_id
           AND i.lifecycle = 1
           AND i.last_transaction_id IS NULL)
       OR NOT EXISTS (
        SELECT 1 FROM game_item_mint_receipts r
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.created_xact_id = pg_current_xact_id())
       OR EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts rr
         WHERE rr.item_instance_id = NEW.item_instance_id)
       OR EXISTS (
        SELECT 1 FROM game_item_container_entries e
         WHERE e.item_instance_id = NEW.item_instance_id)
       OR EXISTS (
        SELECT 1 FROM game_item_container_slots s
         WHERE s.item_instance_id = NEW.item_instance_id)
       OR EXISTS (
        SELECT 1 FROM game_item_corpse_container_entries ce
         WHERE ce.item_instance_id = NEW.item_instance_id) THEN
        RAISE EXCEPTION 'Ground placement must be the item MINT of the current transaction'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
REVOKE ALL ON game_native_map_item_receipts,game_native_map_item_audit FROM PUBLIC;
GRANT SELECT,INSERT ON game_native_map_item_receipts,game_native_map_item_audit TO oteryn_game_runtime;
DO $$ DECLARE name TEXT; BEGIN
 FOREACH name IN ARRAY ARRAY['game_native_map_item_stamp','game_native_map_item_immutable','game_native_map_item_proven','game_item_mint_consistency_guard','game_item_ground_insertion_guard'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',name);
 END LOOP;
END $$;

-- A scope handoff adopts existing source-created custody; it never changes the
-- immutable initialization cause and never recreates an absent Ground row.
CREATE TABLE game_native_map_scope_adoptions (
 source_transaction_id UUID NOT NULL REFERENCES game_native_map_item_receipts(transaction_id),
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
 holder_node_id UUID NOT NULL,holder_registration_revision NUMERIC(20,0) NOT NULL CHECK(holder_registration_revision>0),
 ground_before JSONB CHECK(ground_before IS NULL OR jsonb_typeof(ground_before)='object'),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(source_transaction_id,ownership_generation)
);
CREATE FUNCTION game_native_map_adoption_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r game_native_map_item_receipts%ROWTYPE;g JSONB;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 -- Source receipts are immutable and runtime deliberately has no UPDATE right.
 SELECT * INTO STRICT r FROM game_native_map_item_receipts WHERE transaction_id=NEW.source_transaction_id;
 PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(r.world_id),'hex') || encode(uuid_send(r.channel_id),'hex'),33));
 -- All custody writers serialize on the Item; Ground needs no locking privilege.
 PERFORM 1 FROM game_item_instances WHERE item_instance_id=r.item_instance_id FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'map custody Item absent' USING ERRCODE='23514'; END IF;
 IF NEW.ownership_generation<=r.scope_generation OR NOT EXISTS(
 SELECT 1 FROM game_runtime_scope_assignments s JOIN game_durability_admission_runtime_guards a USING(scope_key)
 WHERE s.world_id=r.world_id AND s.channel_id=r.channel_id AND s.state=1 AND s.ownership_generation=NEW.ownership_generation
  AND s.holder_node_id=NEW.holder_node_id AND s.holder_registration_revision=NEW.holder_registration_revision
  AND a.ready AND a.ownership_generation=s.ownership_generation) THEN
 RAISE EXCEPTION 'map custody adoption requires current higher scope owner' USING ERRCODE='23514'; END IF;
 SELECT to_jsonb(x) INTO g FROM game_item_ground_locations x WHERE x.item_instance_id=r.item_instance_id;
 -- Absence or custody in a different frame is recorded as observation only;
 -- it can never create a new Ground or modify that other owner's row.
 IF NEW.ground_before IS NOT NULL AND (NEW.ground_before IS DISTINCT FROM g OR
  g->>'world_id'<>r.world_id::text OR g->>'channel_id'<>r.channel_id::text OR
  g->>'map_revision'<>r.map_revision OR g->>'content_revision'<>r.content_revision OR
  g->>'native_room_placement_context'<>('\x'||encode(r.placement_context,'hex')) OR
  (g->>'runtime_scope_ownership_generation')::numeric>=NEW.ownership_generation) THEN
 RAISE EXCEPTION 'map custody predecessor substitution' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_native_map_adoption_stamp BEFORE INSERT ON game_native_map_scope_adoptions FOR EACH ROW EXECUTE FUNCTION game_native_map_adoption_stamp();
CREATE TRIGGER game_native_map_adoption_immutable BEFORE UPDATE OR DELETE ON game_native_map_scope_adoptions FOR EACH ROW EXECUTE FUNCTION game_native_map_item_immutable();
-- The runtime can INSERT the exact handoff receipt, but cannot UPDATE Ground.
-- This trigger alone performs its bounded same-transaction generation successor.
CREATE FUNCTION game_native_map_adoption_apply() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE changed BIGINT;
BEGIN
 IF NEW.ground_before IS NOT NULL THEN
  UPDATE game_item_ground_locations g
   SET runtime_scope_ownership_generation=NEW.ownership_generation
   FROM game_native_map_item_receipts r
   WHERE r.transaction_id=NEW.source_transaction_id AND g.item_instance_id=r.item_instance_id
    AND to_jsonb(g)=NEW.ground_before AND NEW.created_xact_id=pg_current_xact_id();
  GET DIAGNOSTICS changed=ROW_COUNT;
  IF changed<>1 THEN RAISE EXCEPTION 'map Ground changed during custody handoff' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_native_map_adoption_apply AFTER INSERT ON game_native_map_scope_adoptions
 FOR EACH ROW EXECUTE FUNCTION game_native_map_adoption_apply();
CREATE FUNCTION game_native_map_ground_rebind() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (to_jsonb(NEW)-'runtime_scope_ownership_generation') IS DISTINCT FROM (to_jsonb(OLD)-'runtime_scope_ownership_generation') OR
  NOT EXISTS(SELECT 1 FROM game_native_map_scope_adoptions a JOIN game_native_map_item_receipts r ON r.transaction_id=a.source_transaction_id
   WHERE r.item_instance_id=OLD.item_instance_id AND a.ownership_generation=NEW.runtime_scope_ownership_generation
    AND a.ground_before=to_jsonb(OLD) AND a.created_xact_id=pg_current_xact_id()) THEN
 RAISE EXCEPTION 'Ground UPDATE requires exact same-transaction map custody handoff' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
DROP TRIGGER game_item_ground_location_immutable ON game_item_ground_locations;
CREATE TRIGGER game_item_ground_location_immutable BEFORE UPDATE ON game_item_ground_locations FOR EACH ROW EXECUTE FUNCTION game_native_map_ground_rebind();
CREATE FUNCTION game_native_map_adoption_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.ground_before IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_native_map_item_receipts r USING(item_instance_id)
  WHERE r.transaction_id=NEW.source_transaction_id AND g.runtime_scope_ownership_generation=NEW.ownership_generation
  AND (to_jsonb(g)-'runtime_scope_ownership_generation')=(NEW.ground_before-'runtime_scope_ownership_generation')
  AND NEW.created_xact_id=pg_current_xact_id()) THEN
 RAISE EXCEPTION 'map scope handoff needs exact existing Ground successor' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_native_map_adoption_proven AFTER INSERT ON game_native_map_scope_adoptions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_native_map_adoption_proven();
REVOKE ALL ON game_native_map_scope_adoptions FROM PUBLIC;
GRANT SELECT,INSERT ON game_native_map_scope_adoptions TO oteryn_game_runtime;
DO $$ DECLARE n TEXT;BEGIN FOREACH n IN ARRAY ARRAY['game_native_map_adoption_stamp','game_native_map_adoption_apply','game_native_map_ground_rebind','game_native_map_adoption_proven'] LOOP
 EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp',n,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;END $$;
-- Preserve0033 shared Ground-owner serialization for every legacy writer.
-- The sole new UPDATE changes generation under an exact live handoff receipt.
CREATE OR REPLACE FUNCTION game_item_ground_owner_lock() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(NEW.world_id),'hex') || encode(uuid_send(NEW.channel_id),'hex'),33));
  RETURN NEW;
 ELSIF TG_OP='DELETE' THEN
  PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(OLD.world_id),'hex') || encode(uuid_send(OLD.channel_id),'hex'),33));
  RETURN OLD;
 ELSIF TG_OP='UPDATE' THEN
  PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(OLD.world_id),'hex') || encode(uuid_send(OLD.channel_id),'hex'),33));
  IF (to_jsonb(NEW)-'runtime_scope_ownership_generation')=(to_jsonb(OLD)-'runtime_scope_ownership_generation')
   AND NEW.runtime_scope_ownership_generation>OLD.runtime_scope_ownership_generation
   AND EXISTS(SELECT 1 FROM game_native_map_scope_adoptions a JOIN game_native_map_item_receipts r ON r.transaction_id=a.source_transaction_id
    WHERE r.item_instance_id=OLD.item_instance_id AND a.ownership_generation=NEW.runtime_scope_ownership_generation
     AND a.ground_before=to_jsonb(OLD) AND a.created_xact_id=pg_current_xact_id()) THEN RETURN NEW; END IF;
 END IF;
 RAISE EXCEPTION 'Ground address updates remain closed' USING ERRCODE='23514';
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_item_ground_owner_lock() SET search_path = %I, pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_item_ground_owner_lock() FROM PUBLIC;
