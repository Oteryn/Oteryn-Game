-- A current checked map qualifies ordinary houses for acquisition independently
-- of public projection configuration. Existing custody addresses are immutable.
CREATE TABLE game_house_auction_catalogue (
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('private_house','shop')),
 rent_gold BIGINT NOT NULL CHECK(rent_gold BETWEEN 0 AND 999999999999),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 tiles BYTEA[] NOT NULL CHECK(cardinality(tiles) BETWEEN 1 AND 4096),
 PRIMARY KEY(world_id,house_key),
 CHECK(game_house_scope_key_valid(house_key)),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_epochs
);
CREATE INDEX game_house_auction_catalogue_epoch ON game_house_auction_catalogue(world_id,reset_epoch);
ALTER TABLE game_house_auctions ADD COLUMN requires_opening_bid BOOLEAN NOT NULL DEFAULT FALSE;
CREATE INDEX game_house_auction_world_order ON game_house_auctions(world_id,auction_id);
CREATE INDEX game_house_tiles_by_house ON game_house_tiles(world_id,house_key,spatial_position);
REVOKE ALL ON game_house_auction_catalogue FROM PUBLIC,oteryn_game_runtime;

CREATE FUNCTION game_house_qualify_auction(p JSONB,n UUID,nr NUMERIC,secret BYTEA)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w UUID:=(p->>'world')::uuid; ch UUID:=(p->>'channel')::uuid;
 epoch NUMERIC:=(p->>'epoch')::numeric; digest BYTEA:=decode(p->>'digest','hex');
 h TEXT:=p->>'house'; k TEXT:=p->>'kind'; rent BIGINT:=(p->>'rent')::bigint;
 tile_set BYTEA[];
BEGIN
 IF game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE OR NOT EXISTS(
  SELECT 1 FROM game_runtime_scope_assignments r WHERE r.world_id=w AND r.channel_id=ch
   AND r.scope_kind=1 AND r.state=1 AND r.holder_node_id=n AND r.holder_registration_revision=nr
   AND r.ownership_generation=(p->>'generation')::numeric FOR SHARE
 ) OR NOT EXISTS(
  SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
   WHERE a.world_id=w AND a.reset_epoch=epoch AND e.map_revision=p->>'map'
   AND e.server_digest=digest AND game_world_reset_map_epoch_current(w,a.reset_epoch)
 ) THEN RETURN FALSE; END IF;
 IF k IS NULL OR k NOT IN ('private_house','shop') OR rent IS NULL OR rent NOT BETWEEN 0 AND 999999999999
  OR NOT coalesce(game_house_scope_key_valid(h),false) OR jsonb_typeof(p->'tiles') IS DISTINCT FROM 'array'
  OR jsonb_array_length(p->'tiles') NOT BETWEEN 1 AND 4096 THEN RETURN FALSE; END IF;
 SELECT array_agg(DISTINCT tile_bytes ORDER BY tile_bytes)
  INTO tile_set FROM (
   SELECT int4send((t->>0)::int)||int4send((t->>1)::int)||int2send((t->>2)::smallint) AS tile_bytes
   FROM jsonb_array_elements(p->'tiles') f(t)
  ) positions;
 IF cardinality(tile_set)<>jsonb_array_length(p->'tiles')
  OR EXISTS(SELECT 1 FROM unnest(tile_set) f(t) WHERE t IS NULL OR octet_length(t)<>10) THEN RETURN FALSE; END IF;

 INSERT INTO game_house_auction_catalogue VALUES(w,h,epoch,k,rent,digest,tile_set)
  ON CONFLICT(world_id,house_key) DO UPDATE
   SET (reset_epoch,kind,rent_gold,content_digest,tiles)=
       (EXCLUDED.reset_epoch,EXCLUDED.kind,EXCLUDED.rent_gold,EXCLUDED.content_digest,EXCLUDED.tiles)
   WHERE game_house_auction_catalogue.reset_epoch<EXCLUDED.reset_epoch;
 IF NOT EXISTS(SELECT 1 FROM game_house_auction_catalogue c WHERE (c.world_id,c.house_key)=(w,h)
   AND (c.reset_epoch,c.kind,c.rent_gold,c.content_digest,c.tiles)=(epoch,k,rent,digest,tile_set))
 THEN RETURN FALSE; END IF;
 -- Serialize with public catalogue capture. With no public source it remains a gameplay-only listing.
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 PERFORM game_public_v2_subject_lock('house',w::text||':'||h);
 IF EXISTS(SELECT 1 FROM game_public_house_catalogue c WHERE (c.world_id,c.house_key)=(w,h)) THEN
  PERFORM game_public_v2_house(w,h);
 END IF;
 RETURN TRUE;
END $$;

CREATE VIEW game_house_current_auction_catalogue AS
 SELECT c.world_id,c.house_key,c.kind,c.rent_gold,c.content_digest
 FROM game_house_auction_catalogue c JOIN game_world_reset_active a USING(world_id,reset_epoch);
REVOKE ALL ON game_house_current_auction_catalogue FROM PUBLIC;
GRANT SELECT ON game_house_current_auction_catalogue TO oteryn_game_runtime;

-- Preserve the existing read model, with a vacant listing for qualified unmaterialized properties.
DO $$ DECLARE definition TEXT; BEGIN
 SELECT pg_get_functiondef('game_public_v2_house(uuid,text)'::regprocedure) INTO STRICT definition;
 IF strpos(definition,'IF p.state=1 THEN')=0 THEN RAISE EXCEPTION 'House projection vacancy branch missing'; END IF;
 EXECUTE replace(definition,'IF p.state=1 THEN',
  'IF p.state=1 OR (p.world_id IS NULL AND EXISTS(SELECT 1 FROM game_house_current_auction_catalogue q
   WHERE (q.world_id,q.house_key,q.kind)=(w,h,c.kind))) THEN');
END $$;

CREATE FUNCTION game_house_open_auction(w UUID,h TEXT,digest BYTEA,op UUID,duration_us BIGINT,
 n UUID,nr NUMERIC,secret BYTEA) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE prop game_house_properties; q game_house_auction_catalogue;
BEGIN
 IF duration_us IS NULL OR duration_us<=0 OR NOT coalesce(game_character_is_uuid_v7(op),false)
  OR game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE OR NOT EXISTS(
  SELECT 1 FROM game_runtime_scope_assignments r WHERE r.world_id=w AND r.scope_kind=1
   AND r.state=1 AND r.holder_node_id=n AND r.holder_registration_revision=nr
 ) THEN RETURN FALSE; END IF;
 SELECT c.* INTO q FROM game_world_reset_active a JOIN game_house_auction_catalogue c USING(world_id,reset_epoch)
   WHERE a.world_id=w AND c.house_key=h AND c.content_digest=digest
    AND game_world_reset_map_epoch_current(w,a.reset_epoch);
 IF NOT FOUND THEN RETURN FALSE; END IF;

 PERFORM pg_advisory_xact_lock(hashtextextended(w::text||':'||h,3356));
 SELECT * INTO prop FROM game_house_properties WHERE world_id=w AND house_key=h FOR UPDATE;
 IF FOUND AND (prop.kind<>q.kind OR prop.content_fence_operation IS NOT NULL
              OR prop.state NOT IN (1,2)) THEN RETURN FALSE; END IF;
 -- Another first bid may have committed while we waited for the property.
 IF prop.state=2 THEN RETURN prop.content_digest=digest; END IF;
 IF EXISTS(SELECT 1 FROM game_house_auctions WHERE world_id=w AND house_key=h AND NOT settled)
 THEN RETURN FALSE; END IF;
 IF EXISTS(SELECT 1 FROM game_house_tiles WHERE world_id=w AND house_key=h AND NOT spatial_position=ANY(q.tiles))
  OR EXISTS(SELECT 1 FROM game_house_tiles WHERE world_id=w AND spatial_position=ANY(q.tiles) AND house_key<>h)
 THEN RETURN FALSE; END IF;
 IF EXISTS(SELECT 1 FROM unnest(q.tiles) f(t) WHERE NOT EXISTS(
  SELECT 1 FROM game_house_tiles WHERE world_id=w AND house_key=h AND spatial_position=t
 )) THEN
  -- Exclude concurrent Ground writes between the custody check and tile registration.
  LOCK TABLE game_item_ground_locations IN SHARE ROW EXCLUSIVE MODE NOWAIT;
  IF EXISTS(SELECT 1 FROM game_item_ground_locations WHERE world_id=w AND spatial_position=ANY(q.tiles)) THEN
   RETURN FALSE;
  END IF;
 END IF;
 IF prop.world_id IS NULL THEN
  INSERT INTO game_house_properties(world_id,house_key,kind,rent_gold,content_digest)
   VALUES(w,h,q.kind,q.rent_gold,digest);
  INSERT INTO game_house_ownership(world_id,house_key,ownership_revision,acquisition_receipt_ref)
   VALUES(w,h,1,game_character_uuid_v7());
 ELSIF prop.state=1 AND (prop.content_digest,prop.rent_gold) IS DISTINCT FROM (digest,q.rent_gold) THEN
  UPDATE game_house_properties SET content_digest=digest,rent_gold=q.rent_gold,revision=revision+1
   WHERE world_id=w AND house_key=h;
 END IF;
 INSERT INTO game_house_tiles(world_id,house_key,spatial_position)
  SELECT w,h,t FROM unnest(q.tiles) f(t) ON CONFLICT(world_id,spatial_position) DO NOTHING;
 IF EXISTS(SELECT 1 FROM unnest(q.tiles) f(t) WHERE NOT EXISTS(
  SELECT 1 FROM game_house_tiles WHERE world_id=w AND house_key=h AND spatial_position=t
 )) THEN RAISE EXCEPTION 'HOUSE_CATALOGUE_TILE_CONFLICT' USING ERRCODE='23514'; END IF;
 INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at,requires_opening_bid)
  VALUES(op,w,h,q.rent_gold,clock_timestamp()+(duration_us::double precision * interval '1 microsecond'),TRUE);
 UPDATE game_house_properties SET state=2,revision=revision+1 WHERE world_id=w AND house_key=h;
 RETURN TRUE;
END $$;

CREATE FUNCTION game_house_opening_bid_proven() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_house_operations o
   WHERE o.operation_id=NEW.auction_id AND o.auction_id=NEW.auction_id
    AND o.kind=1 AND o.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'HOUSE_AUCTION_WITHOUT_OPENING_BID' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_opening_bid_proven AFTER INSERT ON game_house_auctions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.requires_opening_bid)
 EXECUTE FUNCTION game_house_opening_bid_proven();

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_opening_bid_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_qualify_auction(JSONB,UUID,NUMERIC,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_open_auction(UUID,TEXT,BYTEA,UUID,BIGINT,UUID,NUMERIC,BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_opening_bid_proven(), game_house_qualify_auction(JSONB,UUID,NUMERIC,BYTEA),
 game_house_open_auction(UUID,TEXT,BYTEA,UUID,BIGINT,UUID,NUMERIC,BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_house_qualify_auction(JSONB,UUID,NUMERIC,BYTEA),
 game_house_open_auction(UUID,TEXT,BYTEA,UUID,BIGINT,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime;
