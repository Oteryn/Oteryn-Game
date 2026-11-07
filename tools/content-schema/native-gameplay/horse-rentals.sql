-- S06 source-only temporary horse contracts, never Account appearance grants.
CREATE TABLE game_horse_station_worlds(
 world_id UUID PRIMARY KEY,generation BIGINT NOT NULL CHECK(generation>=0),started_seconds BIGINT NOT NULL,
 expires_seconds BIGINT NOT NULL,ended BOOLEAN NOT NULL,
 CHECK((generation=0 AND started_seconds=0 AND expires_seconds=0 AND ended) OR(generation>0 AND started_seconds>0 AND expires_seconds=started_seconds+86400)));
CREATE TABLE game_horse_rental_receipts(
 transaction_id UUID PRIMARY KEY,character_id UUID NOT NULL REFERENCES game_character_roots(character_id),world_id UUID NOT NULL,
 game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),command_id NUMERIC(20,0) NOT NULL,
 npc_key TEXT NOT NULL CHECK(npc_key IN('oteryn:npc.appaloosa','oteryn:npc.palomino')),
 mount_id INTEGER NOT NULL CHECK(mount_id IN(22,25,26)),issued_seconds BIGINT NOT NULL CHECK(issued_seconds>0),
 expires_seconds BIGINT NOT NULL CHECK(expires_seconds=issued_seconds+86400),fee_transaction_id UUID NOT NULL REFERENCES game_item_fee_burns(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),committed_character_revision NUMERIC(20,0) NOT NULL,
 original_character_revision NUMERIC(20,0) NOT NULL CHECK(original_character_revision>=1),
 level_before BIGINT NOT NULL,level_after BIGINT NOT NULL CHECK(level_after=level_before),
 experience_before BIGINT NOT NULL,experience_after BIGINT NOT NULL CHECK(experience_after=experience_before),
 profile_revision TEXT NOT NULL,ruleset_revision TEXT NOT NULL,content_revision TEXT NOT NULL,
 simulation_revision TEXT NOT NULL,evidence_revision TEXT NOT NULL,declaration_revision TEXT NOT NULL,
 policy_revision TEXT NOT NULL,reward_revision TEXT NOT NULL,
 CHECK(committed_character_revision=original_character_revision+1 AND committed_character_revision<=18446744073709551615),
 UNIQUE(character_id,committed_character_revision),
 created_xact_id XID8 NOT NULL,UNIQUE(game_session_id,command_id));
CREATE INDEX game_horse_rental_character ON game_horse_rental_receipts(character_id,issued_seconds DESC);
CREATE INDEX game_horse_rental_fee ON game_horse_rental_receipts(fee_transaction_id);
CREATE TABLE game_horse_station_wave_claims(
 world_id UUID NOT NULL REFERENCES game_horse_station_worlds(world_id),generation BIGINT NOT NULL,channel_id UUID NOT NULL,
 wave SMALLINT NOT NULL CHECK(wave BETWEEN -1 AND 7),node_id UUID NOT NULL,registration NUMERIC NOT NULL,scope_generation NUMERIC NOT NULL,
 created_xact_id XID8 NOT NULL,PRIMARY KEY(world_id,generation,channel_id,wave,node_id,registration,scope_generation));
CREATE TABLE game_horse_station_catches(
 world_id UUID NOT NULL REFERENCES game_horse_station_worlds(world_id),generation BIGINT NOT NULL,channel_id UUID NOT NULL,
 movement_occurrence BYTEA NOT NULL CHECK(octet_length(movement_occurrence)=32),ended BOOLEAN NOT NULL,created_xact_id XID8 NOT NULL,
 PRIMARY KEY(world_id,generation,channel_id,movement_occurrence));
CREATE TABLE game_horse_rental_expiry_notices(
 transaction_id UUID PRIMARY KEY REFERENCES game_horse_rental_receipts(transaction_id),game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),created_xact_id XID8 NOT NULL);
CREATE INDEX game_horse_rental_expiry_session ON game_horse_rental_expiry_notices(game_session_id);
CREATE TABLE game_horse_station_context(
 backend_pid INTEGER PRIMARY KEY,transaction_xid XID8 NOT NULL,world_id UUID NOT NULL,channel_id UUID NOT NULL,generation NUMERIC NOT NULL,
 node_id UUID NOT NULL,registration NUMERIC NOT NULL,phase TEXT NOT NULL CHECK(phase IN('node','rent','catch','notice')),
 character_id UUID,game_session_id UUID,connection NUMERIC,lease NUMERIC,command_id NUMERIC,npc_key TEXT,request_binding BYTEA,fee_id UUID,
 original_revision NUMERIC,progression_before JSONB,source_seed BYTEA,mount_id INTEGER,issued_seconds BIGINT,expires_seconds BIGINT,movement_occurrence BYTEA,notice_house TEXT);
REVOKE ALL ON game_horse_station_context FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_horse_station_draw(p_domain INTEGER,p_seed BYTEA,p_bound INTEGER) RETURNS INTEGER LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE h BYTEA;v NUMERIC;lim NUMERIC;attempt INTEGER;j INTEGER;
BEGIN
 IF p_domain NOT BETWEEN 217 AND 221 OR p_bound<1 OR p_bound>100000 OR octet_length(p_seed)<>32 THEN RAISE EXCEPTION 'invalid horse source draw' USING ERRCODE='23514';END IF;
 lim:=floor(18446744073709551616::numeric/p_bound)*p_bound;
 FOR attempt IN 0..15 LOOP
 h:=sha256(convert_to('oteryn:horse-station-rng:v1','UTF8')||int4send(p_domain)||p_seed||int4send(attempt));v:=0;
 FOR j IN 0..7 LOOP v:=v*256+get_byte(h,j);END LOOP;
 IF v<lim THEN RETURN mod(v,p_bound)::integer+1;END IF;
 END LOOP;RAISE EXCEPTION 'horse source draw rejection budget' USING ERRCODE='23514';
END $$;
CREATE FUNCTION game_horse_station_current() RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM game_horse_station_context c JOIN game_runtime_scope_assignments a
 ON a.world_id=c.world_id AND((c.phase='notice' AND c.notice_house IS NOT NULL AND a.scope_kind=2 AND a.house_key=c.notice_house)
 OR(c.notice_house IS NULL AND a.scope_key='\x01'::bytea||uuid_send(c.world_id)||uuid_send(c.channel_id) AND a.scope_kind=1))
 WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id() AND a.state=1
 AND a.ownership_generation=c.generation AND a.holder_node_id=c.node_id AND a.holder_registration_revision=c.registration)
$$;
CREATE FUNCTION game_horse_rental_character_current() RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT game_horse_station_current() AND EXISTS(SELECT 1 FROM game_horse_station_context c
 JOIN game_durability_reconnect_sessions s ON s.game_session_id=c.game_session_id
 JOIN game_character_roots r ON r.character_id=c.character_id
 JOIN game_durability_admission_character_guards cg ON cg.character_id=c.character_id
 JOIN game_durability_admission_account_guards ag ON ag.account_id=cg.account_id
 JOIN game_runtime_scope_assignments a ON a.world_id=c.world_id AND a.state=1 AND a.ownership_generation=c.generation
 AND a.holder_node_id=c.node_id AND a.holder_registration_revision=c.registration
 AND((c.phase='notice' AND c.notice_house IS NOT NULL AND a.scope_kind=2 AND a.house_key=c.notice_house)
 OR(c.notice_house IS NULL AND a.scope_kind=1 AND a.channel_id=c.channel_id))
 JOIN game_durability_admission_runtime_guards rg ON rg.scope_key=a.scope_key
 WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
 AND s.character_id=c.character_id AND s.world_id=c.world_id AND s.runtime_scope_world_id=c.world_id
 AND((c.notice_house IS NULL AND s.runtime_scope_kind=1 AND s.runtime_scope_channel_id=c.channel_id AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL)
 OR(c.phase='notice' AND c.notice_house IS NOT NULL AND s.runtime_scope_kind=2 AND s.runtime_scope_instance_id IS NOT NULL AND s.runtime_scope_house_key=c.notice_house AND s.runtime_scope_channel_id IS NULL AND s.origin_channel_id=c.channel_id))
 AND s.current_generation=c.connection AND s.character_lease_generation=c.lease AND s.scope_ownership_generation=c.generation AND s.session_state IN(1,2)
 AND r.world_id=c.world_id AND r.account_id=s.account_id AND r.lifecycle=1
 AND cg.account_id=s.account_id AND cg.world_id=c.world_id AND cg.eligible
 AND cg.lease_generation=c.lease AND cg.holder_game_session_id=c.game_session_id
 AND ag.presence_character_id=c.character_id AND ag.holder_game_session_id=c.game_session_id AND rg.ready AND rg.ownership_generation=c.generation)
$$;
CREATE FUNCTION game_horse_station_arm_node(p_world UUID,p_channel UUID,p_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF p_secret IS NULL OR octet_length(p_secret)<>32 OR game_node_prove_current_incarnation(p_node,p_registration,p_secret) IS DISTINCT FROM TRUE
 OR NOT game_runtime_scope_assignment_history_valid() THEN RAISE EXCEPTION 'horse source requires Node possession' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_runtime_scope_assignments a WHERE a.scope_key='\x01'::bytea||uuid_send(p_world)||uuid_send(p_channel)
 AND a.scope_kind=1 AND a.world_id=p_world AND a.channel_id=p_channel AND a.state=1 AND a.ownership_generation=p_generation
 AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'stale horse source Channel' USING ERRCODE='23514';END IF;
 DELETE FROM game_horse_station_context c WHERE c.backend_pid=pg_backend_pid() OR NOT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.pid=c.backend_pid);
 INSERT INTO game_horse_station_context(backend_pid,transaction_xid,world_id,channel_id,generation,node_id,registration,phase)
 VALUES(pg_backend_pid(),pg_current_xact_id(),p_world,p_channel,p_generation,p_node,p_registration,'node');
END $$;
CREATE FUNCTION game_horse_rental_arm(p_world UUID,p_channel UUID,p_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA,
 p_character UUID,p_session UUID,p_connection NUMERIC,p_lease NUMERIC,p_command NUMERIC,p_npc TEXT,p_binding BYTEA,p_fee UUID)
RETURNS TABLE(mount_id INTEGER,expires_seconds BIGINT) LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE n BIGINT;seed BYTEA;chosen INTEGER;progress_before JSONB;
BEGIN
 PERFORM game_horse_station_arm_node(p_world,p_channel,p_generation,p_node,p_registration,p_secret);
 IF p_npc NOT IN('oteryn:npc.appaloosa','oteryn:npc.palomino') OR p_npc IS NULL OR p_command IS NULL OR p_command<1 OR p_command>18446744073709551615
 OR p_binding IS NULL OR octet_length(p_binding)<>32 OR p_fee IS NULL THEN RAISE EXCEPTION 'invalid horse rental source' USING ERRCODE='23514';END IF;
 n:=floor(extract(epoch FROM public.game_daily_effect_now()))::bigint;
 seed:=sha256(uuid_send(p_world)||uuid_send(p_character)||uuid_send(p_session)||convert_to(p_command::text,'UTF8'));
 chosen:=(ARRAY[22,25,26])[game_horse_station_draw(217,seed,3)];
 SELECT to_jsonb(progression) INTO STRICT progress_before FROM game_character_progression_state progression WHERE progression.character_id=p_character FOR UPDATE;
 UPDATE game_horse_station_context SET phase='rent',character_id=p_character,game_session_id=p_session,connection=p_connection,lease=p_lease,
 command_id=p_command,npc_key=p_npc,request_binding=p_binding,fee_id=p_fee,original_revision=(SELECT r.character_revision FROM game_character_roots r WHERE r.character_id=p_character FOR UPDATE),progression_before=progress_before,source_seed=seed,mount_id=chosen,issued_seconds=n,expires_seconds=n+86400
 WHERE backend_pid=pg_backend_pid();
 IF NOT game_horse_rental_character_current() OR EXISTS(SELECT 1 FROM game_horse_rental_receipts r WHERE r.character_id=p_character AND r.expires_seconds>=n)
 OR EXISTS(SELECT 1 FROM game_horse_station_worlds w WHERE w.world_id=p_world AND NOT w.ended AND w.expires_seconds>n)
 OR EXISTS(SELECT 1 FROM game_item_fee_burns f WHERE f.transaction_id=p_fee) THEN
 RAISE EXCEPTION 'horse contract requires current Character and available stable' USING ERRCODE='23514';END IF;
 RETURN QUERY SELECT chosen,n+86400;
END $$;
CREATE FUNCTION game_horse_rental_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_horse_station_context%ROWTYPE;
BEGIN
 SELECT*INTO c FROM game_horse_station_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF game_horse_rental_character_current() IS DISTINCT FROM TRUE OR c.phase IS DISTINCT FROM'rent'
 OR NEW.committed_character_revision IS DISTINCT FROM c.original_revision+1
 OR NEW.original_character_revision IS DISTINCT FROM c.original_revision
 OR (c.progression_before->>'character_revision')::numeric IS DISTINCT FROM c.original_revision
 OR NEW.level_before IS DISTINCT FROM (c.progression_before->>'level')::bigint
 OR NEW.experience_before IS DISTINCT FROM (c.progression_before->>'total_experience')::bigint
 OR NEW.profile_revision IS DISTINCT FROM c.progression_before->>'profile_revision'
 OR NEW.ruleset_revision IS DISTINCT FROM c.progression_before->>'ruleset_revision'
 OR NEW.content_revision IS DISTINCT FROM c.progression_before->>'content_revision'
 OR NEW.simulation_revision IS DISTINCT FROM c.progression_before->>'simulation_revision'
 OR NEW.evidence_revision IS DISTINCT FROM c.progression_before->>'evidence_revision'
 OR NEW.declaration_revision IS DISTINCT FROM c.progression_before->>'declaration_revision'
 OR NEW.policy_revision IS DISTINCT FROM c.progression_before->>'policy_revision'
 OR NEW.reward_revision IS DISTINCT FROM c.progression_before->>'reward_revision'
 OR(NEW.character_id,NEW.world_id,NEW.game_session_id,NEW.command_id,NEW.npc_key,NEW.mount_id,NEW.issued_seconds,NEW.expires_seconds,NEW.fee_transaction_id,NEW.request_binding,NEW.transaction_id)
 IS DISTINCT FROM(c.character_id,c.world_id,c.game_session_id,c.command_id,c.npc_key,c.mount_id,c.issued_seconds,c.expires_seconds,c.fee_id,c.request_binding,c.fee_id) THEN
 RAISE EXCEPTION 'horse rental requires current source fee intent' USING ERRCODE='23514';END IF;
 NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_horse_rental_admit BEFORE INSERT ON game_horse_rental_receipts FOR EACH ROW EXECUTE FUNCTION game_horse_rental_admit();
CREATE TRIGGER game_horse_rental_immutable BEFORE UPDATE OR DELETE ON game_horse_rental_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_horse_rental_no_truncate BEFORE TRUNCATE ON game_horse_rental_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE FUNCTION game_horse_rental_complete() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_horse_station_context%ROWTYPE;
BEGIN
 SELECT*INTO c FROM game_horse_station_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF game_horse_rental_character_current() IS DISTINCT FROM TRUE OR NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
 SELECT 1 FROM game_item_fee_burns f WHERE f.transaction_id=NEW.fee_transaction_id AND f.character_id=NEW.character_id AND f.world_id=NEW.world_id
 AND f.created_xact_id=pg_current_xact_id() AND f.channel_id=c.channel_id AND f.runtime_scope_ownership_generation=c.generation AND f.cause_kind=473 AND f.fee_gold_units=500
 AND f.purchase_service_key='oteryn:service.horse-station.rental' AND f.committed_character_revision=NEW.committed_character_revision) THEN
 RAISE EXCEPTION 'horse rental requires reciprocal audited500gold same transaction' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_horse_rental_complete AFTER INSERT ON game_horse_rental_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_horse_rental_complete();
-- World mutation is driven only by this admitted source receipt or secret-backed catch/cadence.
CREATE FUNCTION game_horse_station_world_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_horse_station_context%ROWTYPE;
BEGIN
 SELECT*INTO c FROM game_horse_station_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF game_horse_station_current() IS DISTINCT FROM TRUE OR NEW.world_id IS DISTINCT FROM c.world_id THEN RAISE EXCEPTION 'horse world requires current Node' USING ERRCODE='23514';END IF;
 IF TG_OP='INSERT' AND NEW.generation=0 AND NEW.ended AND NEW.started_seconds=0 AND NEW.expires_seconds=0 THEN RETURN NEW;END IF;
 IF c.phase='rent' AND game_horse_rental_character_current() AND EXISTS(SELECT 1 FROM game_horse_rental_receipts r WHERE r.transaction_id=c.fee_id AND r.created_xact_id=pg_current_xact_id())
 AND game_horse_station_draw(218,c.source_seed,10)<=3 AND(TG_OP='INSERT' AND NEW.generation=1 OR TG_OP='UPDATE' AND NEW.generation=OLD.generation+1)
 AND NEW.started_seconds=c.issued_seconds AND NEW.expires_seconds=c.issued_seconds+86400 AND NOT NEW.ended THEN RETURN NEW;END IF;
 IF TG_OP='UPDATE' AND NEW.generation=OLD.generation AND NEW.started_seconds=OLD.started_seconds AND NEW.expires_seconds=OLD.expires_seconds AND NEW.ended
 AND(c.phase='catch' AND EXISTS(SELECT 1 FROM game_horse_station_catches h WHERE h.world_id=c.world_id AND h.generation=OLD.generation AND h.channel_id=c.channel_id AND h.movement_occurrence=c.movement_occurrence AND h.ended AND h.created_xact_id=pg_current_xact_id()) OR c.phase='node' AND floor(extract(epoch FROM public.game_daily_effect_now()))::bigint>=OLD.expires_seconds) THEN RETURN NEW;END IF;
 RAISE EXCEPTION 'horse world source transition refused' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_horse_station_world_admit BEFORE INSERT OR UPDATE ON game_horse_station_worlds FOR EACH ROW EXECUTE FUNCTION game_horse_station_world_admit();
CREATE TRIGGER game_horse_station_world_no_delete BEFORE DELETE ON game_horse_station_worlds FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_horse_station_world_no_truncate BEFORE TRUNCATE ON game_horse_station_worlds FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE FUNCTION game_horse_rental_start_world() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_horse_station_context%ROWTYPE;
BEGIN
 SELECT*INTO c FROM game_horse_station_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF game_horse_rental_character_current() IS DISTINCT FROM TRUE OR NEW.created_xact_id<>pg_current_xact_id() THEN RAISE EXCEPTION 'horse rental source lost' USING ERRCODE='23514';END IF;
 IF game_horse_station_draw(218,c.source_seed,10)<=3 THEN
 INSERT INTO game_horse_station_worlds(world_id,generation,started_seconds,expires_seconds,ended) VALUES(NEW.world_id,1,NEW.issued_seconds,NEW.issued_seconds+86400,FALSE)
 ON CONFLICT(world_id)DO UPDATE SET generation=game_horse_station_worlds.generation+1,started_seconds=EXCLUDED.started_seconds,expires_seconds=EXCLUDED.expires_seconds,ended=FALSE;
 END IF;RETURN NULL;
END $$;
CREATE TRIGGER game_horse_rental_start_world AFTER INSERT ON game_horse_rental_receipts FOR EACH ROW EXECUTE FUNCTION game_horse_rental_start_world();
CREATE FUNCTION game_horse_station_wave_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_horse_station_context%ROWTYPE;w game_horse_station_worlds%ROWTYPE;n BIGINT;expected INTEGER;
BEGIN
 SELECT*INTO c FROM game_horse_station_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 SELECT*INTO w FROM game_horse_station_worlds WHERE world_id=c.world_id;
 n:=floor(extract(epoch FROM public.game_daily_effect_now()))::bigint;
 expected:=CASE WHEN NOT w.ended AND w.expires_seconds>n THEN((n-w.started_seconds)/10800)::integer ELSE-1 END;
 IF game_horse_station_current() IS DISTINCT FROM TRUE OR c.phase IS DISTINCT FROM'node'
 OR(NEW.world_id,NEW.channel_id,NEW.generation,NEW.wave,NEW.node_id,NEW.registration,NEW.scope_generation)IS DISTINCT FROM(c.world_id,c.channel_id,w.generation,expected::smallint,c.node_id,c.registration,c.generation) THEN
 RAISE EXCEPTION 'horse physical source slot requires current World and Channel' USING ERRCODE='23514';END IF;
 NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_horse_station_wave_admit BEFORE INSERT ON game_horse_station_wave_claims FOR EACH ROW EXECUTE FUNCTION game_horse_station_wave_admit();
CREATE TRIGGER game_horse_station_wave_immutable BEFORE UPDATE OR DELETE ON game_horse_station_wave_claims FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_horse_station_wave_no_truncate BEFORE TRUNCATE ON game_horse_station_wave_claims FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE FUNCTION game_horse_station_refresh_claim() RETURNS TABLE(generation BIGINT,started_seconds BIGINT,expires_seconds BIGINT,ended BOOLEAN,wave SMALLINT,claimed BOOLEAN)
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_horse_station_context%ROWTYPE;w game_horse_station_worlds%ROWTYPE;n BIGINT;s SMALLINT;
BEGIN
 SELECT*INTO c FROM game_horse_station_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF game_horse_station_current() IS DISTINCT FROM TRUE OR c.phase IS DISTINCT FROM'node' THEN RAISE EXCEPTION 'horse cadence requires current Node' USING ERRCODE='23514';END IF;
 INSERT INTO game_horse_station_worlds VALUES(c.world_id,0,0,0,TRUE)ON CONFLICT(world_id)DO NOTHING;
 SELECT*INTO w FROM game_horse_station_worlds WHERE world_id=c.world_id FOR UPDATE;
 n:=floor(extract(epoch FROM public.game_daily_effect_now()))::bigint;
 IF NOT w.ended AND w.expires_seconds<=n THEN UPDATE game_horse_station_worlds SET ended=TRUE WHERE world_id=c.world_id;w.ended:=TRUE;END IF;
 s:=CASE WHEN NOT w.ended AND w.expires_seconds>n THEN((n-w.started_seconds)/10800)::smallint ELSE-1 END;
 INSERT INTO game_horse_station_wave_claims(world_id,generation,channel_id,wave,node_id,registration,scope_generation,created_xact_id)VALUES(c.world_id,w.generation,c.channel_id,s,c.node_id,c.registration,c.generation,pg_current_xact_id())ON CONFLICT DO NOTHING;
 -- A retried ambiguous commit and a newly admitted owner must reconstruct its
 -- ephemeral native actors. The current runtime suppresses an already installed slot.
 RETURN QUERY SELECT w.generation,w.started_seconds,w.expires_seconds,w.ended,s,TRUE;
END $$;
CREATE FUNCTION game_horse_station_catch(p_occurrence BYTEA) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_horse_station_context%ROWTYPE;w game_horse_station_worlds%ROWTYPE;n BIGINT;result BOOLEAN;seed BYTEA;
BEGIN
 SELECT*INTO c FROM game_horse_station_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF game_horse_station_current() IS DISTINCT FROM TRUE OR c.phase IS DISTINCT FROM'node' OR p_occurrence IS NULL OR octet_length(p_occurrence)<>32 THEN
 RAISE EXCEPTION 'horse catch requires current admitted monster source step' USING ERRCODE='23514';END IF;
 INSERT INTO game_horse_station_worlds VALUES(c.world_id,0,0,0,TRUE)ON CONFLICT(world_id)DO NOTHING;
 SELECT*INTO w FROM game_horse_station_worlds WHERE world_id=c.world_id FOR UPDATE;
 n:=floor(extract(epoch FROM public.game_daily_effect_now()))::bigint;
 SELECT ended INTO result FROM game_horse_station_catches WHERE world_id=c.world_id AND generation=w.generation AND channel_id=c.channel_id AND movement_occurrence=p_occurrence;
 IF FOUND THEN RETURN result;END IF;
 IF w.world_id IS NULL OR w.ended OR w.expires_seconds<=n THEN RETURN FALSE;END IF;
 seed:=sha256(uuid_send(c.world_id)||int8send(w.generation)||uuid_send(c.channel_id)||p_occurrence);
 result:=game_horse_station_draw(219,seed,30)=1;
 UPDATE game_horse_station_context SET phase='catch',movement_occurrence=p_occurrence WHERE backend_pid=pg_backend_pid();
 INSERT INTO game_horse_station_catches VALUES(c.world_id,w.generation,c.channel_id,p_occurrence,result,pg_current_xact_id());
 IF result THEN UPDATE game_horse_station_worlds SET ended=TRUE WHERE world_id=c.world_id;END IF;
 RETURN result;
END $$;
CREATE FUNCTION game_horse_station_catch_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_horse_station_context%ROWTYPE;w game_horse_station_worlds%ROWTYPE;seed BYTEA;
BEGIN
 SELECT*INTO c FROM game_horse_station_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 SELECT*INTO w FROM game_horse_station_worlds WHERE world_id=c.world_id;
 seed:=sha256(uuid_send(c.world_id)||int8send(w.generation)||uuid_send(c.channel_id)||c.movement_occurrence);
 IF game_horse_station_current() IS DISTINCT FROM TRUE OR c.phase IS DISTINCT FROM'catch'
 OR(NEW.world_id,NEW.channel_id,NEW.generation,NEW.movement_occurrence,NEW.ended)IS DISTINCT FROM(c.world_id,c.channel_id,w.generation,c.movement_occurrence,game_horse_station_draw(219,seed,30)=1)THEN
 RAISE EXCEPTION 'horse catch requires exact source occurrence and draw' USING ERRCODE='23514';END IF;
 NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_horse_station_catch_admit BEFORE INSERT ON game_horse_station_catches FOR EACH ROW EXECUTE FUNCTION game_horse_station_catch_admit();
CREATE TRIGGER game_horse_station_catch_immutable BEFORE UPDATE OR DELETE ON game_horse_station_catches FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_horse_station_catch_no_truncate BEFORE TRUNCATE ON game_horse_station_catches FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();
GRANT SELECT,INSERT ON game_horse_rental_receipts,game_horse_station_wave_claims,game_horse_station_catches,game_horse_rental_expiry_notices TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_horse_station_worlds TO oteryn_game_runtime;
CREATE FUNCTION game_horse_rental_expiry_notice(p_character UUID,p_session UUID,p_connection NUMERIC,p_lease NUMERIC) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_horse_station_context%ROWTYPE;r game_horse_rental_receipts%ROWTYPE;n BIGINT;rows INTEGER;
BEGIN
 IF game_horse_station_current() IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'horse expiry requires current Node' USING ERRCODE='23514';END IF;
 UPDATE game_horse_station_context SET phase='notice',character_id=p_character,game_session_id=p_session,connection=p_connection,lease=p_lease WHERE backend_pid=pg_backend_pid();
 IF game_horse_rental_character_current() IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'horse expiry requires current Character' USING ERRCODE='23514';END IF;
 SELECT*INTO r FROM game_horse_rental_receipts WHERE character_id=p_character ORDER BY issued_seconds DESC,transaction_id DESC LIMIT 1;
 n:=floor(extract(epoch FROM public.game_daily_effect_now()))::bigint;
 IF r.transaction_id IS NULL OR r.expires_seconds>=n THEN RETURN FALSE;END IF;
 UPDATE game_horse_station_context SET fee_id=r.transaction_id WHERE backend_pid=pg_backend_pid();
 INSERT INTO game_horse_rental_expiry_notices VALUES(r.transaction_id,p_session,pg_current_xact_id())ON CONFLICT DO NOTHING;
 GET DIAGNOSTICS rows=ROW_COUNT;RETURN rows=1;
END $$;
CREATE FUNCTION game_horse_rental_expiry_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c game_horse_station_context%ROWTYPE;
BEGIN
 SELECT*INTO c FROM game_horse_station_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF game_horse_rental_character_current() IS DISTINCT FROM TRUE OR c.phase IS DISTINCT FROM'notice'
 OR(NEW.transaction_id,NEW.game_session_id)IS DISTINCT FROM(c.fee_id,c.game_session_id)
 OR NOT EXISTS(SELECT 1 FROM game_horse_rental_receipts r WHERE r.transaction_id=c.fee_id AND r.character_id=c.character_id
 AND r.expires_seconds<floor(extract(epoch FROM public.game_daily_effect_now()))::bigint) THEN
 RAISE EXCEPTION 'horse expiry notice requires current contract source' USING ERRCODE='23514';END IF;
 NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_horse_rental_expiry_admit BEFORE INSERT ON game_horse_rental_expiry_notices FOR EACH ROW EXECUTE FUNCTION game_horse_rental_expiry_admit();
CREATE TRIGGER game_horse_rental_expiry_immutable BEFORE UPDATE OR DELETE ON game_horse_rental_expiry_notices FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_horse_rental_expiry_no_truncate BEFORE TRUNCATE ON game_horse_rental_expiry_notices FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();

CREATE FUNCTION game_horse_rental_arm_notice(p_world UUID,p_character UUID,p_session UUID,p_connection NUMERIC,p_lease NUMERIC,p_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE s game_durability_reconnect_sessions%ROWTYPE;
BEGIN
 IF p_secret IS NULL OR octet_length(p_secret)<>32 OR game_node_prove_current_incarnation(p_node,p_registration,p_secret) IS DISTINCT FROM TRUE
 OR NOT game_runtime_scope_assignment_history_valid() THEN RAISE EXCEPTION 'horse expiry requires current Node possession' USING ERRCODE='23514';END IF;
 SELECT * INTO s FROM game_durability_reconnect_sessions WHERE game_session_id=p_session AND character_id=p_character AND world_id=p_world FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'horse expiry current session missing' USING ERRCODE='23514';END IF;
 DELETE FROM game_horse_station_context c WHERE c.backend_pid=pg_backend_pid() OR NOT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.pid=c.backend_pid);
 INSERT INTO game_horse_station_context(backend_pid,transaction_xid,world_id,channel_id,generation,node_id,registration,phase,character_id,game_session_id,connection,lease,notice_house)
 VALUES(pg_backend_pid(),pg_current_xact_id(),p_world,COALESCE(s.runtime_scope_channel_id,s.origin_channel_id),p_generation,p_node,p_registration,'notice',p_character,p_session,p_connection,p_lease,s.runtime_scope_house_key);
 IF game_horse_rental_character_current() IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'horse expiry stale Character/session/lease/scope' USING ERRCODE='23514';END IF;
 RETURN game_horse_rental_expiry_notice(p_character,p_session,p_connection,p_lease);
END $$;
CREATE FUNCTION game_horse_rental_fee_current(p_fee UUID,p_character UUID,p_world UUID) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT game_horse_rental_character_current() AND EXISTS(SELECT 1 FROM game_horse_station_context c WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
 AND c.phase='rent' AND c.fee_id=p_fee AND c.character_id=p_character AND c.world_id=p_world)
$$;
CREATE FUNCTION game_horse_rental_fee_complete(p_fee UUID,p_character UUID,p_world UUID) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT game_horse_rental_fee_current(p_fee,p_character,p_world) AND EXISTS(SELECT 1 FROM game_horse_rental_receipts r JOIN game_item_fee_burns f ON f.transaction_id=r.fee_transaction_id
 JOIN game_character_roots cr ON cr.character_id=r.character_id AND cr.character_revision=r.committed_character_revision
 WHERE r.transaction_id=p_fee AND r.fee_transaction_id=p_fee AND r.character_id=p_character AND r.world_id=p_world
 AND r.created_xact_id=pg_current_xact_id() AND f.created_xact_id=pg_current_xact_id() AND f.cause_kind=473 AND f.fee_gold_units=500
 AND f.committed_character_revision=r.committed_character_revision AND cr.xmin=pg_current_xact_id()::xid)
$$;

-- This immutable rental receipt owns the single edge already advanced by the
-- fee composition. Unchanged XP/level and original pins are privately captured.
DO $rental_chain$ DECLARE definition TEXT; BEGIN
 definition:=rtrim(pg_get_viewdef('game_character_hazard_revision_chain'::regclass,true),E';\n ');
 IF position('game_horse_rental_receipts' IN definition)>0 THEN RAISE EXCEPTION 'rental edge already installed';END IF;
 EXECUTE 'CREATE OR REPLACE VIEW game_character_hazard_revision_chain WITH (security_invoker=true) AS '
 ||definition||' UNION ALL SELECT character_id,original_character_revision,committed_character_revision,'
 ||'level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,'
 ||'content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision '
 ||'FROM game_horse_rental_receipts';
END $rental_chain$;
CREATE CONSTRAINT TRIGGER game_horse_rental_character_chain AFTER INSERT ON game_horse_rental_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
