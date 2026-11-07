-- Halloween current rewardroom:8133death is distinct from static12816 rewardobject.
CREATE TABLE game_pumpkin_reward_visits(
 world_id UUID NOT NULL,death_id UUID NOT NULL,character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 first_entry_ms BIGINT NOT NULL CHECK(first_entry_ms>0),deadline_ms BIGINT NOT NULL CHECK(deadline_ms=first_entry_ms+300000),
 source_game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),source_occurrence BYTEA NOT NULL CHECK(octet_length(source_occurrence)=32),created_xact_id XID8 NOT NULL,
 PRIMARY KEY(character_id,world_id,death_id),UNIQUE(character_id,world_id,source_occurrence),FOREIGN KEY(world_id,death_id) REFERENCES game_boss_reward_deaths(world_id,death_id));
CREATE INDEX game_pumpkin_visits_source_session ON game_pumpkin_reward_visits(source_game_session_id);
CREATE INDEX game_pumpkin_visits_death ON game_pumpkin_reward_visits(world_id,death_id);
CREATE TABLE game_pumpkin_source_context(
 backend_pid INTEGER PRIMARY KEY,transaction_xid XID8 NOT NULL,world_id UUID NOT NULL,channel_id UUID NOT NULL,
 generation NUMERIC NOT NULL,character_id UUID NOT NULL,game_session_id UUID NOT NULL,connection NUMERIC NOT NULL,
 lease NUMERIC NOT NULL,phase TEXT NOT NULL CHECK(phase IN('visit','replay_visit','claim')),death_id UUID NOT NULL,
 occurrence BYTEA,command_id NUMERIC,knife_id UUID,body_placement BYTEA,body_revision NUMERIC,map_digest BYTEA,wire_binding BYTEA,
 knife_before JSONB);
REVOKE ALL ON game_pumpkin_source_context FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_pumpkin_skinning_current() RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_pumpkin_source_context c
 JOIN game_durability_reconnect_sessions s ON s.game_session_id=c.game_session_id
 JOIN game_character_roots r ON r.character_id=c.character_id
 JOIN game_durability_admission_character_guards cg ON cg.character_id=c.character_id
 JOIN game_durability_admission_account_guards ag ON ag.account_id=cg.account_id
 JOIN game_durability_admission_runtime_guards rg ON rg.scope_key='\x01'::bytea||uuid_send(c.world_id)||uuid_send(c.channel_id)
 WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
 AND s.character_id=c.character_id AND s.world_id=c.world_id AND s.runtime_scope_kind=1
 AND s.runtime_scope_world_id=c.world_id AND s.runtime_scope_channel_id=c.channel_id
 AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL
 AND s.current_generation=c.connection AND s.character_lease_generation=c.lease
 AND s.scope_ownership_generation=c.generation AND s.session_state IN(1,2)
 AND r.world_id=c.world_id AND r.account_id=s.account_id AND r.lifecycle=1
 AND cg.account_id=s.account_id AND cg.world_id=c.world_id AND cg.eligible
 AND cg.lease_generation=c.lease AND cg.holder_game_session_id=c.game_session_id
 AND ag.presence_character_id=c.character_id AND ag.holder_game_session_id=c.game_session_id
 AND rg.ready AND rg.ownership_generation=c.generation)
$$;
CREATE FUNCTION game_pumpkin_source_arm(p_world UUID,p_channel UUID,p_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA,
 p_character UUID,p_session UUID,p_connection NUMERIC,p_lease NUMERIC,p_phase TEXT,p_death UUID,p_occurrence BYTEA,p_command NUMERIC,
 p_knife UUID,p_placement BYTEA,p_revision NUMERIC,p_map BYTEA,p_wire BYTEA)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE knife JSONB; now_ms BIGINT;
BEGIN
 IF p_secret IS NULL OR octet_length(p_secret)<>32 OR p_phase IS NULL OR p_phase NOT IN('visit','replay_visit')
 OR NOT game_node_prove_current_incarnation(p_node,p_registration,p_secret) OR NOT game_runtime_scope_assignment_history_valid() THEN
 RAISE EXCEPTION 'Halloween source requires current Node possession' USING ERRCODE='23514'; END IF;
 PERFORM 1 FROM game_runtime_scope_assignments a WHERE a.scope_key='\x01'::bytea||uuid_send(p_world)||uuid_send(p_channel)
 AND a.scope_kind=1 AND a.world_id=p_world AND a.channel_id=p_channel AND a.state=1 AND a.ownership_generation=p_generation
 AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'stale Halloween Channel' USING ERRCODE='23514'; END IF;
 now_ms:=floor(extract(epoch FROM public.game_daily_effect_now())*1000)::bigint;
 PERFORM 1 FROM game_boss_reward_contributors bc JOIN game_boss_reward_deaths bd USING(world_id,death_id)
 WHERE bc.character_id=p_character AND bc.world_id=p_world AND bc.death_id=p_death
 AND bd.boss_key='oteryn:creature.the_mutated_pumpkin' AND bd.scope_kind=1 AND bd.settlement_outcome=1 AND bd.channel_id=p_channel
 AND bd.minted_at<=now_ms AND (bc.damage_dealt>0 OR bc.damage_received>0 OR bc.healing_others>0)
 AND (p_phase IN('claim','replay_visit') OR bd.minted_at+1800000>now_ms) FOR SHARE OF bc,bd;
 IF NOT FOUND THEN RAISE EXCEPTION 'Halloween source requires actual fight participation and live entry window' USING ERRCODE='23514'; END IF;
 IF p_phase IN('visit','replay_visit') THEN
  IF p_occurrence IS NULL OR octet_length(p_occurrence)<>32 OR p_command IS NOT NULL OR p_knife IS NOT NULL OR p_placement IS NOT NULL
  OR p_revision IS NOT NULL OR p_map IS NOT NULL OR p_wire IS NOT NULL THEN RAISE EXCEPTION 'invalid committed portal STEP occurrence' USING ERRCODE='23514'; END IF;
  IF p_phase='replay_visit' AND NOT EXISTS(SELECT 1 FROM game_pumpkin_reward_visits v WHERE v.character_id=p_character AND v.world_id=p_world
   AND v.death_id=p_death AND v.source_occurrence=p_occurrence AND v.deadline_ms>now_ms) THEN
   RAISE EXCEPTION 'Halloween entry replay requires its exact live committed first occurrence' USING ERRCODE='23514'; END IF;
 END IF;
 IF EXISTS(SELECT 1 FROM game_pumpkin_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id()) THEN
 RAISE EXCEPTION 'Halloween source transaction already armed' USING ERRCODE='23514'; END IF;
 DELETE FROM game_pumpkin_source_context c WHERE c.backend_pid=pg_backend_pid() OR NOT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.pid=c.backend_pid);
 INSERT INTO game_pumpkin_source_context VALUES(pg_backend_pid(),pg_current_xact_id(),p_world,p_channel,p_generation,p_character,p_session,p_connection,p_lease,p_phase,p_death,p_occurrence,p_command,p_knife,p_placement,p_revision,p_map,p_wire,knife);
 PERFORM 1 FROM game_durability_reconnect_sessions s JOIN game_durability_admission_character_guards cg ON cg.character_id=p_character
 JOIN game_durability_admission_account_guards ag ON ag.account_id=cg.account_id
 JOIN game_durability_admission_runtime_guards rg ON rg.scope_key='\x01'::bytea||uuid_send(p_world)||uuid_send(p_channel)
 WHERE s.game_session_id=p_session AND s.character_id=p_character FOR SHARE OF s,cg,ag,rg;
 IF NOT FOUND OR NOT game_pumpkin_skinning_current() THEN RAISE EXCEPTION 'stale Halloween Character/session/lease/scope fence' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_pumpkin_visit_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_pumpkin_source_context%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_pumpkin_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT game_pumpkin_skinning_current() OR c.phase IS DISTINCT FROM 'visit'
 OR (NEW.world_id,NEW.death_id,NEW.character_id,NEW.source_game_session_id,NEW.source_occurrence)
 IS DISTINCT FROM(c.world_id,c.death_id,c.character_id,c.game_session_id,c.occurrence) THEN
 RAISE EXCEPTION 'Halloween visit requires committed native portal STEP and current participant' USING ERRCODE='23514'; END IF;
 NEW.first_entry_ms:=floor(extract(epoch FROM public.game_daily_effect_now())*1000)::bigint; NEW.deadline_ms:=NEW.first_entry_ms+300000;
 NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
END $$;
CREATE TRIGGER game_pumpkin_visit_admit BEFORE INSERT ON game_pumpkin_reward_visits FOR EACH ROW EXECUTE FUNCTION game_pumpkin_visit_admit();
CREATE TRIGGER game_pumpkin_visit_immutable BEFORE UPDATE OR DELETE ON game_pumpkin_reward_visits FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_pumpkin_visit_no_truncate BEFORE TRUNCATE ON game_pumpkin_reward_visits FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();
REVOKE ALL ON game_pumpkin_reward_visits FROM PUBLIC;
GRANT SELECT,INSERT ON game_pumpkin_reward_visits TO oteryn_game_runtime;
GRANT SELECT ON game_pumpkin_reward_visits TO oteryn_game_control;
DO $$ DECLARE signature TEXT; BEGIN
 FOREACH signature IN ARRAY ARRAY['game_pumpkin_skinning_current()',
 'game_pumpkin_source_arm(uuid,uuid,numeric,uuid,numeric,bytea,uuid,uuid,numeric,numeric,text,uuid,bytea,numeric,uuid,bytea,numeric,bytea,bytea)',
 'game_pumpkin_visit_admit()'] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_pumpkin_source_arm(uuid,uuid,numeric,uuid,numeric,bytea,uuid,uuid,numeric,numeric,text,uuid,bytea,numeric,uuid,bytea,numeric,bytea,bytea) TO oteryn_game_runtime;
