-- Allow an exact metadata refresh alongside the existing lifecycle successor.
CREATE OR REPLACE FUNCTION game_house_property_transition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'House properties are never deleted' USING ERRCODE='23514';END IF;
 IF (NEW.kind,NEW.rent_gold,NEW.content_digest) IS DISTINCT FROM (OLD.kind,OLD.rent_gold,OLD.content_digest)
  AND (NEW.kind=OLD.kind OR (OLD.state=1 AND NOT EXISTS(
   SELECT 1 FROM game_house_auctions a WHERE (a.world_id,a.house_key)=(OLD.world_id,OLD.house_key) AND NOT a.settled)))
  AND (to_jsonb(NEW)-ARRAY['kind','rent_gold','content_digest'])
    = (to_jsonb(OLD)-ARRAY['kind','rent_gold','content_digest']) THEN
  RETURN NEW;
 END IF;
 IF ((NEW.world_id,NEW.house_key,NEW.kind)<>(OLD.world_id,OLD.house_key,OLD.kind)
  OR NEW.revision<>OLD.revision+1 OR NEW.acl_revision<OLD.acl_revision
  OR (OLD.state=6 AND NEW.state<>6)
  OR NEW.ownership_generation<>OLD.ownership_generation+
   ((NEW.owner_character_id,NEW.owner_account_id,NEW.owner_guild_id) IS DISTINCT FROM (OLD.owner_character_id,OLD.owner_account_id,OLD.owner_guild_id))::integer
  OR ((OLD.owner_character_id IS NOT NULL OR OLD.owner_guild_id IS NOT NULL) AND NEW.owner_character_id IS NULL AND NEW.owner_guild_id IS NULL
   AND NEW.acquisition_receipt_ref IS DISTINCT FROM OLD.acquisition_receipt_ref)) THEN
  RAISE EXCEPTION 'House property requires an exact successor' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_content_record_activation_with_houses(
 p_world UUID,p_channel UUID,p_sequence NUMERIC,p_previous NUMERIC,
 p_server BYTEA,p_client BYTEA,p_frame BYTEA,p_keys TEXT[],p_kinds TEXT[],p_rents BIGINT[]
) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE replay BOOLEAN;
BEGIN
 -- Issuances are Channel-scoped, but the property rows are World-scoped.
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-catalogue:'||p_world::text,0));
 SELECT EXISTS(SELECT 1 FROM game_content_activations
  WHERE world_id=p_world AND channel_id=p_channel AND activation_sequence=p_sequence) INTO replay;
 PERFORM game_content_record_activation(p_world,p_channel,p_sequence,p_previous,p_server,p_client,p_frame);
 IF replay AND EXISTS(
  SELECT 1 FROM game_content_activations a
  WHERE a.world_id=p_world AND (
   (a.channel_id=p_channel AND a.activation_sequence>p_sequence)
   OR (a.channel_id<>p_channel AND a.server_artifact_digest<>p_server AND NOT EXISTS(
    SELECT 1 FROM game_content_activations newer WHERE newer.world_id=a.world_id
     AND newer.channel_id=a.channel_id AND newer.activation_sequence>a.activation_sequence))
  )) THEN
  RETURN;
 END IF;

 IF cardinality(p_keys)>1024 OR cardinality(p_keys)<>cardinality(p_kinds)
  OR cardinality(p_keys)<>cardinality(p_rents) THEN
  RAISE EXCEPTION 'Invalid authored house catalogue dimensions' USING ERRCODE='23514';
 END IF;
 WITH registered AS (
  INSERT INTO game_house_properties AS property(world_id,house_key,kind,rent_gold,content_digest)
  SELECT p_world,house_key,kind,rent_gold,p_server
  FROM unnest(p_keys,p_kinds,p_rents) AS authored(house_key,kind,rent_gold)
  ORDER BY house_key
  ON CONFLICT (world_id,house_key) DO UPDATE
  SET kind=EXCLUDED.kind,rent_gold=EXCLUDED.rent_gold,content_digest=EXCLUDED.content_digest
  WHERE (property.kind,property.rent_gold,property.content_digest)
     IS DISTINCT FROM (EXCLUDED.kind,EXCLUDED.rent_gold,EXCLUDED.content_digest)
  RETURNING world_id,house_key
 )
 INSERT INTO game_house_ownership(world_id,house_key,ownership_revision,acquisition_receipt_ref)
 SELECT world_id,house_key,1,game_character_uuid_v7() FROM registered
 ON CONFLICT (world_id,house_key) DO NOTHING;
END $$;

DO $$ BEGIN
 EXECUTE format(
  'ALTER FUNCTION game_house_property_transition() SET search_path=pg_catalog,%I,pg_temp',
  current_schema());
 EXECUTE format(
  'ALTER FUNCTION game_content_record_activation_with_houses(uuid,uuid,numeric,numeric,bytea,bytea,bytea,text[],text[],bigint[]) SET search_path=pg_catalog,%I,pg_temp',
  current_schema());
END $$;
REVOKE ALL ON FUNCTION game_content_record_activation_with_houses(UUID,UUID,NUMERIC,NUMERIC,BYTEA,BYTEA,BYTEA,TEXT[],TEXT[],BIGINT[])
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_content_record_activation_with_houses(UUID,UUID,NUMERIC,NUMERIC,BYTEA,BYTEA,BYTEA,TEXT[],TEXT[],BIGINT[])
 TO oteryn_game_control;
