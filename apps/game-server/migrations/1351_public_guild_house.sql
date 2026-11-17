-- PUBPROJ-B: World-scoped public houses and name handoff dependencies.
-- Static metadata follows docs/architecture/OTERYN_HOUSE_CATALOGUE_OWNER_CONTRACT_V1.md:26-43.
-- Auction quote is the already accepted HOUSE-OWN-0 held-bid quote, not bid maxima on the wire.
CREATE TABLE game_public_v2_name_handoffs (
 family TEXT NOT NULL, world_id TEXT NOT NULL, subject TEXT NOT NULL,
 epoch BIGINT NOT NULL, revision BIGINT NOT NULL,
 name_key TEXT NOT NULL, namespace_world TEXT NOT NULL,
 created_ms BIGINT NOT NULL,
 PRIMARY KEY(family,world_id,subject,epoch,revision,name_key,namespace_world),
 FOREIGN KEY(family,world_id,subject) REFERENCES game_public_v2_heads(family,world_id,subject)
);
CREATE INDEX game_public_v2_name_handoff_lookup ON game_public_v2_name_handoffs(namespace_world,name_key,created_ms);
REVOKE ALL ON game_public_v2_name_handoffs FROM PUBLIC;
GRANT SELECT,DELETE ON game_public_v2_name_handoffs TO oteryn_game_runtime;
GRANT SELECT ON game_public_v2_name_handoffs TO oteryn_game_control;

CREATE FUNCTION game_public_v2_names(f TEXT,w TEXT,p JSONB)
RETURNS TABLE(name_key TEXT,namespace_world TEXT) LANGUAGE sql IMMUTABLE AS $$
 SELECT DISTINCT game_character_name_key(n.name),n.ns FROM (
 SELECT g->>'name' AS name,g->>'world_id' AS ns FROM jsonb_array_elements(CASE WHEN f='guild' THEN p ELSE '[]'::jsonb END) g
 UNION ALL SELECT m->>'name','' FROM jsonb_array_elements(CASE WHEN f='guild' THEN p ELSE '[]'::jsonb END) g,
 LATERAL jsonb_array_elements(g->'members') m
 UNION ALL SELECT h->>'owner_name',CASE WHEN h->>'owner_kind'='GUILD' THEN w ELSE '' END
 FROM jsonb_array_elements(CASE WHEN f='house' THEN p ELSE '[]'::jsonb END) h WHERE h ? 'owner_name'
 ) n WHERE n.name IS NOT NULL
$$;
ALTER FUNCTION game_public_v2_touch(TEXT,TEXT,TEXT,JSONB) RENAME TO game_public_v2_touch_before_names;
CREATE FUNCTION game_public_v2_touch(f TEXT,w TEXT,s TEXT,p JSONB) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE prior JSONB; rev BIGINT; stamp BIGINT;
BEGIN
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 SELECT payload INTO prior FROM game_public_v2_heads WHERE family=f AND world_id=w AND subject=s FOR UPDATE;
 -- Non-projected progression changes need no new guild/house revision.
 IF FOUND AND prior=p THEN RETURN; END IF;
 PERFORM game_public_v2_touch_before_names(f,w,s,p);
 SELECT revision,observed_ms INTO STRICT rev,stamp FROM game_public_v2_heads WHERE family=f AND world_id=w AND subject=s;
 INSERT INTO game_public_v2_name_handoffs
 SELECT f,w,s,e.projection_epoch,rev,n.name_key,n.namespace_world,stamp
 FROM (SELECT * FROM game_public_v2_names(f,w,coalesce(prior,'[]'::jsonb))
 EXCEPT SELECT * FROM game_public_v2_names(f,w,p)) n
 CROSS JOIN game_public_projection_epoch e WHERE e.epoch_scope=1;
END $$;
CREATE FUNCTION game_public_v2_ready(f TEXT,w TEXT,s TEXT) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT NOT EXISTS(
 SELECT 1 FROM game_public_v2_heads h CROSS JOIN LATERAL game_public_v2_names(f,w,h.payload) n
 WHERE h.family=f AND h.world_id=w AND h.subject=s AND (
 EXISTS(SELECT 1 FROM game_public_v2_name_handoffs x WHERE x.name_key=n.name_key AND x.namespace_world=n.namespace_world
 AND x.created_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint-60000)
 OR (n.namespace_world='' AND (
 EXISTS(SELECT 1 FROM game_public_character_profile_outbox x WHERE x.vacated_name_key=n.name_key
 AND x.created_at>floor(extract(epoch FROM statement_timestamp())*1000)::bigint-60000)
 OR EXISTS(SELECT 1 FROM game_public_name_handoffs x WHERE x.name_key=n.name_key
 AND x.vacated_at>floor(extract(epoch FROM statement_timestamp())*1000)::bigint-60000))))
 )
$$;

CREATE TABLE game_public_house_source_pending (
 world_id UUID NOT NULL,house_key TEXT NOT NULL,pending_ms BIGINT NOT NULL,
 PRIMARY KEY(world_id,house_key)
);
REVOKE ALL ON game_public_house_source_pending FROM PUBLIC;
GRANT SELECT ON game_public_house_source_pending TO oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_public_house_withhold(w UUID,h TEXT) RETURNS VOID LANGUAGE sql SECURITY DEFINER AS $$
 INSERT INTO game_public_house_source_pending VALUES(w,h,floor(extract(epoch FROM clock_timestamp())*1000)::bigint)
 ON CONFLICT(world_id,house_key) DO NOTHING
$$;

CREATE FUNCTION game_public_v2_house(w UUID,h TEXT) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_public_house_catalogue; p game_house_properties; a game_house_auctions;
 owner TEXT; owner_kind TEXT; status TEXT; extra JSONB:='{}'; out JSONB;
 due BIGINT; disposition SMALLINT; n INT; price BIGINT; top BIGINT; second BIGINT;
BEGIN
 -- Serialize derivation before reading the catalogue/property cut, not after it.
 PERFORM 1 FROM game_public_projection_epoch WHERE epoch_scope=1 FOR SHARE;
 PERFORM game_public_v2_subject_lock('house',w::text||':'||h);
 SELECT * INTO c FROM game_public_house_catalogue WHERE world_id=w AND house_key=h;
 IF NOT FOUND THEN DELETE FROM game_public_house_source_pending WHERE world_id=w AND house_key=h; PERFORM game_public_v2_touch('house',w::text,h,'[]'); RETURN; END IF;
 SELECT * INTO p FROM game_house_properties WHERE world_id=w AND house_key=h;
 IF FOUND AND p.kind<>c.kind THEN PERFORM game_public_house_withhold(w,h); RETURN; END IF;
 IF FOUND AND p.state=6 THEN DELETE FROM game_public_house_source_pending WHERE world_id=w AND house_key=h; PERFORM game_public_v2_touch('house',w::text,h,'[]'); RETURN; END IF;
 status:='UNAVAILABLE';
 IF p.state=1 THEN status:='VACANT';
 ELSIF p.state=2 THEN
  SELECT * INTO a FROM game_house_auctions WHERE world_id=w AND house_key=h AND NOT settled;
  IF NOT FOUND THEN PERFORM game_public_house_withhold(w,h); RETURN; END IF;
  IF FOUND THEN
   -- Same bounded held-bid ordering/pricing as housing/src/house_auction.rs:37-60.
   SELECT count(*),max(maximum) FILTER(WHERE rn=1),max(maximum) FILTER(WHERE rn=2)
   INTO n,top,second FROM (SELECT maximum,row_number() OVER(ORDER BY maximum DESC,reached_order) rn
   FROM game_house_bids WHERE auction_id=a.auction_id AND state=1 ORDER BY maximum DESC,reached_order LIMIT 257) b;
   IF n>256 THEN PERFORM game_public_house_withhold(w,h); RETURN; END IF;
   IF n<=256 THEN
    price:=CASE WHEN n<2 THEN 0 ELSE least(top,second+1) END;
    status:='AUCTION';extra:=jsonb_build_object('current_bid',price::text,'auction_ends_at',floor(extract(epoch FROM a.ends_at))::bigint::text);
   END IF;
  END IF;
 ELSIF p.state IN(3,4) THEN
  IF p.owner_character_id IS NOT NULL AND game_public_v2_character_visible(p.owner_character_id) THEN
   SELECT name INTO owner FROM game_character_roots WHERE character_id=p.owner_character_id AND world_id=w;
   owner_kind:='CHARACTER';
  ELSIF p.owner_guild_id IS NOT NULL THEN
   SELECT name INTO owner FROM game_guilds WHERE guild_id=p.owner_guild_id AND world_id=w AND state IN(1,2,3);
   owner_kind:='GUILD';
  END IF;
  SELECT CASE WHEN cause='MOVE_OUT' THEN state ELSE 2 END,due_at INTO disposition,due FROM game_house_dispositions WHERE world_id=w AND house_key=h AND state<>4 AND released_at IS NULL AND ownership_revision=p.ownership_generation;
  IF disposition IS NULL THEN
   SELECT due_at_unix_ms INTO due FROM game_house_move_out_notices WHERE world_id=w AND house_key=h AND ownership_revision=p.ownership_generation;
  END IF;
  IF owner IS NOT NULL AND p.paid_until IS NOT NULL AND (disposition IS NULL OR disposition=1) THEN
   status:=CASE WHEN due IS NULL AND p.state=3 THEN 'OWNED' WHEN due IS NOT NULL THEN 'MOVE_OUT_PENDING' ELSE 'UNAVAILABLE' END;
   IF status<>'UNAVAILABLE' THEN
    extra:=jsonb_build_object('owner_kind',owner_kind,'owner_name',owner,'paid_until',floor(extract(epoch FROM p.paid_until))::bigint::text);
    IF status='MOVE_OUT_PENDING' THEN extra:=extra||jsonb_build_object('move_out_at',(due/1000)::text); END IF;
   END IF;
  END IF;
 END IF;
 DELETE FROM game_public_house_source_pending WHERE world_id=w AND house_key=h;
 out:=jsonb_build_array(jsonb_build_object('name',c.name,'kind',c.kind,'town',c.town,'size_sqm',c.size_sqm::text,
 'beds',c.beds::text,'rent_gold',c.rent_gold::text,'status',status)||extra);
 PERFORM game_public_v2_touch('house',w::text,h,out);
END $$;
CREATE FUNCTION game_public_v2_house_trigger() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE old_world UUID; old_house TEXT; new_world UUID; new_house TEXT;
BEGIN
 IF TG_OP<>'INSERT' THEN
  IF TG_TABLE_NAME='game_house_bids' THEN SELECT world_id,house_key INTO old_world,old_house FROM game_house_auctions WHERE auction_id=OLD.auction_id;
  ELSE old_world:=OLD.world_id;old_house:=OLD.house_key; END IF;
  IF old_world IS NOT NULL THEN PERFORM game_public_v2_house(old_world,old_house); END IF;
 END IF;
 IF TG_OP<>'DELETE' THEN
  IF TG_TABLE_NAME='game_house_bids' THEN SELECT world_id,house_key INTO new_world,new_house FROM game_house_auctions WHERE auction_id=NEW.auction_id;
  ELSE new_world:=NEW.world_id;new_house:=NEW.house_key; END IF;
  IF new_world IS NOT NULL AND (TG_OP='INSERT' OR (new_world,new_house) IS DISTINCT FROM (old_world,old_house)) THEN
   PERFORM game_public_v2_house(new_world,new_house);
  END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER public_v2_house_property AFTER INSERT OR UPDATE OR DELETE ON game_house_properties FOR EACH ROW EXECUTE FUNCTION game_public_v2_house_trigger();
CREATE TRIGGER public_v2_house_auction AFTER INSERT OR UPDATE OR DELETE ON game_house_auctions FOR EACH ROW EXECUTE FUNCTION game_public_v2_house_trigger();
CREATE TRIGGER public_v2_house_bid AFTER INSERT OR UPDATE OR DELETE ON game_house_bids FOR EACH ROW EXECUTE FUNCTION game_public_v2_house_trigger();
CREATE TRIGGER public_v2_house_notice AFTER INSERT OR UPDATE OR DELETE ON game_house_move_out_notices FOR EACH ROW EXECUTE FUNCTION game_public_v2_house_trigger();
CREATE TRIGGER public_v2_house_disposition AFTER INSERT OR UPDATE OR DELETE ON game_house_dispositions FOR EACH ROW EXECUTE FUNCTION game_public_v2_house_trigger();
CREATE TRIGGER public_v2_house_catalogue AFTER INSERT OR UPDATE OR DELETE ON game_public_house_catalogue FOR EACH ROW EXECUTE FUNCTION game_public_v2_house_trigger();

CREATE INDEX game_public_house_character_owner ON game_house_properties(owner_character_id) WHERE owner_character_id IS NOT NULL;
CREATE TABLE game_public_v2_character_dependencies_state (
 character_id UUID PRIMARY KEY,name TEXT,world_id UUID,eligible BOOLEAN NOT NULL
);
REVOKE ALL ON game_public_v2_character_dependencies_state FROM PUBLIC;
GRANT SELECT ON game_public_v2_character_dependencies_state TO oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_public_v2_character_dependencies(c UUID) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE g UUID; h RECORD; changed BOOLEAN;
BEGIN
 -- Level-ups do not re-aggregate a 2000-member roster when its public name,
 -- World and visibility did not change. Member edits have their own source trigger.
 INSERT INTO game_public_v2_character_dependencies_state AS prior
 SELECT c,r.name,r.world_id,game_public_v2_character_visible(c)
 FROM (VALUES(1)) d(n) LEFT JOIN game_character_roots r ON r.character_id=c
 ON CONFLICT(character_id) DO UPDATE SET name=EXCLUDED.name,world_id=EXCLUDED.world_id,eligible=EXCLUDED.eligible
 WHERE (prior.name,prior.world_id,prior.eligible) IS DISTINCT FROM (EXCLUDED.name,EXCLUDED.world_id,EXCLUDED.eligible)
 RETURNING true INTO changed;
 IF changed IS DISTINCT FROM true THEN RETURN; END IF;
 SELECT guild_id INTO g FROM game_guild_members WHERE character_id=c;
 IF g IS NOT NULL THEN PERFORM game_public_v2_guild(g); END IF;
 -- HOUSE-OWN-0's account/World slot bounds this set; index avoids a property scan.
 FOR h IN SELECT world_id,house_key FROM game_house_properties WHERE owner_character_id=c ORDER BY world_id,house_key LOOP
  PERFORM game_public_v2_house(h.world_id,h.house_key);
 END LOOP;
END $$;
CREATE FUNCTION game_public_v2_guild_dependencies() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c UUID; h RECORD;
BEGIN
 IF TG_TABLE_NAME='game_guild_members' THEN
  IF TG_OP<>'INSERT' THEN PERFORM game_public_profile_touch(OLD.character_id,NULL,NULL); END IF;
  IF TG_OP<>'DELETE' AND (TG_OP='INSERT' OR NEW.character_id IS DISTINCT FROM OLD.character_id) THEN
   PERFORM game_public_profile_touch(NEW.character_id,NULL,NULL);
  END IF;
 ELSE
  IF (OLD.state=4) IS DISTINCT FROM (NEW.state=4) THEN
   FOR c IN SELECT character_id FROM game_guild_members WHERE guild_id=NEW.guild_id ORDER BY character_id LOOP
    PERFORM game_public_profile_touch(c,NULL,NULL);
   END LOOP;
  END IF;
  FOR h IN SELECT world_id,house_key FROM game_house_properties WHERE owner_guild_id=NEW.guild_id ORDER BY world_id,house_key LOOP
   PERFORM game_public_v2_house(h.world_id,h.house_key);
  END LOOP;
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER public_v2_a_member_profile AFTER INSERT OR UPDATE OR DELETE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION game_public_v2_guild_dependencies();
CREATE TRIGGER public_v2_guild_owner_name AFTER UPDATE ON game_guilds FOR EACH ROW EXECUTE FUNCTION game_public_v2_guild_dependencies();

CREATE TABLE game_public_house_catalogue_state (
 world_id UUID PRIMARY KEY,
 reset_epoch NUMERIC(20,0) NOT NULL,
 map_revision TEXT NOT NULL,
 ready BOOLEAN NOT NULL,
 pending_ms BIGINT NOT NULL CHECK(pending_ms>=0)
);
REVOKE ALL ON game_public_house_catalogue_state FROM PUBLIC;
GRANT SELECT ON game_public_house_catalogue_state TO oteryn_game_runtime,oteryn_game_control;
INSERT INTO game_public_house_catalogue_state
SELECT a.world_id,a.reset_epoch,e.map_revision,false,floor(extract(epoch FROM e.activated_at)*1000)::bigint
FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch);
CREATE FUNCTION game_public_house_reset_pending() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 INSERT INTO game_public_house_catalogue_state
 SELECT NEW.world_id,NEW.reset_epoch,e.map_revision,false,floor(extract(epoch FROM clock_timestamp())*1000)::bigint
 FROM game_world_reset_epochs e WHERE e.world_id=NEW.world_id AND e.reset_epoch=NEW.reset_epoch
 ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch,map_revision=EXCLUDED.map_revision,
 ready=false,pending_ms=EXCLUDED.pending_ms;
 RETURN NULL;
END $$;
CREATE TRIGGER public_house_reset_pending AFTER INSERT OR UPDATE ON game_world_reset_active
FOR EACH ROW EXECUTE FUNCTION game_public_house_reset_pending();
CREATE FUNCTION game_public_v2_source_pending(f TEXT) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT min(x.pending_ms) FROM (
 SELECT coalesce(s.pending_ms,floor(extract(epoch FROM e.activated_at)*1000)::bigint) AS pending_ms
 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
 LEFT JOIN game_public_house_catalogue_state s ON s.world_id=a.world_id
 WHERE f='house' AND (s.world_id IS NULL OR NOT s.ready OR s.reset_epoch<>a.reset_epoch OR s.map_revision<>e.map_revision)
 UNION ALL SELECT pending_ms FROM game_public_house_source_pending WHERE f='house') x
$$;
ALTER FUNCTION game_public_v2_ready(TEXT,TEXT,TEXT) RENAME TO game_public_v2_ready_before_catalogue;
CREATE FUNCTION game_public_v2_ready(f TEXT,w TEXT,s TEXT) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT game_public_v2_ready_before_catalogue(f,w,s) AND (f<>'house' OR EXISTS(
 SELECT 1 FROM game_public_house_catalogue_state q JOIN game_world_reset_active a USING(world_id,reset_epoch)
 JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
 WHERE q.world_id=nullif(w,'')::uuid AND q.ready AND q.map_revision=e.map_revision
 AND NOT EXISTS(SELECT 1 FROM game_public_house_source_pending z WHERE z.world_id=q.world_id AND z.house_key=s)))
$$;
CREATE FUNCTION game_public_house_catalogue_sync(w UUID,ch UUID,g NUMERIC,epoch NUMERIC,map TEXT,p JSONB,n UUID,nr NUMERIC,secret BYTEA)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE current_map TEXT; stamp BIGINT:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
BEGIN
 IF game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM TRUE THEN RETURN false; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments ra
 WHERE ra.scope_kind=1 AND ra.world_id=w AND ra.channel_id=ch AND ra.state=1
 AND ra.ownership_generation=g AND ra.holder_node_id=n AND ra.holder_registration_revision=nr FOR SHARE) THEN RETURN false; END IF;
 SELECT e.map_revision INTO current_map FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
 WHERE a.world_id=w AND a.reset_epoch=epoch FOR SHARE OF a,e;
 IF NOT FOUND OR (map<>'' AND map<>current_map) THEN RETURN false; END IF;
 IF p IS NOT NULL AND (jsonb_typeof(p)<>'array' OR jsonb_array_length(p)>1000 OR octet_length(p::text)>1048576) THEN RETURN false; END IF;
 INSERT INTO game_public_house_catalogue_state VALUES(w,epoch,current_map,false,stamp)
 ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch,map_revision=EXCLUDED.map_revision,ready=false,
 pending_ms=CASE WHEN game_public_house_catalogue_state.ready OR game_public_house_catalogue_state.reset_epoch<>epoch
 OR game_public_house_catalogue_state.map_revision<>current_map THEN stamp ELSE game_public_house_catalogue_state.pending_ms END;
 -- NULL is unavailable metadata; [] is an explicit checked empty/no-bundle catalogue.
 IF p IS NULL THEN RETURN true; END IF;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(p) x WHERE NOT x ?& ARRAY['house_key','name','kind','town','size_sqm','beds','rent_gold','content_digest']
 OR x->>'house_key' !~ '^[a-z0-9._:-]{1,128}$' OR x->>'town' !~ '^[a-z0-9._:-]{1,128}$'
 OR octet_length(x->>'name') NOT BETWEEN 1 AND 64 OR (x->>'name') COLLATE "C" !~ '^[ -~]+$'
 OR strpos(x->>'name',chr(34))<>0 OR strpos(x->>'name',chr(92))<>0 OR strpos(x->>'name',chr(47))<>0
 OR x->>'kind' NOT IN('private_house','shop','guildhall') OR x->>'content_digest' !~ '^[0-9a-f]{64}$') THEN RETURN false; END IF;
 INSERT INTO game_public_house_catalogue(world_id,house_key,name,kind,town,size_sqm,beds,rent_gold,content_digest)
 SELECT w,x->>'house_key',x->>'name',x->>'kind',x->>'town',(x->>'size_sqm')::bigint,(x->>'beds')::bigint,
 (x->>'rent_gold')::bigint,decode(x->>'content_digest','hex') FROM jsonb_array_elements(p) x
 ON CONFLICT(world_id,house_key) DO UPDATE SET name=EXCLUDED.name,kind=EXCLUDED.kind,town=EXCLUDED.town,
 size_sqm=EXCLUDED.size_sqm,beds=EXCLUDED.beds,rent_gold=EXCLUDED.rent_gold,content_digest=EXCLUDED.content_digest;
 DELETE FROM game_public_house_catalogue c WHERE c.world_id=w AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p) x WHERE x->>'house_key'=c.house_key);
 -- Empty catalogues still withdraw every previously exported subject, even when
 -- their source property/cached catalogue has already disappeared.
 PERFORM game_public_v2_house(w,h.subject) FROM game_public_v2_heads h WHERE h.family='house' AND h.world_id=w::text;
 UPDATE game_public_house_catalogue_state SET ready=true WHERE world_id=w;
 RETURN true;
END $$;

DO $$ DECLARE f REGPROCEDURE; BEGIN
 FOR f IN SELECT p.oid::regprocedure FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
 WHERE n.nspname=current_schema() AND (p.proname LIKE 'game_public_v2_%' OR p.proname LIKE 'game_public_house_%') LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_public_v2_ready(TEXT,TEXT,TEXT),game_public_v2_source_pending(TEXT)
TO oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_public_house_catalogue_sync(UUID,UUID,NUMERIC,NUMERIC,TEXT,JSONB,UUID,NUMERIC,BYTEA)
TO oteryn_game_runtime;

-- Baseline existing Guilds; profiles are refreshed after all PUBPROJ migrations
-- (the profile dependency seam also reaches 1352's death classification).
DO $$ DECLARE g UUID; BEGIN
 FOR g IN SELECT guild_id FROM game_guilds ORDER BY guild_id LOOP
  PERFORM game_public_v2_guild(g);
 END LOOP;
END $$;
