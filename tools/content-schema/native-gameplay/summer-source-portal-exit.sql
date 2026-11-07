-- Server Source TP return is distinct from a fabricated client Exit command.
-- Merged main0989 reserves10/11 for signed staff terminal returns.
-- SourcePortalExit is12. Replace only the two named range/occurrence checks;
-- all existing death/idle/transport/loss checks and proof functions stay intact.
DO $source_return_shape$
DECLARE reason_attribute SMALLINT;command_attribute SMALLINT;
BEGIN
 SELECT attnum INTO STRICT reason_attribute FROM pg_attribute
 WHERE attrelid='game_activity_runtime_returns'::regclass AND attname='reason' AND NOT attisdropped;
 SELECT attnum INTO STRICT command_attribute FROM pg_attribute
 WHERE attrelid='game_activity_runtime_returns'::regclass AND attname='command_id' AND NOT attisdropped;
 IF NOT EXISTS(SELECT 1 FROM pg_constraint
  WHERE conrelid='game_activity_runtime_returns'::regclass AND conname='nx1_return_reason'
  AND contype='c' AND convalidated AND conkey=ARRAY[reason_attribute]::SMALLINT[])
 OR NOT EXISTS(SELECT 1 FROM pg_constraint
  WHERE conrelid='game_activity_runtime_returns'::regclass AND conname='nx1_return_server_command'
  AND contype='c' AND convalidated AND cardinality(conkey)=2
  AND conkey @> ARRAY[command_attribute,reason_attribute]::SMALLINT[])
 OR (SELECT count(*) FROM pg_constraint
  WHERE conrelid='game_activity_runtime_returns'::regclass AND contype='c' AND convalidated
  AND conname=ANY(ARRAY['nx1_return_death','nx1_return_death_command','nx1_return_idle',
   'nx1_return_transport','nx1_return_loss','nx1_return_active','nx1_return_offline']))<>7 THEN
 RAISE EXCEPTION 'SourcePortalExit requires exact merged staff return constraint identities';END IF;
 ALTER TABLE game_activity_runtime_returns
  DROP CONSTRAINT nx1_return_reason,
  DROP CONSTRAINT nx1_return_server_command,
  ADD CONSTRAINT nx1_return_reason CHECK(reason BETWEEN 1 AND 12),
  ADD CONSTRAINT nx1_return_server_command CHECK(command_id IS NOT NULL OR reason IN(2,3,4,5,6,7,8,9,10,11,12));
END $source_return_shape$;
CREATE TABLE game_summer_portal_exit_context(
 backend_pid INTEGER NOT NULL,transaction_xid XID8 NOT NULL,return_id UUID NOT NULL,
 handoff UUID NOT NULL,character_id UUID NOT NULL,source_session UUID NOT NULL,
 connection_generation NUMERIC NOT NULL,lease_generation NUMERIC NOT NULL,
 scope_generation NUMERIC NOT NULL,node_id UUID NOT NULL,registration NUMERIC NOT NULL,
 PRIMARY KEY(backend_pid,transaction_xid)
);
REVOKE ALL ON game_summer_portal_exit_context FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_summer_portal_exit_arm(p_node UUID,p_registration NUMERIC,p_secret BYTEA,p_handoff UUID,
 p_character UUID,p_session UUID,p_connection NUMERIC,p_lease NUMERIC,p_scope_generation NUMERIC,p_return UUID)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF game_node_prove_current_incarnation(p_node,p_registration,p_secret) IS DISTINCT FROM TRUE
 OR p_return IS NULL OR NOT game_node_is_uuid_v7(p_return) THEN
 RAISE EXCEPTION 'source TP return requires actual Node possession' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_summer_portal_rooms room JOIN game_activity_handoff_groups g ON g.handoff_id=room.admission
 JOIN game_activity_handoff_participants member ON member.handoff_id=g.handoff_id AND member.character_id=p_character
 JOIN game_durability_reconnect_sessions s ON s.game_session_id=member.destination_game_session_id
 JOIN game_character_roots root ON root.character_id=p_character AND root.world_id=g.world_id
 JOIN game_runtime_scope_assignments ownership ON ownership.scope_key='\x03'::bytea||uuid_send(g.world_id)||uuid_send(g.instance_id)
 JOIN game_durability_admission_character_guards cg ON cg.character_id=p_character
 JOIN game_durability_admission_account_guards ag ON ag.account_id=s.account_id
 JOIN game_durability_admission_runtime_guards rg ON rg.scope_key='\x02'::bytea||uuid_send(g.world_id)||uuid_send(g.instance_id)
 WHERE room.admission=p_handoff AND room.boss_key='oteryn:boss.rakesh_moonfang' AND g.activity_key=room.boss_key AND g.state=2
 AND room.source_portal_key='oteryn:source-portal.rakesh_moonfang' AND g.ownership_generation=p_scope_generation
 AND s.game_session_id=p_session AND s.character_id=p_character AND s.session_state IN(1,2)
 AND s.current_generation=p_connection AND s.character_lease_generation=p_lease AND s.current_transport_ref IS NOT NULL
 AND s.world_id=g.world_id AND s.runtime_scope_kind=2 AND s.runtime_scope_world_id=g.world_id AND s.runtime_scope_instance_id=g.instance_id
 AND s.runtime_scope_channel_id IS NULL AND s.runtime_scope_house_key IS NULL
 AND s.scope_ownership_generation=p_scope_generation AND root.lifecycle=1 AND root.account_id=s.account_id
 AND cg.world_id=g.world_id AND cg.account_id=s.account_id AND cg.eligible AND cg.lease_generation=p_lease AND cg.holder_game_session_id=p_session
 AND ag.presence_character_id=p_character AND ag.holder_game_session_id=p_session AND rg.ready AND rg.ownership_generation=p_scope_generation
 AND ownership.state=1 AND ownership.ownership_generation=p_scope_generation
 AND ownership.holder_node_id=p_node AND ownership.holder_registration_revision=p_registration
 FOR SHARE OF room,g,member,s,root,ownership,cg,ag,rg;
 IF NOT FOUND THEN RAISE EXCEPTION 'source TP return lacks its current exact activity participant/session/scope' USING ERRCODE='23514';END IF;
 IF EXISTS(SELECT 1 FROM game_summer_portal_exit_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id()) THEN
 RAISE EXCEPTION 'source return context already sealed' USING ERRCODE='23514';END IF;
 DELETE FROM game_summer_portal_exit_context c WHERE c.backend_pid=pg_backend_pid() OR NOT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.pid=c.backend_pid);
 INSERT INTO game_summer_portal_exit_context VALUES(pg_backend_pid(),pg_current_xact_id(),p_return,p_handoff,p_character,p_session,p_connection,p_lease,p_scope_generation,p_node,p_registration);
END $$;
CREATE FUNCTION game_summer_portal_exit_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.reason<>12 THEN RETURN NEW;END IF;
 IF NEW.command_id IS NOT NULL OR NEW.destination_transport_ref IS NULL OR NOT EXISTS(
  SELECT 1 FROM game_summer_portal_exit_context c JOIN game_node_registrations n ON n.node_id=c.node_id AND n.registration_revision=c.registration
  WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
  AND (c.return_id,c.handoff,c.character_id,c.source_session,c.connection_generation,c.lease_generation)
    IS NOT DISTINCT FROM(NEW.return_id,NEW.handoff_id,NEW.character_id,NEW.source_game_session_id,NEW.source_connection_generation,NEW.source_lease_generation)
  AND n.state=1 AND NOT EXISTS(SELECT 1 FROM game_node_registration_endings e WHERE e.node_id=c.node_id)) THEN
 RAISE EXCEPTION 'source TP return lacks its same-transaction native owner context' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_summer_portal_exit_admit BEFORE INSERT ON game_activity_runtime_returns FOR EACH ROW EXECUTE FUNCTION game_summer_portal_exit_admit();
DO $source_return_privileges$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_summer_portal_exit_arm(uuid,numeric,bytea,uuid,uuid,uuid,numeric,numeric,numeric,uuid)','game_summer_portal_exit_admit()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_control',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
 END LOOP;
END $source_return_privileges$;
