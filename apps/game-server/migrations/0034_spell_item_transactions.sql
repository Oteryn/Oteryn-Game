-- LOCAL CANDIDATE SPELL-ITEM-1. Does not admit Content or activate spells.
-- Existing loot/death causes remain closed; spell causes have their own
-- command/cost/source receipt on the SAME durable ItemInstance owner.

ALTER TABLE game_item_instances ADD COLUMN state_revision NUMERIC(20,0) NOT NULL DEFAULT 1
    CHECK (state_revision BETWEEN 1 AND 18446744073709551615);
-- Legacy stack order is unknown. No UUID sorting/backfill invents a source
-- ordering. New real Ground insertions acquire append order from this owner.
CREATE SEQUENCE game_item_ground_stack_ordinal_seq AS BIGINT MINVALUE 1;
ALTER TABLE game_item_ground_locations ADD COLUMN stack_ordinal BIGINT UNIQUE
    CHECK (stack_ordinal IS NULL OR stack_ordinal > 0);
CREATE FUNCTION game_item_ground_stack_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.stack_ordinal IS NOT NULL THEN
        RAISE EXCEPTION 'Ground ordinal is assigned by the item owner' USING ERRCODE='23514';
    END IF;
    NEW.stack_ordinal := nextval('game_item_ground_stack_ordinal_seq');
    RETURN NEW;
END $$;

CREATE FUNCTION game_item_ground_owner_lock() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(NEW.world_id),'hex') || encode(uuid_send(NEW.channel_id),'hex'),33));
  RETURN NEW;
 ELSIF TG_OP='DELETE' THEN
  PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(OLD.world_id),'hex') || encode(uuid_send(OLD.channel_id),'hex'),33));
  RETURN OLD;
 END IF;
 RAISE EXCEPTION 'Ground address updates remain closed' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_item_ground_owner_lock BEFORE INSERT OR DELETE OR UPDATE ON game_item_ground_locations
 FOR EACH ROW EXECUTE FUNCTION game_item_ground_owner_lock();
CREATE TRIGGER game_item_ground_stack_stamp BEFORE INSERT ON game_item_ground_locations
    FOR EACH ROW EXECUTE FUNCTION game_item_ground_stack_stamp();

ALTER TABLE game_item_instances DROP CONSTRAINT game_item_instances_minted_transaction_key;
-- Several ordered Food draws may produce the same definition. Deferred MINT
-- proof still binds every distinct output identity to exactly one source line.
ALTER TABLE game_item_instances ADD CONSTRAINT game_item_instances_minted_transaction_key
    UNIQUE (minted_transaction_id,item_instance_id);

CREATE FUNCTION game_spell_cost_binding_valid(bytes BYTEA) RETURNS BOOLEAN
LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE v JSONB; k TEXT; a JSONB; e JSONB; previous TEXT; current TEXT; n NUMERIC;
BEGIN
 v:=convert_from(bytes,'UTF8')::jsonb;
 IF jsonb_typeof(v)<>'object' OR (SELECT count(*) FROM jsonb_object_keys(v))<>11 OR v->'version'<>'1'::jsonb THEN RETURN false; END IF;
 IF EXISTS(SELECT 1 FROM jsonb_object_keys(v) t(key) WHERE key NOT IN('version','before_revision','after_revision','mana_before','mana_after','soul_before','soul_after','cooldowns_before','cooldowns_after','caster_before','caster_after')) THEN RETURN false; END IF;
 FOREACH k IN ARRAY ARRAY['before_revision','after_revision','mana_before','mana_after','soul_before','soul_after'] LOOP
  IF jsonb_typeof(v->k)<>'number' OR (v->>k)!~'^[0-9]+$' THEN RETURN false; END IF;
  n:=(v->>k)::numeric;
  IF n>(CASE WHEN k IN('before_revision','after_revision') THEN 18446744073709551615 ELSE 4294967295 END) THEN RETURN false; END IF;
 END LOOP;
 IF (v->>'after_revision')::numeric<>(v->>'before_revision')::numeric+1
  OR (v->>'mana_after')::numeric>(v->>'mana_before')::numeric
  OR (v->>'soul_after')::numeric>(v->>'soul_before')::numeric THEN RETURN false; END IF;
 FOREACH k IN ARRAY ARRAY['caster_before','caster_after'] LOOP
  a:=v->k;
  IF jsonb_typeof(a)<>'array' OR jsonb_array_length(a)<>32 THEN RETURN false; END IF;
  FOR e IN SELECT value FROM jsonb_array_elements(a) LOOP
   IF jsonb_typeof(e)<>'number' OR e::text!~'^[0-9]+$' OR (e::text)::numeric>255 THEN RETURN false; END IF;
  END LOOP;
  IF NOT EXISTS(SELECT 1 FROM jsonb_array_elements(a) t(value) WHERE value<>'0'::jsonb) THEN RETURN false; END IF;
 END LOOP;
 IF v->'caster_before'=v->'caster_after' THEN RETURN false; END IF;
 FOREACH k IN ARRAY ARRAY['cooldowns_before','cooldowns_after'] LOOP
  a:=v->k; previous:=NULL;
  IF jsonb_typeof(a)<>'array' THEN RETURN false; END IF;
  FOR e IN SELECT value FROM jsonb_array_elements(a) LOOP
   IF jsonb_typeof(e)<>'array' OR jsonb_array_length(e)<>2 OR jsonb_typeof(e->0)<>'string'
     OR jsonb_typeof(e->1)<>'number' OR (e->>1)!~'^[0-9]+$' OR (e->>1)::numeric>18446744073709551615 THEN RETURN false; END IF;
   current:=e->>0;
   IF current!~'^(spell|group):.+$' OR (previous IS NOT NULL AND (previous COLLATE "C") >= (current COLLATE "C")) THEN RETURN false; END IF;
   previous:=current;
  END LOOP;
 END LOOP;
 RETURN true;
EXCEPTION WHEN OTHERS THEN RETURN false;
END $$;

CREATE TABLE game_spell_item_receipts (
    transaction_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID UNIQUE NOT NULL CHECK (game_character_is_uuid_v7(event_id)),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
    caster_lease_generation NUMERIC(20,0),
    caster_placement_digest BYTEA,
    CHECK((caster_lease_generation IS NULL)=(caster_placement_digest IS NULL)),
    CHECK(caster_lease_generation IS NULL OR caster_lease_generation BETWEEN 1 AND 18446744073709551615),
    CHECK(caster_placement_digest IS NULL OR (octet_length(caster_placement_digest)=16 AND caster_placement_digest<>decode(repeat('00',16),'hex'))),
    spell_family TEXT NOT NULL CHECK(spell_family='Spell'),
    spell_production_key TEXT NOT NULL CHECK(octet_length(spell_production_key) BETWEEN 1 AND 512),
    spell_revision TEXT NOT NULL CHECK(octet_length(spell_revision) BETWEEN 1 AND 512),
    catalog_digest BYTEA NOT NULL CHECK(octet_length(catalog_digest)=32),
    binding BYTEA NOT NULL CHECK(octet_length(binding)=32),
    intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 524288),
    cost BYTEA NOT NULL CHECK(octet_length(cost) BETWEEN 1 AND 65536),
    cost_binding BYTEA NOT NULL CHECK(cost_binding=sha256(cost)),
    CHECK(game_spell_cost_binding_valid(cost)),
    operation_count INTEGER NOT NULL CHECK(operation_count BETWEEN 0 AND 500),
    occurred_at_unix_ms BIGINT NOT NULL CHECK(occurred_at_unix_ms>0),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(game_session_id,command_id),
    CHECK(binding=sha256(intent))
);
CREATE TABLE game_spell_item_lines (
    transaction_id UUID NOT NULL REFERENCES game_spell_item_receipts(transaction_id),
    ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 1 AND 500),
    operation_kind SMALLINT NOT NULL CHECK(operation_kind IN(1,2,3,4)),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id) DEFERRABLE INITIALLY DEFERRED,
    definition_family TEXT NOT NULL CHECK(definition_family='Item'),
    definition_production_key TEXT NOT NULL,
    definition_revision TEXT NOT NULL,
    quantity_before BIGINT NOT NULL CHECK(quantity_before BETWEEN 0 AND 4294967295),
    quantity_after BIGINT NOT NULL CHECK(quantity_after BETWEEN 0 AND 4294967295),
    state_revision_before NUMERIC(20,0) NOT NULL CHECK(state_revision_before BETWEEN 0 AND 18446744073709551615),
    world_id UUID NOT NULL, channel_id UUID NOT NULL,
    spatial_position BYTEA NOT NULL CHECK(octet_length(spatial_position) BETWEEN 1 AND 128),
    map_revision TEXT NOT NULL, content_revision TEXT NOT NULL,
    placement_context BYTEA NOT NULL CHECK(octet_length(placement_context) BETWEEN 1 AND 128),
    source_stack_ordinal BIGINT,
    destination_parent_item_instance_id UUID REFERENCES game_item_instances(item_instance_id),
    destination_ordinal NUMERIC(20,0),
    CHECK((destination_parent_item_instance_id IS NULL)=(destination_ordinal IS NULL)),
    CHECK(destination_ordinal IS NULL OR (operation_kind IN(1,4) AND destination_ordinal BETWEEN 1 AND 18446744073709551615)),
    content_generation_digest BYTEA NOT NULL CHECK(octet_length(content_generation_digest)=32),
    blocks_movement BOOLEAN NOT NULL DEFAULT false,
    blocks_projectile BOOLEAN NOT NULL DEFAULT false,
    immovable_block_solid BOOLEAN NOT NULL,
    expires_at_unix_ms BIGINT,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(transaction_id,ordinal), UNIQUE(transaction_id,item_instance_id),
    CHECK((operation_kind=1 AND quantity_before=0 AND quantity_after>0 AND state_revision_before=0 AND source_stack_ordinal IS NULL)
       OR (operation_kind IN(2,3) AND quantity_before>0 AND quantity_after=0 AND state_revision_before>0 AND source_stack_ordinal>0)
       OR (operation_kind=4 AND quantity_before>0 AND quantity_after>quantity_before AND state_revision_before>0 AND source_stack_ordinal IS NULL AND destination_ordinal IS NOT NULL)),
    CHECK(expires_at_unix_ms IS NULL OR operation_kind=1)
);
CREATE UNIQUE INDEX game_spell_item_one_mint ON game_spell_item_lines(item_instance_id) WHERE operation_kind=1;
CREATE TABLE game_spell_item_audit_outbox (
    event_id UUID PRIMARY KEY REFERENCES game_spell_item_receipts(event_id),
    transaction_id UUID UNIQUE NOT NULL REFERENCES game_spell_item_receipts(transaction_id),
    occurred_at_unix_ms BIGINT NOT NULL,
    expires_at_unix_ms BIGINT NOT NULL CHECK(expires_at_unix_ms=occurred_at_unix_ms+7776000000),
    envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 524288),
    envelope_sha256 BYTEA NOT NULL CHECK(envelope_sha256=sha256(envelope)),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE FUNCTION game_spell_item_line_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM game_spell_item_receipts r JOIN game_item_instances i ON i.item_instance_id=NEW.item_instance_id
        WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id()
          AND NEW.created_xact_id=pg_current_xact_id() AND NEW.ordinal<=r.operation_count
          AND (NEW.world_id,NEW.channel_id)=(r.world_id,r.channel_id)
          AND (i.world_id,i.definition_family,i.definition_production_key,i.definition_revision_ref)=(NEW.world_id,NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
          AND ((NEW.operation_kind=1 AND i.lifecycle=1 AND i.quantity=NEW.quantity_after
                AND i.minted_transaction_id=NEW.transaction_id AND i.state_revision=1
                AND ((NEW.destination_parent_item_instance_id IS NULL AND EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id
                    AND (g.world_id,g.channel_id,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)
                      =(NEW.world_id,NEW.channel_id,NEW.spatial_position,NEW.map_revision,NEW.content_revision,NEW.placement_context)
                    AND g.runtime_scope_ownership_generation=r.ownership_generation AND g.stack_ordinal IS NOT NULL))
                  OR (NEW.destination_parent_item_instance_id IS NOT NULL AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=r.character_id AND e.parent_item_instance_id=NEW.destination_parent_item_instance_id AND e.placement_ordinal=NEW.destination_ordinal AND e.placed_transaction_id=NEW.transaction_id)))
                AND NOT (EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id) AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)))
            OR (NEW.operation_kind=4 AND i.lifecycle=1 AND i.last_transaction_id=NEW.transaction_id
                AND i.quantity=NEW.quantity_after AND i.state_revision=NEW.state_revision_before+1
                AND EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=i.item_instance_id AND q.transaction_id=NEW.transaction_id AND q.quantity_before=NEW.quantity_before)
                AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=r.character_id AND e.parent_item_instance_id=NEW.destination_parent_item_instance_id AND e.placement_ordinal=NEW.destination_ordinal)
                AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id))
            OR (NEW.operation_kind IN(2,3) AND i.lifecycle=2 AND i.last_transaction_id=NEW.transaction_id
                AND i.quantity=0 AND i.state_revision=NEW.state_revision_before+1
                AND EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=i.item_instance_id AND q.transaction_id=NEW.transaction_id AND q.quantity_before=NEW.quantity_before)
                AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)
                AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)))) THEN
        RAISE EXCEPTION 'spell item line does not prove its actual item/custody successor' USING ERRCODE='23514';
    END IF;
    IF NEW.operation_kind=3 AND NOT EXISTS(SELECT 1 FROM game_item_mint_receipts m
       WHERE m.item_instance_id=NEW.item_instance_id AND m.loot_purpose_key='CORPSE_MATERIALIZATION') THEN
        RAISE EXCEPTION 'Animate Dead consumes only an actual materialized corpse' USING ERRCODE='23514';
    END IF;
    IF NEW.operation_kind=3 AND NOT EXISTS(SELECT 1 FROM game_spell_companion_acquisition_receipts c WHERE c.transaction_id=NEW.transaction_id AND c.corpse_item_instance_id=NEW.item_instance_id AND c.corpse_state_revision=NEW.state_revision_before AND c.created_xact_id=pg_current_xact_id()) THEN RAISE EXCEPTION 'corpse consumption requires its prepared actual companion acquisition receipt' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE FUNCTION game_spell_item_receipt_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE origin JSONB;
BEGIN
    origin:=convert_from(NEW.intent,'UTF8')::jsonb->'caster_origin';
    IF (NEW.caster_lease_generation IS NULL AND COALESCE(origin,'null'::jsonb)<>'null'::jsonb)
       OR (NEW.caster_lease_generation IS NOT NULL AND
           (origin IS NULL OR jsonb_typeof(origin)<>'object'
            OR (origin->>'lease') IS DISTINCT FROM NEW.caster_lease_generation::text
            OR (origin->'placement') IS DISTINCT FROM to_jsonb(ARRAY(SELECT get_byte(NEW.caster_placement_digest,n) FROM generate_series(0,15)n)))) THEN
        RAISE EXCEPTION 'field creation origin columns must match immutable source intent' USING ERRCODE='23514';
    END IF;
    IF NEW.created_xact_id<>pg_current_xact_id()
       OR (SELECT count(*) FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id AND l.created_xact_id=pg_current_xact_id())<>NEW.operation_count
       OR NOT EXISTS(SELECT 1 FROM game_spell_item_audit_outbox a WHERE a.transaction_id=NEW.transaction_id AND a.event_id=NEW.event_id
           AND a.created_xact_id=pg_current_xact_id() AND a.occurred_at_unix_ms=NEW.occurred_at_unix_ms
           AND a.envelope=NEW.intent AND a.envelope_sha256=NEW.binding) THEN
        RAISE EXCEPTION 'spell item receipt, bounded lines and audit must commit together' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_item_line_proven AFTER INSERT ON game_spell_item_lines
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_item_line_proven();
CREATE CONSTRAINT TRIGGER game_spell_item_receipt_proven AFTER INSERT ON game_spell_item_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_item_receipt_proven();


CREATE TABLE game_spell_item_expiry_receipts (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
 transaction_id UUID UNIQUE NOT NULL CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID UNIQUE NOT NULL CHECK(game_character_is_uuid_v7(event_id)),
 source_transaction_id UUID NOT NULL,source_ordinal INTEGER NOT NULL,
 state_revision_before NUMERIC(20,0) NOT NULL CHECK(state_revision_before BETWEEN 1 AND 18446744073709551614),
 source_stack_ordinal BIGINT NOT NULL CHECK(source_stack_ordinal>0),
 world_id UUID NOT NULL,channel_id UUID NOT NULL,
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
 expires_at_unix_ms BIGINT NOT NULL,occurred_at_unix_ms BIGINT NOT NULL CHECK(occurred_at_unix_ms>=expires_at_unix_ms),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(source_transaction_id,source_ordinal) REFERENCES game_spell_item_lines(transaction_id,ordinal)
);
CREATE FUNCTION game_spell_item_expiry_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
   WHERE (l.transaction_id,l.ordinal)=(NEW.source_transaction_id,NEW.source_ordinal)
     AND l.item_instance_id=NEW.item_instance_id AND l.operation_kind=1 AND l.expires_at_unix_ms=NEW.expires_at_unix_ms
     AND NEW.created_xact_id=pg_current_xact_id() AND i.lifecycle=2 AND i.quantity=0
     AND i.last_transaction_id=NEW.transaction_id AND i.state_revision=NEW.state_revision_before+1
     AND (l.world_id,l.channel_id)=(NEW.world_id,NEW.channel_id)
     AND floor(extract(epoch FROM clock_timestamp())*1000)::bigint>=NEW.expires_at_unix_ms
     AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)) THEN
   RAISE EXCEPTION 'spell expiry must retire its actual due field in the same transaction' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_item_expiry_proven AFTER INSERT ON game_spell_item_expiry_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_item_expiry_proven();

CREATE TABLE game_spell_companion_acquisition_receipts (
 transaction_id UUID PRIMARY KEY REFERENCES game_spell_item_receipts(transaction_id),
 corpse_item_instance_id UUID UNIQUE NOT NULL REFERENCES game_item_instances(item_instance_id),
 corpse_state_revision NUMERIC(20,0) NOT NULL CHECK(corpse_state_revision BETWEEN 1 AND 18446744073709551614),
 world_id UUID NOT NULL,channel_id UUID NOT NULL,
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
 actor_local_id BIGINT NOT NULL CHECK(actor_local_id BETWEEN 1 AND 4294967295),
 actor_local_generation NUMERIC(20,0) NOT NULL CHECK(actor_local_generation BETWEEN 1 AND 18446744073709551615),
 master_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 master_game_session_id UUID NOT NULL,
 master_actor_local_id BIGINT NOT NULL CHECK(master_actor_local_id BETWEEN 1 AND 4294967295),
 master_actor_local_generation NUMERIC(20,0) NOT NULL CHECK(master_actor_local_generation BETWEEN 1 AND 18446744073709551615),
 creature_key TEXT NOT NULL CHECK(octet_length(creature_key) BETWEEN 1 AND 512),
 creature_revision TEXT NOT NULL CHECK(octet_length(creature_revision) BETWEEN 1 AND 512),
 spawn_x INTEGER NOT NULL,spawn_y INTEGER NOT NULL,spawn_z INTEGER NOT NULL CHECK(spawn_z BETWEEN -32768 AND 32767),
 binding_json JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(world_id,channel_id,ownership_generation,actor_local_id,actor_local_generation)
);
CREATE FUNCTION game_spell_companion_acquisition_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id)
   WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()
     AND l.created_xact_id=pg_current_xact_id() AND l.operation_kind=3 AND l.item_instance_id=NEW.corpse_item_instance_id
     AND l.state_revision_before=NEW.corpse_state_revision
     AND (convert_from(r.intent,'UTF8')::jsonb)->'companion'=NEW.binding_json
     AND (r.world_id,r.channel_id,r.ownership_generation,r.character_id,r.game_session_id)
      =(NEW.world_id,NEW.channel_id,NEW.ownership_generation,NEW.master_character_id,NEW.master_game_session_id)) THEN
   RAISE EXCEPTION 'companion acquisition requires its exact corpse consume source in the same transaction' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_companion_acquisition_proven AFTER INSERT ON game_spell_companion_acquisition_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_companion_acquisition_proven();

-- Additional branches of the existing item guards follow. Their old bodies
-- are copied verbatim after the narrow same-transaction spell branch.
-- Preserved base: 0011_item_transfer_backpack.sql game_item_instance_guard
CREATE OR REPLACE FUNCTION game_item_instance_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    NEW.state_revision := OLD.state_revision + 1;
    IF TG_OP = 'UPDATE' AND OLD.lifecycle = 1
       AND NEW.last_transaction_id IS NOT NULL
       AND NEW.last_transaction_id IS DISTINCT FROM OLD.last_transaction_id
       AND (to_jsonb(NEW) - 'quantity' - 'lifecycle' - 'last_transaction_id' - 'state_revision')
         = (to_jsonb(OLD) - 'quantity' - 'lifecycle' - 'last_transaction_id' - 'state_revision') THEN
        INSERT INTO game_item_transfer_quantity_evidence
            (item_instance_id, transaction_id, quantity_before)
        VALUES (OLD.item_instance_id, NEW.last_transaction_id, OLD.quantity);
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 item record changes only through a TRANSFER'
        USING ERRCODE = '23514';
END;
$$;

-- Preserved base: 0031_character_gold_fee_change_mint.sql game_item_mint_consistency_guard
CREATE OR REPLACE FUNCTION game_item_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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

-- Preserved base: 0031_character_gold_fee_change_mint.sql game_item_placement_proven
CREATE OR REPLACE FUNCTION game_item_placement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_TABLE_NAME='game_item_container_entries' AND EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.operation_kind=1 AND l.item_instance_id=NEW.item_instance_id AND l.transaction_id=NEW.placed_transaction_id
        AND l.destination_parent_item_instance_id=NEW.parent_item_instance_id AND l.destination_ordinal=NEW.placement_ordinal
        AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE='23514'; END IF;
       RETURN NULL;
    END IF;
    IF TG_TABLE_NAME = 'game_item_container_slots' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_transfer_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.source_item_instance_id = NEW.item_instance_id
               AND r.shape = 1
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER receipt'
                USING ERRCODE = '23514';
        END IF;
    ELSIF TG_TABLE_NAME = 'game_item_container_entries' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_transfer_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.source_item_instance_id = NEW.item_instance_id
               AND r.shape IN (2,4)
               AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
               AND r.destination_ordinal = NEW.placement_ordinal
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_reward_claim_mint_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.item_instance_id = NEW.item_instance_id
               AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
               AND r.destination_ordinal = NEW.placement_ordinal
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_item_fee_burns f
             WHERE f.transaction_id = NEW.placed_transaction_id
               AND f.character_id = NEW.character_id
               AND f.backpack_item_instance_id = NEW.parent_item_instance_id
               AND NEW.item_instance_id IN (f.change_platinum_item_instance_id,
                                            f.change_gold_item_instance_id)
               AND f.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER, reward-claim MINT or fee change receipt'
                USING ERRCODE = '23514';
        END IF;
        PERFORM 1 FROM game_item_container_slots
         WHERE item_instance_id = NEW.parent_item_instance_id FOR UPDATE;
        IF (SELECT count(*) FROM game_item_container_entries e
             WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 20 THEN
            RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- Preserved base: 0023_character_gold_fee_burn.sql game_item_instance_change_proven
CREATE OR REPLACE FUNCTION game_item_instance_change_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS(SELECT 1 FROM game_spell_item_expiry_receipts e JOIN game_item_instances i USING(item_instance_id)
      WHERE e.item_instance_id=NEW.item_instance_id AND e.transaction_id=NEW.last_transaction_id
        AND i.last_transaction_id=e.transaction_id AND e.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.item_instance_id=NEW.item_instance_id AND l.operation_kind<>1
         AND l.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
         WHERE r.transaction_id = NEW.last_transaction_id
           AND (r.source_item_instance_id = NEW.item_instance_id
                OR r.receiver_item_instance_id = NEW.item_instance_id)
           -- Repair generation 2 (finding 2's root cause, applied here too):
           -- the matched receipt must be this SAME physical transaction's
           -- own receipt, never a historical one whose logical TransactionId
           -- is replayed from a different, already-committed transaction.
           AND r.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
         WHERE d.transaction_id = NEW.last_transaction_id
           AND d.item_instance_id = NEW.item_instance_id
           AND d.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          -- The line's fee record is this same physical transaction's own record, never a
          -- committed one a later line is appended to.
          JOIN game_item_fee_burns f ON f.transaction_id = l.transaction_id
                                    AND f.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'item change must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Preserved base: 0015_corpse_decay_retire.sql game_item_ground_removal_proven
CREATE OR REPLACE FUNCTION game_item_ground_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS(SELECT 1 FROM game_spell_item_expiry_receipts e JOIN game_item_instances i USING(item_instance_id)
      WHERE e.item_instance_id=OLD.item_instance_id AND e.source_stack_ordinal=OLD.stack_ordinal AND (e.world_id,e.channel_id)=(OLD.world_id,OLD.channel_id)
        AND i.last_transaction_id=e.transaction_id AND e.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
       WHERE l.item_instance_id=OLD.item_instance_id AND l.operation_kind IN(2,3)
         AND (l.world_id,l.channel_id,l.spatial_position,l.map_revision,l.content_revision,l.placement_context,l.source_stack_ordinal)
           =(OLD.world_id,OLD.channel_id,OLD.spatial_position,OLD.map_revision,OLD.content_revision,OLD.native_room_placement_context,OLD.stack_ordinal)
         AND i.last_transaction_id=l.transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
          JOIN game_item_instances i ON i.item_instance_id = d.item_instance_id
         WHERE d.item_instance_id = OLD.item_instance_id
           AND d.corpse_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = d.transaction_id
           AND d.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1 FROM game_item_mint_receipts m
         WHERE m.item_instance_id = OLD.item_instance_id
           AND m.loot_purpose_key = 'CORPSE_MATERIALIZATION') THEN
        RAISE EXCEPTION 'a corpse Ground location is never removed by a TRANSFER'
            USING ERRCODE = '23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Ground removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Preserved base: 0013_corpse_container_mint.sql game_item_ground_insertion_guard
CREATE OR REPLACE FUNCTION game_item_ground_insertion_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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

-- Preserved base: 0015_corpse_decay_retire.sql game_item_corpse_entry_removal_proven
CREATE OR REPLACE FUNCTION game_item_corpse_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
          JOIN game_item_instances i ON i.item_instance_id = d.item_instance_id
         WHERE d.item_instance_id = OLD.item_instance_id
           AND d.corpse_item_instance_id = OLD.parent_item_instance_id
           AND d.item_instance_id <> d.corpse_item_instance_id
           AND i.last_transaction_id = d.transaction_id
           AND d.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'corpse container entry removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Preserved base: 0023_character_gold_fee_burn.sql game_item_container_entry_removal_proven
CREATE OR REPLACE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          JOIN game_item_fee_burns f ON f.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND f.character_id = OLD.character_id
           AND f.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND l.created_xact_id = pg_current_xact_id()
           AND f.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'backpack entry removal must commit with its fee BURN line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_spell_item_receipts','game_spell_item_lines','game_spell_item_audit_outbox','game_spell_item_expiry_receipts','game_spell_companion_acquisition_receipts'] LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',n || '_immutable',n);
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',n || '_no_truncate',n);
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_item_instance_guard() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_item_mint_consistency_guard() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_item_placement_proven() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_item_instance_change_proven() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_removal_proven() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_insertion_guard() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_item_corpse_entry_removal_proven() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_item_container_entry_removal_proven() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_spell_item_line_proven() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_spell_item_receipt_proven() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_owner_lock() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_stack_stamp() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_spell_item_expiry_proven() SET search_path = %I, pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_spell_companion_acquisition_proven() SET search_path = %I, pg_temp',current_schema());
END $$;

REVOKE ALL ON game_spell_item_receipts,game_spell_item_lines,game_spell_item_audit_outbox,
    game_item_ground_stack_ordinal_seq FROM PUBLIC;
REVOKE ALL ON FUNCTION game_spell_item_line_proven(),game_spell_item_receipt_proven(),game_item_ground_stack_stamp() FROM PUBLIC;
GRANT SELECT,INSERT ON game_spell_item_receipts,game_spell_item_lines,game_spell_item_audit_outbox TO oteryn_game_runtime;
GRANT USAGE ON SEQUENCE game_item_ground_stack_ordinal_seq TO oteryn_game_runtime;
GRANT SELECT ON game_spell_item_receipts,game_spell_item_lines,game_spell_item_audit_outbox TO oteryn_game_control;

REVOKE ALL ON game_spell_item_expiry_receipts,game_spell_companion_acquisition_receipts FROM PUBLIC;
REVOKE ALL ON FUNCTION game_spell_item_expiry_proven(),game_spell_companion_acquisition_proven() FROM PUBLIC;
GRANT SELECT,INSERT ON game_spell_item_expiry_receipts,game_spell_companion_acquisition_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_spell_item_expiry_receipts,game_spell_companion_acquisition_receipts TO oteryn_game_control;

CREATE INDEX game_item_ground_native_cell_lookup ON game_item_ground_locations(world_id,channel_id,spatial_position);

REVOKE ALL ON FUNCTION game_item_ground_owner_lock() FROM PUBLIC;

DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_spell_cost_binding_valid(BYTEA) SET search_path = %I, pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_spell_cost_binding_valid(BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_spell_cost_binding_valid(BYTEA) TO oteryn_game_runtime,oteryn_game_control;
