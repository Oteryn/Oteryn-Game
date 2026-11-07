-- P18: a receipt must prove a legal Forge transition, not merely repeat the
-- values supplied by its writer. The existing Item/dust/fee/stash owners remain
-- unchanged. The policy projection follows the immutable migration admission
-- precedent of 0070; it is generated from the already admitted native Items.
-- A new/changed definition or policy is refused until its own migration admits
-- the matching projection. Runtime cannot supply or modify these policy facts.
-- Existing completed operations are not reinterpreted by this successor guard.

-- BEGIN GENERATED FORGE POLICY
-- Producer: tools/content-migration/forge_sql_admission.py --write
-- Canonical native profiles + crates/gameplay/items/src/forge/prices.rs.
-- Projection SHA256: f5aeafbc23af318cac95868656cbe69b92864e868b2e5218400027c939a3bb63
CREATE FUNCTION game_forge_admitted_profile(p_key TEXT,p_revision TEXT)
RETURNS TABLE(classification INT,max_tier INT,equipment_slot TEXT)
LANGUAGE sql IMMUTABLE AS $$
 SELECT c,m,s FROM (VALUES
  ('oteryn:item.tibia.i12683','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i16160','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i17812','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i17824','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i17859','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i21171','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i21174','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i3264','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3269','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3271','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3275','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3279','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3291','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i3292','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3301','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3306','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i3307','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3308','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3311','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3316','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3317','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i3318','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i3320','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i3322','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i3333','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i3342','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i3346','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i34082','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i34083','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i34084','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i34085','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i34086','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i34087','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43864','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43865','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43866','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43867','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43868','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43869','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43870','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43871','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43872','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43873','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43874','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i43875','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i50157','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i50159','definition-r1',4,10,'hand'),
  ('oteryn:item.tibia.i50271','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i50272','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i50273','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i7380','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7381','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i7383','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7384','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7386','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7387','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7388','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7389','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7392','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7402','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7406','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7413','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7415','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7425','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i7427','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7428','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7436','definition-r1',2,2,'hand'),
  ('oteryn:item.tibia.i7449','definition-r1',1,1,'hand'),
  ('oteryn:item.tibia.i7454','definition-r1',2,2,'hand')
 ) p(k,r,c,m,s) WHERE p.k=p_key AND p.r=p_revision
$$;

-- Crystal 201189f item_tiers.lua:1-90; result-tier price for transfer.
CREATE FUNCTION game_forge_admitted_price(p_class INT,p_tier INT)
RETURNS TABLE(gold BIGINT,cores INT,convergence_fusion BIGINT,convergence_transfer BIGINT)
LANGUAGE sql IMMUTABLE AS $$
 SELECT g::bigint,n,f::bigint,t::bigint FROM (VALUES
  (1,1,25000,1,0,0),
  (2,1,750000,1,0,0),
  (2,2,5000000,1,0,0),
  (3,1,4000000,1,0,0),
  (3,2,10000000,2,0,0),
  (3,3,20000000,3,0,0),
  (4,1,8000000,1,55000000,65000000),
  (4,2,20000000,2,110000000,165000000),
  (4,3,40000000,5,170000000,375000000),
  (4,4,65000000,10,300000000,800000000),
  (4,5,100000000,15,875000000,2000000000),
  (4,6,250000000,25,2350000000,5250000000),
  (4,7,750000000,35,6950000000,14500000000),
  (4,8,2500000000,50,21250000000,42500000000),
  (4,9,8000000000,60,50000000000,100000000000),
  (4,10,15000000000,85,125000000000,300000000000)
 ) p(c,r,g,n,f,t) WHERE p.c=p_class AND p.r=p_tier
$$;

CREATE FUNCTION game_forge_admitted_material(p_key TEXT,p_revision TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT EXISTS(SELECT 1 FROM (VALUES
  ('oteryn:item.tibia.i37109','definition-r1'),
  ('oteryn:item.tibia.i37110','definition-r1')
 ) p(k,r) WHERE p.k=p_key AND p.r=p_revision)
$$;
-- END GENERATED FORGE POLICY

-- serde_json's Value objects are lexicographically ordered and compact. Native
-- snapshots contain integer values, strings, booleans and null; no floating point.
CREATE FUNCTION game_forge_canonical_json(p_value JSONB) RETURNS TEXT
LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE result TEXT;
BEGIN
 CASE jsonb_typeof(p_value)
 WHEN 'object' THEN
  SELECT '{'||coalesce(string_agg(to_jsonb(key)::text||':'||game_forge_canonical_json(value),',' ORDER BY key COLLATE "C"),'')||'}'
   INTO result FROM jsonb_each(p_value);
 WHEN 'array' THEN
  SELECT '['||coalesce(string_agg(game_forge_canonical_json(value),',' ORDER BY ordinal),'')||']'
   INTO result FROM jsonb_array_elements(p_value) WITH ORDINALITY AS a(value,ordinal);
 ELSE result:=p_value::text;
 END CASE;
 RETURN result;
END $$;

CREATE FUNCTION game_forge_json_bytes(p_value BYTEA) RETURNS JSONB
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT coalesce(jsonb_agg(get_byte(p_value,n) ORDER BY n),'[]'::jsonb)
 FROM generate_series(0,octet_length(p_value)-1) n
$$;

CREATE FUNCTION game_forge_wire_handle(p_intent BYTEA,p_offset INT) RETURNS NUMERIC
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT sum(get_byte(p_intent,p_offset+n)::numeric*power(256::numeric,n))
 FROM generate_series(0,7) n
$$;

-- Independently reproduce simulation-determinism/src/lib.rs:337-365. The Rust
-- reservation supplies the draws, but cannot choose them: SQL verifies all three
-- against the immutable secret, Character and occurrence. No seed is returned.
CREATE FUNCTION game_forge_expected_rolls(p_character UUID,p_occurrence UUID)
RETURNS SMALLINT[] LANGUAGE plpgsql STABLE AS $$
DECLARE seed BYTEA; prefix BYTEA; digest BYTEA; value NUMERIC; draws SMALLINT[]:='{}'; n INT; j INT;
BEGIN
 SELECT sha256(secret||uuid_send(p_character)) INTO STRICT seed FROM game_forge_decision_root WHERE singleton;
 prefix:=convert_to('oteryn.sim.decision.v1','UTF8')||decode('00','hex')||int4send(1)
  ||seed||uuid_send(p_occurrence)||int2send(12::smallint)||convert_to('forge_fusion','UTF8');
 FOR n IN 0..2 LOOP
  digest:=sha256(prefix||int8send(n::bigint)); value:=0;
  FOR j IN 0..7 LOOP value:=value*256+get_byte(digest,j); END LOOP;
  draws:=array_append(draws,mod(value,CASE WHEN n=2 THEN 10001 ELSE 100 END)::smallint);
 END LOOP;
 RETURN draws;
END $$;

CREATE FUNCTION game_forge_reservation_policy_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE operation INT:=get_byte(NEW.intent,0); flags INT:=get_byte(NEW.intent,17);
BEGIN
 IF NEW.simulation_revision<>1 OR ARRAY[NEW.success_roll,NEW.protection_roll,NEW.bonus_roll]
      IS DISTINCT FROM game_forge_expected_rolls(NEW.character_id,NEW.occurrence_id)
    OR get_byte(NEW.intent,50)<>0 THEN
  RAISE EXCEPTION 'Forge reservation does not bind the native frozen draws' USING ERRCODE='23514'; END IF;
 IF operation BETWEEN 1 AND 4 THEN
  IF flags NOT IN(3,7,11,15) OR (operation<>1 AND flags<>3)
     OR game_forge_wire_handle(NEW.intent,18)=0 OR game_forge_wire_handle(NEW.intent,34)=0
     OR substring(NEW.intent FROM 27 FOR 8)<>decode(repeat('00',8),'hex')
     OR substring(NEW.intent FROM 43 FOR 8)<>decode(repeat('00',8),'hex') THEN
   RAISE EXCEPTION 'Forge pair intent is not canonical session handles' USING ERRCODE='23514'; END IF;
 ELSIF flags<>0 OR substring(NEW.intent FROM 19 FOR 32)<>decode(repeat('00',32),'hex') THEN
  RAISE EXCEPTION 'non-pair Forge intent contains item operands' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_forge_reservation_policy_guard BEFORE INSERT ON game_forge_reservations
 FOR EACH ROW EXECUTE FUNCTION game_forge_reservation_policy_guard();

-- The nominal core prerequisite survives the core-refund bonus. Count existing
-- native carried/stashed units; never introduce a cached Forge material balance.
CREATE FUNCTION game_forge_available_cores(p_character UUID,p_world UUID) RETURNS BIGINT
LANGUAGE sql STABLE AS $$
 WITH RECURSIVE carried(item,depth) AS (
  SELECT e.item_instance_id,1 FROM game_item_container_entries e JOIN game_item_container_slots b
   ON b.character_id=e.character_id AND b.world_id=e.world_id AND b.item_instance_id=e.parent_item_instance_id
   WHERE e.character_id=p_character AND e.world_id=p_world
  UNION ALL
  SELECT e.item_instance_id,c.depth+1 FROM carried c JOIN game_item_nested_entries e ON e.parent_item_instance_id=c.item
   JOIN game_item_instances parent ON parent.item_instance_id=c.item AND parent.lifecycle=1
   WHERE e.world_id=p_world AND c.depth<8
 ), eligible AS (
  SELECT i.quantity FROM carried c JOIN game_item_instances i ON i.item_instance_id=c.item
   WHERE i.world_id=p_world AND i.lifecycle=1 AND i.definition_family='Item'
    AND i.definition_production_key='oteryn:item.tibia.i37110'
    AND game_forge_admitted_material(i.definition_production_key,i.definition_revision_ref)
    AND NOT EXISTS(SELECT 1 FROM game_item_keys t WHERE t.item_instance_id=i.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_imbuements t WHERE t.item_instance_id=i.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_tiers t WHERE t.item_instance_id=i.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules t WHERE t.item_instance_id=i.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules t WHERE t.item_instance_id=i.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries t WHERE t.parent_item_instance_id=i.item_instance_id)
  LIMIT 501
 ) SELECT (SELECT coalesce(sum(quantity),0) FROM eligible)
   +coalesce((SELECT quantity FROM game_character_stash_balances WHERE character_id=p_character AND world_id=p_world
     AND definition_key='oteryn:item.tibia.i37110' AND game_forge_admitted_material(definition_key,definition_revision)),0)
$$;

CREATE FUNCTION game_forge_transition_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_forge_operations%ROWTYPE; r game_forge_reservations%ROWTYPE;
 a JSONB; b JSONB; pa RECORD; pb RECORD; price RECORD; target INT; source INT; result_tier INT;
 source_tier INT:=0; source_quantity INT:=0; nominal_dust INT; nominal_cores INT:=0;
 dust INT; cores INT; gold BIGINT:=0; succeeded BOOLEAN:=true; bonus INT:=0;
 boost BOOLEAN; protect BOOLEAN; expected_reply JSONB; expected_proof JSONB; active BYTEA;
 first_handle NUMERIC; second_handle NUMERIC; retired BIGINT; tier_changes BIGINT;
BEGIN
 SELECT * INTO STRICT p FROM game_forge_operations WHERE character_id=NEW.character_id AND occurrence_id=NEW.occurrence_id;
 IF NOT p.applied OR p.operation=7 THEN RETURN NULL; END IF;
 SELECT * INTO STRICT r FROM game_forge_reservations WHERE character_id=p.character_id AND occurrence_id=p.occurrence_id;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:'||r.world_id::text||':'||r.channel_id::text,0));
 SELECT server_artifact_digest INTO active FROM game_content_activations
  WHERE world_id=r.world_id AND channel_id=r.channel_id ORDER BY activation_sequence DESC LIMIT 1;
 IF active IS DISTINCT FROM r.content_digest OR r.ruleset_revision<>'crystal-201189f-forge-v1'
    OR r.simulation_revision<>1 OR r.exhausted OR p.created_xact_id<>pg_current_xact_id()
    OR ARRAY[r.success_roll,r.protection_roll,r.bonus_roll] IS DISTINCT FROM game_forge_expected_rolls(r.character_id,r.occurrence_id) THEN
  RAISE EXCEPTION 'Forge applied proof has stale revisions or substituted draws' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(p.inputs) WITH ORDINALITY AS e(l,n)
     WHERE (p.operation>=5 OR n>2) AND NOT game_forge_admitted_material(l->'before'->>'definition_production_key',l->'before'->>'definition_revision_ref'))
    OR EXISTS(SELECT 1 FROM jsonb_array_elements(p.outputs) l WHERE NOT game_forge_admitted_material(l->>'key',l->>'revision'))
    OR (p.stash IS NOT NULL AND NOT game_forge_admitted_material(p.stash->>'key',p.stash->>'revision')) THEN
  RAISE EXCEPTION 'Forge material has no admitted native definition revision' USING ERRCODE='23514'; END IF;
 IF p.operation BETWEEN 1 AND 4 THEN
  a:=p.inputs->0; b:=p.inputs->1;
  SELECT * INTO pa FROM game_forge_admitted_profile(a->'before'->>'definition_production_key',a->'before'->>'definition_revision_ref');
  IF NOT FOUND THEN RAISE EXCEPTION 'Forge receiver has no admitted SQL policy projection' USING ERRCODE='23514'; END IF;
  SELECT * INTO pb FROM game_forge_admitted_profile(b->'before'->>'definition_production_key',b->'before'->>'definition_revision_ref');
  IF NOT FOUND THEN RAISE EXCEPTION 'Forge source has no admitted SQL policy projection' USING ERRCODE='23514'; END IF;
  target:=(a->>'tier_before')::int; source:=(b->>'tier_before')::int;
  IF pa.classification<>pb.classification OR target>pa.max_tier OR source>pb.max_tier
     OR (a->>'item')::uuid IS DISTINCT FROM r.first_item_instance_id
     OR (b->>'item')::uuid IS DISTINCT FROM r.second_item_instance_id THEN
   RAISE EXCEPTION 'Forge pair violates admitted classification, cap or identity' USING ERRCODE='23514'; END IF;
  boost:=(get_byte(r.intent,17)&4)<>0; protect:=(get_byte(r.intent,17)&8)<>0;
  IF p.operation IN(1,2) THEN
   IF target<>source OR target>=pa.max_tier
      OR (p.operation=1 AND (a->'before'->>'definition_production_key',a->'before'->>'definition_revision_ref')
           IS DISTINCT FROM (b->'before'->>'definition_production_key',b->'before'->>'definition_revision_ref'))
      OR (p.operation=2 AND (pa.classification<>4 OR pa.equipment_slot<>pb.equipment_slot)) THEN
    RAISE EXCEPTION 'Forge fusion pair violates its admitted shape' USING ERRCODE='23514'; END IF;
   result_tier:=target+1;
   IF p.operation=1 THEN
    nominal_cores:=boost::int+protect::int; succeeded:=r.success_roll<50+15*boost::int;
    IF succeeded THEN
     bonus:=CASE WHEN r.bonus_roll<7400 THEN 0 WHEN r.bonus_roll<9000 THEN 1 WHEN r.bonus_roll<9500 THEN 2
       WHEN r.bonus_roll<9525 THEN 3 WHEN r.bonus_roll<9550 THEN 4 WHEN r.bonus_roll<9950 THEN 5
       WHEN r.bonus_roll<9975 THEN 6 ELSE 7 END;
     IF bonus IN(4,5,6) THEN source_quantity:=1;
      source_tier:=CASE bonus WHEN 4 THEN greatest(target-1,0) WHEN 5 THEN target ELSE target+1 END; END IF;
    ELSIF protect AND r.protection_roll>=50 THEN bonus:=8; source_quantity:=1; source_tier:=target;
    ELSIF target>0 THEN source_quantity:=1; source_tier:=target-1;
    END IF;
   END IF;
  ELSE
   IF target<>0 OR source<(CASE WHEN p.operation=3 THEN 2 ELSE 1 END)
      OR (p.operation=3 AND pa.equipment_slot<>pb.equipment_slot)
      OR (p.operation=4 AND pa.classification<>4) THEN
    RAISE EXCEPTION 'Forge transfer pair violates its admitted shape' USING ERRCODE='23514'; END IF;
   result_tier:=source-CASE WHEN p.operation=3 THEN 1 ELSE 0 END;
   IF result_tier>pa.max_tier THEN RAISE EXCEPTION 'Forge transfer exceeds receiver cap' USING ERRCODE='23514'; END IF;
  END IF;
  SELECT * INTO price FROM game_forge_admitted_price(pa.classification,result_tier);
  IF NOT FOUND THEN RAISE EXCEPTION 'Forge result tier has no admitted price' USING ERRCODE='23514'; END IF;
  nominal_dust:=CASE p.operation WHEN 2 THEN 130 WHEN 4 THEN 160 ELSE 100 END;
  IF p.operation IN(3,4) THEN nominal_cores:=price.cores; END IF;
  gold:=CASE p.operation WHEN 2 THEN price.convergence_fusion WHEN 4 THEN price.convergence_transfer ELSE price.gold END;
  IF bonus=3 THEN gold:=0; END IF;
  dust:=CASE WHEN bonus=1 THEN 0 ELSE nominal_dust END;
  cores:=CASE WHEN bonus=2 THEN 0 ELSE nominal_cores END;
  IF NOT succeeded THEN result_tier:=target;
  ELSIF bonus=7 AND target+2<=pa.classification AND target+2<=pa.max_tier THEN result_tier:=target+2;
  END IF;
  IF (a->>'quantity_after')::int<>1 OR (a->>'tier_after')::int<>result_tier
     OR (b->>'quantity_after')::int<>source_quantity OR (b->>'tier_after')::int<>source_tier
     OR p.dust_before<nominal_dust
     OR game_forge_available_cores(p.character_id,r.world_id)+cores<nominal_cores THEN
   RAISE EXCEPTION 'Forge transition or nominal resources do not match its frozen outcome' USING ERRCODE='23514'; END IF;
  first_handle:=game_forge_wire_handle(r.intent,18); second_handle:=game_forge_wire_handle(r.intent,34);
 ELSE
  nominal_dust:=CASE WHEN p.operation=5 THEN 60 ELSE 0 END; dust:=nominal_dust; cores:=0;
 END IF;
 IF p.gold_spent<>gold OR p.dust_spent<>dust
    OR game_forge_resource_delta(p,'oteryn:item.tibia.i37110')<>(CASE WHEN p.operation=6 THEN 1 ELSE -cores END)
    OR game_forge_resource_delta(p,'oteryn:item.tibia.i37109')<>(CASE p.operation WHEN 5 THEN 3 WHEN 6 THEN -50 ELSE 0 END) THEN
  RAISE EXCEPTION 'Forge mandatory costs do not match its frozen outcome' USING ERRCODE='23514'; END IF;
 expected_reply:=jsonb_build_object('operation',CASE p.operation WHEN 1 THEN 'fusion' WHEN 2 THEN 'convergence_fusion'
   WHEN 3 THEN 'transfer' WHEN 4 THEN 'convergence_transfer' WHEN 5 THEN 'dust_to_slivers' ELSE 'slivers_to_cores' END,
  'succeeded',succeeded,'applied',true,'reason',NULL,'bonus',bonus,
  'result',jsonb_build_object('first_item',first_handle,'first_tier',result_tier,'second_item',second_handle,
    'second_tier',CASE WHEN source_quantity=1 THEN source_tier ELSE NULL END,'dust',p.dust_before-dust,
    'dust_limit',p.dust_limit,'gold_spent',gold,'cores_spent',cores,'slivers_spent',CASE WHEN p.operation=6 THEN 50 ELSE 0 END,
    'slivers_created',CASE WHEN p.operation=5 THEN 3 ELSE 0 END,'cores_created',CASE WHEN p.operation=6 THEN 1 ELSE 0 END));
 IF p.reply IS DISTINCT FROM expected_reply THEN RAISE EXCEPTION 'Forge reply differs from its proven native result' USING ERRCODE='23514'; END IF;
 SELECT count(*) FILTER(WHERE (l->>'quantity_after')::bigint=0),
        count(*) FILTER(WHERE (l->>'tier_before')::int<>(l->>'tier_after')::int)
  INTO retired,tier_changes FROM jsonb_array_elements(p.inputs) l;
 expected_proof:=jsonb_build_object('occurrence',game_forge_json_bytes(uuid_send(p.occurrence_id)),
  'character',game_forge_json_bytes(uuid_send(p.character_id)),'world',game_forge_json_bytes(uuid_send(r.world_id)),
  'operation',p.operation,'content',game_forge_json_bytes(r.content_digest),'ruleset',r.ruleset_revision,
  'simulation_revision',r.simulation_revision,'rolls',jsonb_build_array(r.success_roll,r.protection_roll,r.bonus_roll),'reply',p.reply,
  'inputs_hash',game_forge_json_bytes(sha256(convert_to(game_forge_canonical_json(p.inputs),'UTF8'))),
  'outputs_hash',game_forge_json_bytes(sha256(convert_to(game_forge_canonical_json(p.outputs),'UTF8'))),
  'stash_hash',game_forge_json_bytes(sha256(convert_to(game_forge_canonical_json(coalesce(p.stash,'null'::jsonb)),'UTF8'))),
  'inputs',jsonb_array_length(p.inputs),'outputs',jsonb_array_length(p.outputs),'retired',retired,'tier_changes',tier_changes,
  'dust_spent',p.dust_spent,'gold_spent',p.gold_spent,'stash_spent',p.stash IS NOT NULL);
 IF convert_from(p.audit_proof,'UTF8')::jsonb IS DISTINCT FROM expected_proof THEN
  RAISE EXCEPTION 'Forge audit proof does not bind its exact native effects and reservation' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_forge_transition_proven AFTER INSERT ON game_forge_operations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_forge_transition_proven();

DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_forge_admitted_profile(text,text)','game_forge_admitted_price(integer,integer)',
  'game_forge_admitted_material(text,text)',
  'game_forge_canonical_json(jsonb)','game_forge_json_bytes(bytea)','game_forge_wire_handle(bytea,integer)',
  'game_forge_expected_rolls(uuid,uuid)','game_forge_reservation_policy_guard()',
  'game_forge_available_cores(uuid,uuid)','game_forge_transition_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',n);
 END LOOP;
END $$;
