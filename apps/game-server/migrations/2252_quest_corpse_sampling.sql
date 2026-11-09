-- M13: read-only sampling keeps the native corpse, race proof, contents and decay.
CREATE TABLE game_quest_corpse_samples (
 character_id UUID NOT NULL,
 cause_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
 transition_key TEXT NOT NULL CHECK(octet_length(transition_key) BETWEEN 1 AND 128),
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
 corpse_item UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 source_item UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 appearance BIGINT NOT NULL CHECK(appearance BETWEEN 1 AND 4294967295),
 maximum_age_ms INTEGER NOT NULL CHECK(maximum_age_ms BETWEEN 1 AND 3600000),
 actor_position JSONB NOT NULL CHECK(jsonb_typeof(actor_position)='array' AND jsonb_array_length(actor_position)=3),
 before_item JSONB NOT NULL CHECK(jsonb_typeof(before_item)='object' AND octet_length(before_item::TEXT)<=16384),
 before_ground JSONB NOT NULL CHECK(jsonb_typeof(before_ground)='object' AND octet_length(before_ground::TEXT)<=16384),
 ingress BYTEA NOT NULL CHECK(octet_length(ingress) BETWEEN 1 AND 4096),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
 FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
 REFERENCES game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key) DEFERRABLE INITIALLY DEFERRED,
 CHECK(corpse_item<>source_item)
);
CREATE INDEX game_quest_corpse_sample_body ON game_quest_corpse_samples(corpse_item);
CREATE INDEX game_quest_corpse_sample_source ON game_quest_corpse_samples(source_item);

CREATE FUNCTION game_quest_corpse_adjacent(cell BYTEA,actor JSONB) RETURNS BOOLEAN
LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE x BIGINT; y BIGINT; z BIGINT;
BEGIN
 IF octet_length(cell)<>10 OR jsonb_typeof(actor) IS DISTINCT FROM 'array'
    OR jsonb_array_length(actor)<>3 THEN RETURN false; END IF;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(actor) part(value)
    WHERE CASE WHEN part.value::TEXT ~ '^-?[0-9]{1,11}$'
      THEN part.value::TEXT::BIGINT NOT BETWEEN -2147483648 AND 2147483647 ELSE true END)
 THEN RETURN false; END IF;
 x:=(get_byte(cell,0)::BIGINT<<24)+(get_byte(cell,1)::BIGINT<<16)+(get_byte(cell,2)::BIGINT<<8)+get_byte(cell,3);
 y:=(get_byte(cell,4)::BIGINT<<24)+(get_byte(cell,5)::BIGINT<<16)+(get_byte(cell,6)::BIGINT<<8)+get_byte(cell,7);
 IF x>=2147483648 THEN x:=x-4294967296; END IF;
 IF y>=2147483648 THEN y:=y-4294967296; END IF;
 z:=get_byte(cell,8)*256+get_byte(cell,9);
 IF z>=32768 THEN z:=z-65536; END IF;
 z:=-z;
 RETURN abs(x-(actor->>0)::BIGINT)<=1 AND abs(y-(actor->>1)::BIGINT)<=1 AND z=(actor->>2)::BIGINT;
END $$;

CREATE FUNCTION game_quest_corpse_sample_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE source_row RECORD; sample_row RECORD; input_row RECORD; ingress_json JSONB; proof JSONB;
BEGIN
 SELECT * INTO source_row FROM game_quest_reward_sources
 WHERE (character_id,cause_id,cause_ordinal,transition_key)=(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key);
 IF NOT FOUND THEN RAISE EXCEPTION 'corpse sample lacks quest source' USING ERRCODE='23514'; END IF;
 IF NOT (source_row.plan ? 'corpse_target') THEN
  IF TG_TABLE_NAME='game_quest_corpse_samples' THEN RAISE EXCEPTION 'corpse sample lacks authored predicate' USING ERRCODE='23514'; END IF;
  RETURN NULL;
 END IF;
 SELECT * INTO sample_row FROM game_quest_corpse_samples
 WHERE (character_id,cause_id,cause_ordinal,transition_key)=(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key);
 IF NOT FOUND OR sample_row.created_xact_id<>pg_current_xact_id() OR source_row.created_xact_id<>pg_current_xact_id()
  OR source_row.plan #>> '{corpse_target,appearance}' IS DISTINCT FROM sample_row.appearance::TEXT
  OR source_row.plan #>> '{corpse_target,maximum_age_ms}' IS DISTINCT FROM sample_row.maximum_age_ms::TEXT
  OR source_row.plan->'capture_creature'='true'::JSONB
 THEN RAISE EXCEPTION 'corpse sample predicate differs from quest source' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_item_instances i
  JOIN game_item_ground_locations g USING(item_instance_id,world_id)
  JOIN game_corpse_source_races p USING(item_instance_id)
  JOIN game_item_mint_receipts r ON r.transaction_id=p.mint_transaction_id AND r.item_instance_id=i.item_instance_id
  WHERE i.item_instance_id=sample_row.corpse_item AND i.world_id=sample_row.world_id
   AND g.channel_id=sample_row.channel_id AND g.runtime_scope_ownership_generation=sample_row.scope_generation
   AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
   AND i.definition_production_key='oteryn:item.tibia.i'||sample_row.appearance::TEXT
   AND i.minted_transaction_id=p.mint_transaction_id AND p.race_key=source_row.plan->>'creature_target'
   AND r.loot_purpose_key='CORPSE_MATERIALIZATION'
   AND floor(extract(epoch FROM transaction_timestamp())*1000)::BIGINT-r.occurred_at BETWEEN 0 AND sample_row.maximum_age_ms-1
   AND to_jsonb(i)=sample_row.before_item AND to_jsonb(g)=sample_row.before_ground
   AND game_quest_corpse_adjacent(g.spatial_position,sample_row.actor_position))
 THEN RAISE EXCEPTION 'corpse sample lacks unchanged fresh native corpse and adjacency' USING ERRCODE='23514'; END IF;
 SELECT * INTO input_row FROM game_quest_callback_consumptions c
 WHERE (c.character_id,c.cause_id,c.cause_ordinal,c.transition_key)=(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
  AND c.item_instance_id=sample_row.source_item AND c.quantity=1 AND c.replacement IS NULL AND c.created_xact_id=pg_current_xact_id();
 IF NOT FOUND OR input_row.before_item->>'world_id' IS DISTINCT FROM sample_row.world_id::TEXT
  OR input_row.before_item->>'definition_production_key' IS DISTINCT FROM source_row.plan->>'carried_item'
 THEN RAISE EXCEPTION 'corpse sample lacks exact carried tool' USING ERRCODE='23514'; END IF;
 proof:=convert_from(input_row.proof,'UTF8')::JSONB;
 SELECT jsonb_agg(get_byte(sample_row.ingress,n) ORDER BY n) INTO ingress_json
 FROM generate_series(0,octet_length(sample_row.ingress)-1) bytes(n);
 IF proof->'reward_callback' IS DISTINCT FROM source_row.plan OR proof #> '{carried_use,intent}' IS DISTINCT FROM ingress_json
 THEN RAISE EXCEPTION 'corpse sample ingress differs from tool receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_corpse_sample_complete AFTER INSERT ON game_quest_corpse_samples
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_corpse_sample_complete();
CREATE CONSTRAINT TRIGGER game_quest_reward_corpse_sample_complete AFTER INSERT ON game_quest_reward_sources
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_corpse_sample_complete();
CREATE TRIGGER game_quest_corpse_sample_immutable BEFORE UPDATE OR DELETE ON game_quest_corpse_samples
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_corpse_sample_no_truncate BEFORE TRUNCATE ON game_quest_corpse_samples EXECUTE FUNCTION game_item_reject_truncate();
DO $migration$
BEGIN
 EXECUTE format('ALTER FUNCTION game_quest_corpse_sample_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON game_quest_corpse_samples FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_corpse_samples TO oteryn_game_runtime;
GRANT SELECT ON game_quest_corpse_samples TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_quest_corpse_sample_complete(),game_quest_corpse_adjacent(BYTEA,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_corpse_adjacent(BYTEA,JSONB) TO oteryn_game_runtime;
