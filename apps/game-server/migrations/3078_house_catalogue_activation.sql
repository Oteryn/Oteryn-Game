CREATE TABLE game_house_catalogue_membership (
 world_id uuid NOT NULL,
 house_key text NOT NULL,
 content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
 PRIMARY KEY(world_id,house_key),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
REVOKE ALL ON game_house_catalogue_membership FROM PUBLIC;
GRANT SELECT ON game_house_catalogue_membership TO oteryn_game_runtime;

CREATE FUNCTION game_house_catalogue_activate(w uuid,ch uuid,g numeric,seq numeric,digest bytea,
 entries jsonb,n uuid,nr numeric,secret bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE entry record;
BEGIN
 IF game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM true THEN RETURN false; END IF;
 PERFORM 1 FROM game_runtime_scope_assignments
 WHERE world_id=w AND channel_id=ch AND scope_kind=1 AND state=1 AND ownership_generation=g
 AND holder_node_id=n AND holder_registration_revision=nr FOR SHARE;
 IF NOT FOUND THEN RETURN false; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended(
    'oteryn:content-activation:' || w::text || ':' || ch::text, 0));
 IF digest IS NULL OR octet_length(digest)<>32 OR NOT EXISTS(
  SELECT 1 FROM game_content_activations a WHERE a.world_id=w AND a.channel_id=ch
   AND a.activation_sequence=seq AND a.server_artifact_digest=digest
   AND seq=(SELECT max(b.activation_sequence) FROM game_content_activations b WHERE b.world_id=w AND b.channel_id=ch)
 ) THEN RETURN false; END IF;
 IF jsonb_typeof(entries) IS DISTINCT FROM 'array' THEN RETURN false; END IF;
 IF jsonb_array_length(entries)>1024 OR octet_length(entries::text)>262144 THEN RETURN false; END IF;

 FOR entry IN SELECT * FROM jsonb_to_recordset(entries) AS x(house_key text,kind text,rent_gold bigint)
  ORDER BY house_key COLLATE "C" LOOP
  INSERT INTO game_house_properties(world_id,house_key,kind,rent_gold,content_digest)
  VALUES(w,entry.house_key,entry.kind,entry.rent_gold,digest) ON CONFLICT DO NOTHING;
  IF FOUND THEN
   INSERT INTO game_house_ownership(world_id,house_key,ownership_revision,acquisition_receipt_ref)
   VALUES(w,entry.house_key,1,game_character_uuid_v7());
  ELSE
   UPDATE game_house_properties SET rent_gold=entry.rent_gold,content_digest=digest,revision=revision+1
   WHERE world_id=w AND house_key=entry.house_key AND kind=entry.kind
    AND state=1 AND content_fence_operation IS NULL
    AND (rent_gold,content_digest) IS DISTINCT FROM (entry.rent_gold,digest);
  END IF;
  INSERT INTO game_house_catalogue_membership(world_id,house_key,content_digest)
  SELECT w,entry.house_key,digest FROM game_house_properties
  WHERE world_id=w AND house_key=entry.house_key AND kind=entry.kind AND content_digest=digest
  ON CONFLICT(world_id,house_key) DO UPDATE SET content_digest=EXCLUDED.content_digest
  WHERE game_house_catalogue_membership.content_digest<>EXCLUDED.content_digest;
 END LOOP;
 RETURN true;
END $$;

CREATE FUNCTION game_house_first_bid_apply(p jsonb,n uuid,nr numeric,secret bytea,envelope bytea,digest bytea)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE prop game_house_properties%ROWTYPE;
BEGIN
 IF (p->>'kind')::smallint IS DISTINCT FROM 1 OR game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM true THEN
  RAISE EXCEPTION 'HOUSE_INVALID_INPUT' USING ERRCODE='23514';
 END IF;
 SELECT * INTO prop FROM game_house_properties
 WHERE world_id=(p->>'world')::uuid AND house_key=p->>'house' FOR UPDATE;
 IF NOT FOUND OR prop.state<>1 OR prop.kind NOT IN ('private_house','shop')
  OR prop.content_digest IS DISTINCT FROM digest OR prop.content_fence_operation IS NOT NULL
  OR NOT EXISTS(SELECT 1 FROM game_house_catalogue_membership m
    WHERE m.world_id=prop.world_id AND m.house_key=prop.house_key AND m.content_digest=digest)
  OR (p->>'first_rent')::bigint IS DISTINCT FROM prop.rent_gold THEN
  RAISE EXCEPTION 'HOUSE_NOT_CURRENT' USING ERRCODE='23514';
 END IF;
 INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at)
 VALUES((p->>'auction')::uuid,prop.world_id,prop.house_key,prop.rent_gold,
  to_timestamp((p->>'ends_at_us')::numeric/1000000));
 UPDATE game_house_properties SET state=2,revision=revision+1
 WHERE world_id=prop.world_id AND house_key=prop.house_key;
 -- The existing value owner validates the live session and writes the bid,
 -- escrow, ledger and receipt. A refusal rolls back the auction too.
 PERFORM game_house_value_apply(p,n,nr,secret,envelope);
END $$;

DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY[
  'game_house_catalogue_activate(uuid,uuid,numeric,numeric,bytea,jsonb,uuid,numeric,bytea)'::regprocedure,
  'game_house_first_bid_apply(jsonb,uuid,numeric,bytea,bytea,bytea)'::regprocedure
 ] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',f);
 END LOOP;
END $$;
