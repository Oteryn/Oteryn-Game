-- Closed actual source STEP authority; every older lever/session publisher stays intact.
CREATE TABLE game_summer_portal_context (
 backend_pid INTEGER PRIMARY KEY, transaction_xid XID8 NOT NULL,
 handoff_id UUID NOT NULL, join_id UUID NOT NULL, phase TEXT NOT NULL CHECK(phase IN('open','join')),
 world_id UUID NOT NULL,channel_id UUID NOT NULL,source_generation NUMERIC NOT NULL,
 instance_id UUID NOT NULL,destination_generation NUMERIC NOT NULL,
 node_id UUID NOT NULL,registration NUMERIC NOT NULL,
 character_id UUID NOT NULL,source_session UUID NOT NULL,destination_session UUID NOT NULL,
 source_connection NUMERIC NOT NULL,source_lease NUMERIC NOT NULL,step_occurrence NUMERIC NOT NULL,
 request_sha256 BYTEA NOT NULL CHECK(octet_length(request_sha256)=32),
 group_kind SMALLINT NOT NULL,group_id UUID NOT NULL,membership_sha256 BYTEA NOT NULL,
 source_map_digest BYTEA NOT NULL CHECK(octet_length(source_map_digest)=32)
);
REVOKE ALL ON game_summer_portal_context FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_summer_portal_group(p_world UUID,p_character UUID,p_channel UUID)
RETURNS TABLE(kind SMALLINT,identity UUID,membership BYTEA) LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=p_character AND world_id=p_world AND lifecycle=1) THEN RETURN;END IF;
 RETURN QUERY SELECT 3::smallint,p_channel,sha256(convert_to('oteryn:summer-source-channel-room:v1','UTF8')||uuid_send(p_world)||uuid_send(p_channel));
END $$;
REVOKE ALL ON FUNCTION game_summer_portal_group(uuid,uuid,uuid) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_summer_portal_arm(p_handoff UUID,p_join UUID,p_world UUID,p_channel UUID,p_source_generation NUMERIC,
 p_instance UUID,p_destination_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA,
 p_character UUID,p_source UUID,p_destination UUID,p_connection NUMERIC,p_lease NUMERIC,p_step NUMERIC,p_binding BYTEA)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE cohort RECORD;phase TEXT;source_map BYTEA;now_ms BIGINT;
BEGIN
 IF p_handoff IS NULL OR p_join IS NULL OR p_destination=p_source OR p_step IS NULL OR p_step<=0
 OR p_binding IS NULL OR octet_length(p_binding)<>32 OR p_secret IS NULL OR octet_length(p_secret)<>32
 OR NOT game_node_prove_current_incarnation(p_node,p_registration,p_secret)
 OR NOT game_runtime_scope_assignment_history_valid() THEN
  RAISE EXCEPTION 'Summer portal requires current Node and exact STEP identity' USING ERRCODE='23514';
 END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:summer-source-room:rakesh:'||encode(uuid_send(p_world),'hex')||':'||encode(uuid_send(p_channel),'hex'),0));
 SELECT * INTO cohort FROM game_summer_portal_group(p_world,p_character,p_channel);
 IF NOT FOUND THEN RAISE EXCEPTION 'Summer portal requires an actual current Character' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_runtime_scope_assignments a
 WHERE a.scope_key='\x01'::bytea||uuid_send(p_world)||uuid_send(p_channel) AND a.scope_kind=1
 AND a.world_id=p_world AND a.channel_id=p_channel AND a.state=1 AND a.ownership_generation=p_source_generation
 AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'Summer source scope is stale' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_runtime_scope_assignments a
 JOIN game_durability_admission_runtime_guards rg ON rg.scope_key='\x02'::bytea||uuid_send(p_world)||uuid_send(p_instance)
 WHERE a.scope_key='\x03'::bytea||uuid_send(p_world)||uuid_send(p_instance) AND a.scope_kind=3
 AND a.world_id=p_world AND a.activity_instance_id=p_instance AND a.state=1 AND a.ownership_generation=p_destination_generation
 AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration AND rg.ready AND rg.ownership_generation=p_destination_generation FOR SHARE OF a,rg;
 IF NOT FOUND THEN RAISE EXCEPTION 'Summer destination scope is stale' USING ERRCODE='23514';END IF;
 SELECT pos.map_revision_digest INTO source_map FROM game_durability_reconnect_sessions s
 JOIN game_character_roots r ON r.character_id=s.character_id
 JOIN game_character_last_positions pos ON pos.character_id=r.character_id
 JOIN game_durability_admission_character_guards cg ON cg.character_id=r.character_id
 JOIN game_durability_admission_account_guards ag ON ag.account_id=r.account_id
 JOIN game_durability_admission_runtime_guards rg ON rg.scope_key='\x01'::bytea||uuid_send(p_world)||uuid_send(p_channel)
 WHERE s.game_session_id=p_source AND s.character_id=p_character AND s.world_id=p_world
 AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=p_world AND s.runtime_scope_channel_id=p_channel
 AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL AND s.origin_channel_id IS NULL
 AND s.session_state IN(1,2) AND s.current_generation=p_connection AND s.character_lease_generation=p_lease
 AND s.scope_ownership_generation=p_source_generation AND r.world_id=p_world AND r.account_id=s.account_id AND r.lifecycle=1
 AND cg.account_id=r.account_id AND cg.world_id=p_world AND cg.eligible AND cg.lease_generation=p_lease AND cg.holder_game_session_id=p_source
 AND ag.presence_character_id=p_character AND ag.holder_game_session_id=p_source
 AND rg.ready AND rg.ownership_generation=p_source_generation
 AND pos.world_id=p_world AND pos.character_lease_generation=p_lease
 AND (pos.tile_x,pos.tile_y,pos.tile_floor)=(33039,32964,8)
 FOR SHARE OF s,r,pos,cg,ag,rg;
 IF NOT FOUND OR EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=p_destination) THEN
  RAISE EXCEPTION 'Summer portal requires actual source participant at its committed source anchor and fresh destination' USING ERRCODE='23514';
 END IF;
 PERFORM 1 FROM game_boss_room_catalogue catalogue WHERE catalogue.boss_key='oteryn:boss.rakesh_moonfang'
 AND catalogue.enabled AND catalogue.boss_creature_key='oteryn:creature.rakesh_moonfang'
 AND catalogue.minimum_players=1 AND catalogue.maximum_players=15 AND catalogue.minimum_level=1
 AND catalogue.cooldown_seconds=7200 AND catalogue.time_limit_seconds=600
 AND catalogue.room_template_key='oteryn:room.rakesh_moonfang_source_portal'
 AND catalogue.template_revision='source-portal-b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105' FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'Summer source catalogue changed' USING ERRCODE='23514';END IF;
 now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 IF EXISTS(SELECT 1 FROM game_activity_handoff_groups WHERE handoff_id=p_handoff) THEN
  phase:='join';
  PERFORM 1 FROM game_summer_portal_rooms room JOIN game_activity_handoff_groups g ON g.handoff_id=room.admission
  JOIN game_boss_room_activities a ON a.admission=g.handoff_id
  WHERE room.admission=p_handoff AND room.world_id=p_world AND room.instance_id=p_instance
  AND room.boss_key='oteryn:boss.rakesh_moonfang' AND room.group_kind=cohort.kind AND room.group_id=cohort.identity
  AND room.membership_sha256=cohort.membership AND room.source_channel_id=p_channel
  AND room.source_portal_key='oteryn:source-portal.rakesh_moonfang' AND room.source_map_digest=source_map
  AND g.state=2 AND g.ownership_generation=p_destination_generation AND a.state=1 AND a.deadline>now_ms
  AND a.boss_key=room.boss_key AND a.boss_creature_key='oteryn:creature.rakesh_moonfang'
  AND (SELECT count(*) FROM game_activity_handoff_participants WHERE handoff_id=g.handoff_id)<15
  AND NOT EXISTS(SELECT 1 FROM game_activity_handoff_participants WHERE handoff_id=g.handoff_id AND character_id=p_character)
  FOR UPDATE OF g,a;
  IF NOT FOUND THEN RAISE EXCEPTION 'Summer join requires the same live source World+Channel room' USING ERRCODE='23514';END IF;
 ELSE
  phase:='open';
  IF EXISTS(SELECT 1 FROM game_summer_portal_rooms room JOIN game_boss_room_activities a ON a.admission=room.admission
   WHERE room.world_id=p_world AND room.boss_key='oteryn:boss.rakesh_moonfang' AND room.group_kind=cohort.kind
   AND room.group_id=cohort.identity AND room.membership_sha256=cohort.membership AND room.source_channel_id=p_channel
   AND a.state=1 AND a.deadline>now_ms) THEN
   RAISE EXCEPTION 'Existing shared Summer source room must be joined, never duplicated' USING ERRCODE='23514';END IF;
 END IF;
 IF EXISTS(SELECT 1 FROM game_summer_portal_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Summer source transaction already armed' USING ERRCODE='23514';END IF;
 DELETE FROM game_summer_portal_context c WHERE c.backend_pid=pg_backend_pid() OR NOT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.pid=c.backend_pid);
 INSERT INTO game_summer_portal_context VALUES(pg_backend_pid(),pg_current_xact_id(),p_handoff,p_join,phase,p_world,p_channel,p_source_generation,
  p_instance,p_destination_generation,p_node,p_registration,p_character,p_source,p_destination,p_connection,p_lease,p_step,p_binding,
  cohort.kind,cohort.identity,cohort.membership,source_map);
END $$;

CREATE FUNCTION game_summer_portal_participant_current(p JSONB) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_summer_portal_context c
 WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id() AND c.phase='join'
 AND (p->>'handoff_id')::uuid=c.handoff_id AND (p->>'world_id')::uuid=c.world_id
 AND (p->>'character_id')::uuid=c.character_id AND (p->>'source_game_session_id')::uuid=c.source_session
 AND (p->>'destination_game_session_id')::uuid=c.destination_session AND (p->>'origin_channel_id')::uuid=c.channel_id
 AND (p->>'source_connection_generation')::numeric=c.source_connection AND (p->>'source_lease_generation')::numeric=c.source_lease
 AND (p->>'source_ownership_generation')::numeric=c.source_generation AND (p->>'entry_x')::int=33039
 AND (p->>'entry_y')::int=32964 AND (p->>'entry_floor')::int=8
 AND (p->>'map_revision_digest')::bytea=c.source_map_digest)
$$;

CREATE FUNCTION game_summer_portal_context_live() RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_summer_portal_context c
 JOIN game_runtime_scope_assignments source ON source.scope_key='\x01'::bytea||uuid_send(c.world_id)||uuid_send(c.channel_id)
 JOIN game_runtime_scope_assignments target ON target.scope_key='\x03'::bytea||uuid_send(c.world_id)||uuid_send(c.instance_id)
 WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
 AND source.state=1 AND source.ownership_generation=c.source_generation AND source.holder_node_id=c.node_id AND source.holder_registration_revision=c.registration
 AND target.state=1 AND target.ownership_generation=c.destination_generation AND target.holder_node_id=c.node_id AND target.holder_registration_revision=c.registration
 AND game_node_lock_current_registration(c.node_id,c.registration))
$$;

CREATE FUNCTION game_summer_portal_commit() RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_summer_portal_context%ROWTYPE;party RECORD;member game_activity_handoff_participants%ROWTYPE;now_ms BIGINT;
BEGIN
 SELECT * INTO c FROM game_summer_portal_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT FOUND OR NOT game_summer_portal_context_live() THEN RAISE EXCEPTION 'Summer portal commit requires private current Node context' USING ERRCODE='23514';END IF;
 SELECT * INTO party FROM game_summer_portal_group(c.world_id,c.character_id,c.channel_id);
 IF NOT FOUND OR (party.kind,party.identity,party.membership) IS DISTINCT FROM(c.group_kind,c.group_id,c.membership_sha256) THEN
  RAISE EXCEPTION 'Summer portal cohort changed before commit' USING ERRCODE='23514';END IF;
 SELECT p.* INTO member FROM game_activity_handoff_participants p
 JOIN game_activity_handoff_groups g USING(handoff_id)
 JOIN game_activity_runtime_placements q USING(handoff_id,character_id)
 JOIN game_activity_runtime_entries e USING(handoff_id)
 JOIN game_boss_room_activities a ON a.admission=g.handoff_id
 JOIN game_durability_reconnect_sessions source ON source.game_session_id=p.source_game_session_id
 JOIN game_durability_reconnect_sessions target ON target.game_session_id=p.destination_game_session_id
 WHERE p.handoff_id=c.handoff_id AND p.world_id=c.world_id AND p.character_id=c.character_id
 AND p.source_game_session_id=c.source_session AND p.destination_game_session_id=c.destination_session
 AND p.source_connection_generation=c.source_connection AND p.source_lease_generation=c.source_lease
 AND p.source_ownership_generation=c.source_generation AND p.origin_channel_id=c.channel_id
 AND (p.entry_x,p.entry_y,p.entry_floor)=(33039,32964,8) AND p.map_revision_digest=c.source_map_digest
 AND g.world_id=c.world_id AND g.instance_id=c.instance_id AND g.ownership_generation=c.destination_generation AND g.state=2
 AND g.activity_key='oteryn:boss.rakesh_moonfang' AND a.boss_key=g.activity_key AND a.state=1
 AND a.deadline>floor(extract(epoch FROM clock_timestamp())*1000)::bigint
 AND a.boss_creature_key='oteryn:creature.rakesh_moonfang'
 AND (q.destination_x,q.destination_y,q.destination_floor)=(33045,32988,-8)
 AND (q.exit_x,q.exit_y,q.exit_floor)=(33040,32962,8)
 AND source.session_state=3 AND source.current_transport_ref IS NULL AND source.prepared_attempt_ref IS NULL
 AND source.current_generation=c.source_connection AND source.character_lease_generation=c.source_lease
 AND target.character_id=c.character_id AND target.account_id=p.account_id AND target.world_id=c.world_id
 AND target.runtime_scope_kind=2 AND target.runtime_scope_world_id=c.world_id AND target.runtime_scope_instance_id=c.instance_id
 AND target.runtime_scope_channel_id IS NULL AND target.runtime_scope_house_key IS NULL AND target.origin_channel_id=c.channel_id
 AND target.scope_ownership_generation=c.destination_generation AND target.character_lease_generation=c.source_lease+1
 AND target.current_generation=1 AND target.activity_handoff_id=c.handoff_id AND target.session_state IN(1,2)
 FOR SHARE OF p,g,q,e,a,source,target;
 IF NOT FOUND THEN RAISE EXCEPTION 'Summer portal commit requires actual terminal source, successor and unchanged room' USING ERRCODE='23514';END IF;
 now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 IF c.phase='open' THEN
  IF NOT EXISTS(SELECT 1 FROM game_activity_runtime_entries WHERE handoff_id=c.handoff_id
   AND initiating_character_id=c.character_id AND initiating_game_session_id=c.source_session
   AND initiating_connection_generation=c.source_connection AND command_id=c.step_occurrence AND request_binding=c.request_sha256)
   OR (SELECT participant_count FROM game_activity_handoff_groups WHERE handoff_id=c.handoff_id)<>1 THEN
   RAISE EXCEPTION 'Summer initial room requires its one committed actual STEP batch' USING ERRCODE='23514';END IF;
  INSERT INTO game_summer_portal_rooms(admission,world_id,instance_id,boss_key,group_kind,group_id,membership_sha256,source_channel_id,source_portal_key,source_map_digest)
  VALUES(c.handoff_id,c.world_id,c.instance_id,'oteryn:boss.rakesh_moonfang',c.group_kind,c.group_id,c.membership_sha256,c.channel_id,'oteryn:source-portal.rakesh_moonfang',c.source_map_digest);
 ELSE
  INSERT INTO game_summer_portal_joins(join_id,admission,character_id,source_game_session_id,source_connection_generation,source_lease_generation,step_occurrence,request_sha256,joined_at)
  VALUES(c.join_id,c.handoff_id,c.character_id,c.source_session,c.source_connection,c.source_lease,c.step_occurrence,c.request_sha256,now_ms);
 END IF;
END $$;

CREATE FUNCTION game_summer_portal_room_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_summer_portal_context%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_summer_portal_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT FOUND OR c.phase IS DISTINCT FROM 'open' OR NOT game_summer_portal_context_live()
 OR (NEW.admission,NEW.world_id,NEW.instance_id,NEW.boss_key,NEW.group_kind,NEW.group_id,NEW.membership_sha256,NEW.source_channel_id,NEW.source_portal_key,NEW.source_map_digest)
 IS DISTINCT FROM(c.handoff_id,c.world_id,c.instance_id,'oteryn:boss.rakesh_moonfang'::text,c.group_kind,c.group_id,c.membership_sha256,c.channel_id,'oteryn:source-portal.rakesh_moonfang'::text,c.source_map_digest) THEN
  RAISE EXCEPTION 'Summer room requires its exact privately armed committed source' USING ERRCODE='23514';END IF;
 NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE FUNCTION game_summer_portal_join_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_summer_portal_context%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_summer_portal_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT FOUND OR c.phase IS DISTINCT FROM 'join' OR NOT game_summer_portal_context_live()
 OR (NEW.join_id,NEW.admission,NEW.character_id,NEW.source_game_session_id,NEW.source_connection_generation,NEW.source_lease_generation,NEW.step_occurrence,NEW.request_sha256)
 IS DISTINCT FROM(c.join_id,c.handoff_id,c.character_id,c.source_session,c.source_connection,c.source_lease,c.step_occurrence,c.request_sha256) THEN
  RAISE EXCEPTION 'Summer join requires its exact privately armed committed source' USING ERRCODE='23514';END IF;
 NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_summer_portal_room_admit BEFORE INSERT ON game_summer_portal_rooms FOR EACH ROW EXECUTE FUNCTION game_summer_portal_room_admit();
CREATE TRIGGER game_summer_portal_join_admit BEFORE INSERT ON game_summer_portal_joins FOR EACH ROW EXECUTE FUNCTION game_summer_portal_join_admit();
CREATE TRIGGER game_summer_portal_room_immutable BEFORE UPDATE OR DELETE ON game_summer_portal_rooms FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_summer_portal_join_immutable BEFORE UPDATE OR DELETE ON game_summer_portal_joins FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_summer_portal_room_no_truncate BEFORE TRUNCATE ON game_summer_portal_rooms FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE TRIGGER game_summer_portal_join_no_truncate BEFORE TRUNCATE ON game_summer_portal_joins FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();

-- Preserve every original snapshot/fresh-session check. Only a privately armed
-- exact current source join may extend a COMMITTED group's membership.
DO $source_participant$ DECLARE definition TEXT;old TEXT;replacement TEXT;BEGIN
 definition:=pg_get_functiondef('game_activity_handoff_participant_guard()'::regprocedure);
 old:='IF NOT FOUND OR v_group.state <> 1 THEN';
 replacement:='IF NOT FOUND OR (v_group.state <> 1 AND NOT (v_group.state=2 AND game_summer_portal_participant_current(to_jsonb(NEW)))) THEN';
 IF strpos(definition,old)=0 THEN RAISE EXCEPTION 'Summer participant state boundary changed';END IF;
 definition:=replace(definition,old,replacement);
 old:='IF v_count >= v_group.participant_count THEN';
 replacement:='IF v_count >= v_group.participant_count AND NOT game_summer_portal_participant_current(to_jsonb(NEW)) THEN';
 IF strpos(definition,old)=0 THEN RAISE EXCEPTION 'Summer participant count boundary changed';END IF;
 EXECUTE replace(definition,old,replacement);
END $source_participant$;

-- Historical expected count is initial batch + immutable, source-proven joins.
-- Initial participant_count is never rewritten and generic rooms have zero joins.
CREATE FUNCTION game_summer_portal_expected_members(p_handoff UUID,p_initial SMALLINT) RETURNS BIGINT LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT p_initial::bigint + CASE WHEN EXISTS(
  SELECT 1 FROM game_summer_portal_rooms room
  JOIN game_activity_handoff_groups g ON g.handoff_id=room.admission
  WHERE room.admission=p_handoff AND room.world_id=g.world_id AND room.instance_id=g.instance_id
  AND room.boss_key='oteryn:boss.rakesh_moonfang' AND g.activity_key=room.boss_key
  AND g.template_revision='source-portal-b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105'
  AND room.source_portal_key='oteryn:source-portal.rakesh_moonfang'
 ) THEN (SELECT count(*) FROM game_summer_portal_joins WHERE admission=p_handoff) ELSE 0 END
$$;
GRANT SELECT ON game_summer_portal_rooms,game_summer_portal_joins TO oteryn_game_runtime;
DO $$ DECLARE signature TEXT;BEGIN
 FOREACH signature IN ARRAY ARRAY[
 'game_summer_portal_arm(uuid,uuid,uuid,uuid,numeric,uuid,numeric,uuid,numeric,bytea,uuid,uuid,uuid,numeric,numeric,numeric,bytea)',
 'game_summer_portal_commit()','game_summer_portal_group(uuid,uuid,uuid)',
 'game_summer_portal_participant_current(jsonb)','game_summer_portal_context_live()',
 'game_summer_portal_room_admit()','game_summer_portal_join_admit()',
 'game_summer_portal_expected_members(uuid,smallint)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_control',signature);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_summer_portal_arm(uuid,uuid,uuid,uuid,numeric,uuid,numeric,uuid,numeric,bytea,uuid,uuid,uuid,numeric,numeric,numeric,bytea),
 game_summer_portal_commit(),game_summer_portal_expected_members(uuid,smallint) TO oteryn_game_runtime;

GRANT EXECUTE ON FUNCTION game_summer_portal_participant_current(jsonb) TO oteryn_game_runtime;
CREATE FUNCTION game_summer_portal_join_proven() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_summer_portal_context%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_summer_portal_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT FOUND OR NOT game_summer_portal_context_live() OR c.phase IS DISTINCT FROM 'join'
 OR (NEW.join_id,NEW.admission,NEW.character_id,NEW.source_game_session_id,NEW.source_connection_generation,NEW.source_lease_generation,NEW.step_occurrence,NEW.request_sha256,NEW.created_xact_id)
 IS DISTINCT FROM(c.join_id,c.handoff_id,c.character_id,c.source_session,c.source_connection,c.source_lease,c.step_occurrence,c.request_sha256,pg_current_xact_id())
 OR NOT EXISTS(SELECT 1 FROM game_activity_handoff_participants p
  JOIN game_activity_handoff_groups g USING(handoff_id)
  JOIN game_activity_runtime_placements q USING(handoff_id,character_id)
  JOIN game_durability_reconnect_sessions source ON source.game_session_id=p.source_game_session_id
  JOIN game_durability_reconnect_sessions target ON target.game_session_id=p.destination_game_session_id
  WHERE p.handoff_id=c.handoff_id AND p.character_id=c.character_id AND p.world_id=c.world_id
  AND p.source_game_session_id=c.source_session AND p.destination_game_session_id=c.destination_session
  AND p.origin_channel_id=c.channel_id AND p.source_connection_generation=c.source_connection AND p.source_lease_generation=c.source_lease
  AND p.source_ownership_generation=c.source_generation AND p.map_revision_digest=c.source_map_digest
  AND g.state=2 AND g.instance_id=c.instance_id AND g.ownership_generation=c.destination_generation
  AND (q.destination_x,q.destination_y,q.destination_floor)=(33045,32988,-8)
  AND (q.exit_x,q.exit_y,q.exit_floor)=(33040,32962,8)
  AND source.session_state=3 AND source.current_transport_ref IS NULL AND source.prepared_attempt_ref IS NULL
  AND target.character_id=c.character_id AND target.account_id=p.account_id AND target.world_id=c.world_id
  AND target.runtime_scope_kind=2 AND target.runtime_scope_world_id=c.world_id AND target.runtime_scope_instance_id=c.instance_id
  AND target.runtime_scope_channel_id IS NULL AND target.runtime_scope_house_key IS NULL AND target.origin_channel_id=c.channel_id
  AND target.scope_ownership_generation=c.destination_generation AND target.character_lease_generation=c.source_lease+1
  AND target.current_generation=1 AND target.activity_handoff_id=c.handoff_id)
 OR (SELECT count(*) FROM game_activity_handoff_participants WHERE handoff_id=c.handoff_id)
    <>game_summer_portal_expected_members(c.handoff_id,(SELECT participant_count FROM game_activity_handoff_groups WHERE handoff_id=c.handoff_id)) THEN
  RAISE EXCEPTION 'Summer join requires immutable reciprocal actual successor proof and exact membership count' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_summer_portal_join_proven AFTER INSERT ON game_summer_portal_joins
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_summer_portal_join_proven();
ALTER FUNCTION game_summer_portal_join_proven() SET search_path=pg_catalog,public,pg_temp;
REVOKE ALL ON FUNCTION game_summer_portal_join_proven() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_summer_portal_context_matches(p_kind SMALLINT,p_identity UUID,p_membership BYTEA) RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_summer_portal_context c WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
 AND c.group_kind=p_kind AND c.group_id=p_identity AND c.membership_sha256=p_membership)
$$;
ALTER FUNCTION game_summer_portal_context_matches(smallint,uuid,bytea) SET search_path=pg_catalog,public,pg_temp;
REVOKE ALL ON FUNCTION game_summer_portal_context_matches(smallint,uuid,bytea) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_summer_portal_context_matches(smallint,uuid,bytea) TO oteryn_game_runtime;

-- Reciprocal conservation: a mature committed Source parent/participant cannot
-- omit the private source child, even when a caller bypasses commit().
CREATE FUNCTION game_summer_portal_parent_proven() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE handoff UUID;g game_activity_handoff_groups%ROWTYPE;c game_summer_portal_context%ROWTYPE;
BEGIN
 handoff:=(to_jsonb(NEW)->>'handoff_id')::uuid;
 SELECT * INTO g FROM game_activity_handoff_groups WHERE handoff_id=handoff;
 IF NOT FOUND OR g.state<>2 OR g.activity_key<>'oteryn:boss.rakesh_moonfang' THEN RETURN NULL;END IF;
 SELECT * INTO c FROM game_summer_portal_context
 WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT FOUND OR NOT game_summer_portal_context_live()
 OR (c.handoff_id,c.world_id,c.instance_id,c.destination_generation)
 IS DISTINCT FROM(g.handoff_id,g.world_id,g.instance_id,g.ownership_generation)
 OR g.template_revision<>'source-portal-b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105'
 OR NOT EXISTS(SELECT 1 FROM game_summer_portal_rooms room
  WHERE room.admission=g.handoff_id AND room.world_id=g.world_id AND room.instance_id=g.instance_id
  AND room.boss_key=g.activity_key AND room.group_kind=c.group_kind AND room.group_id=c.group_id
  AND room.membership_sha256=c.membership_sha256 AND room.source_channel_id=c.channel_id
  AND room.source_portal_key='oteryn:source-portal.rakesh_moonfang' AND room.source_map_digest=c.source_map_digest
  AND (c.phase<>'open' OR room.created_xact_id=pg_current_xact_id())) THEN
  RAISE EXCEPTION 'Summer Source parent requires its same-XID private reciprocal room child' USING ERRCODE='23514';
 END IF;
 IF c.phase='open' THEN
  IF g.participant_count<>1 OR NOT EXISTS(SELECT 1 FROM game_activity_runtime_entries e
   WHERE e.handoff_id=g.handoff_id AND e.initiating_character_id=c.character_id
   AND e.initiating_game_session_id=c.source_session AND e.initiating_connection_generation=c.source_connection
   AND e.command_id=c.step_occurrence AND e.request_binding=c.request_sha256
   AND e.holder_node_id=c.node_id AND e.holder_registration_revision=c.registration) THEN
   RAISE EXCEPTION 'Summer Source parent requires its exact initial STEP entry' USING ERRCODE='23514';
  END IF;
 ELSIF TG_TABLE_NAME<>'game_activity_handoff_participants'
 OR (to_jsonb(NEW)->>'character_id')::uuid IS DISTINCT FROM c.character_id
 OR NOT EXISTS(SELECT 1 FROM game_summer_portal_joins j
  WHERE (j.join_id,j.admission,j.character_id,j.source_game_session_id,j.source_connection_generation,
   j.source_lease_generation,j.step_occurrence,j.request_sha256,j.created_xact_id)
  IS NOT DISTINCT FROM(c.join_id,c.handoff_id,c.character_id,c.source_session,c.source_connection,
   c.source_lease,c.step_occurrence,c.request_sha256,pg_current_xact_id())) THEN
  RAISE EXCEPTION 'Summer Source participant requires its same-XID private reciprocal join child' USING ERRCODE='23514';
 END IF;
 IF (SELECT count(*) FROM game_activity_handoff_participants WHERE handoff_id=g.handoff_id)
 <>game_summer_portal_expected_members(g.handoff_id,g.participant_count) THEN
  RAISE EXCEPTION 'Summer Source parent membership is incomplete' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_summer_portal_parent_proven
 AFTER INSERT OR UPDATE ON game_activity_handoff_groups DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_summer_portal_parent_proven();
CREATE CONSTRAINT TRIGGER game_summer_portal_entry_parent_proven
 AFTER INSERT ON game_activity_runtime_entries DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_summer_portal_parent_proven();
CREATE CONSTRAINT TRIGGER game_summer_portal_member_parent_proven
 AFTER INSERT ON game_activity_handoff_participants DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_summer_portal_parent_proven();
ALTER FUNCTION game_summer_portal_parent_proven() SET search_path=pg_catalog,public,pg_temp;
REVOKE ALL ON FUNCTION game_summer_portal_parent_proven() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

-- Preserve every existing retirement ownership/return proof. Only the qualified
-- Source room's immutable sequential joins extend its original batch count.
DO $source_retirement$ DECLARE definition TEXT;old TEXT;BEGIN
 definition:=pg_get_functiondef('game_activity_retirement_binding_valid(uuid,uuid,uuid,numeric,uuid,numeric)'::regprocedure);
 old:='g.participant_count=(SELECT count(*) FROM game_activity_handoff_participants p WHERE p.handoff_id=g.handoff_id)';
 IF strpos(definition,old)=0 THEN RAISE EXCEPTION 'Summer retirement count boundary changed';END IF;
 EXECUTE replace(definition,old,'game_summer_portal_expected_members(g.handoff_id,g.participant_count)=(SELECT count(*) FROM game_activity_handoff_participants p WHERE p.handoff_id=g.handoff_id)');
END $source_retirement$;
