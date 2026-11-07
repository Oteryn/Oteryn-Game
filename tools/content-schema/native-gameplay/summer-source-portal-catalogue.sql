-- Only a possession-proven current source Node may publish this one immutable
-- source policy. This changes no actor, membership, cooldown, quest or reward.
CREATE FUNCTION game_summer_portal_publish(p_world UUID,p_channel UUID,p_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA,
 p_character UUID,p_session UUID,p_connection NUMERIC,p_lease NUMERIC,p_content TEXT,p_server BYTEA,p_client BYTEA,p_frame BYTEA)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE existing game_boss_room_catalogue%ROWTYPE;
BEGIN
 IF p_secret IS NULL OR octet_length(p_secret)<>32 OR NOT game_node_prove_current_incarnation(p_node,p_registration,p_secret)
 OR NOT game_runtime_scope_assignment_history_valid() OR p_content IS NULL OR p_content='' OR octet_length(p_content)>128 THEN
  RAISE EXCEPTION 'Summer source publication requires current Node and content' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards rg USING(scope_key)
 JOIN game_durability_reconnect_sessions s ON s.game_session_id=p_session
 JOIN game_character_roots r ON r.character_id=s.character_id
 JOIN game_durability_admission_character_guards cg ON cg.character_id=r.character_id
 JOIN game_durability_admission_account_guards ag ON ag.account_id=r.account_id
 WHERE a.scope_key='\x01'::bytea||uuid_send(p_world)||uuid_send(p_channel) AND a.state=1 AND a.ownership_generation=p_generation
 AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration AND rg.ready AND rg.ownership_generation=p_generation
 AND s.character_id=p_character AND s.world_id=p_world AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=p_world
 AND s.runtime_scope_channel_id=p_channel AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL
 AND s.session_state IN(1,2) AND s.current_generation=p_connection AND s.character_lease_generation=p_lease AND s.scope_ownership_generation=p_generation
 AND r.world_id=p_world AND r.lifecycle=1 AND r.content_revision=p_content AND r.account_id=s.account_id
 AND cg.eligible AND cg.world_id=p_world AND cg.account_id=r.account_id AND cg.lease_generation=p_lease AND cg.holder_game_session_id=p_session
 AND ag.presence_character_id=p_character AND ag.holder_game_session_id=p_session FOR SHARE OF a,rg,s,r,cg,ag;
 IF NOT FOUND THEN RAISE EXCEPTION 'Summer publication requires current actual source actor' USING ERRCODE='23514';END IF;
 PERFORM 1 FROM game_content_activations activation WHERE activation.world_id=p_world AND activation.channel_id=p_channel
 AND activation.server_artifact_digest=p_server AND activation.client_artifact_digest=p_client AND activation.frame_binding_digest=p_frame
 AND NOT EXISTS(SELECT 1 FROM game_content_activations newer WHERE newer.world_id=p_world AND newer.channel_id=p_channel
  AND newer.activation_sequence>activation.activation_sequence) FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'Summer publication requires the current source activation' USING ERRCODE='23514';END IF;
 SELECT * INTO existing FROM game_boss_room_catalogue WHERE boss_key='oteryn:boss.rakesh_moonfang' FOR UPDATE;
 IF FOUND THEN
  IF (existing.boss_creature_key,existing.minimum_players,existing.maximum_players,existing.minimum_level,existing.cooldown_seconds,existing.time_limit_seconds,
      existing.lever_positions,existing.room_template_key,existing.reward_key,existing.enabled,existing.template_revision,existing.entry_positions,existing.exit_position,existing.boss_position,existing.after_kill_seconds)
    IS DISTINCT FROM('oteryn:creature.rakesh_moonfang'::text,1::smallint,15::smallint,1::bigint,7200::bigint,600::bigint,
      '[{"x":33039,"y":32964,"floor":8}]'::jsonb,'oteryn:room.rakesh_moonfang_source_portal'::text,'oteryn:reward.rakesh_moonfang'::text,true,
      'source-portal-b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105'::text,
      '[{"x":33045,"y":32988,"floor":-8}]'::jsonb,'{"x":33040,"y":32962,"floor":8}'::jsonb,'{"x":33042,"y":32993,"floor":-8}'::jsonb,0::bigint) THEN
   RAISE EXCEPTION 'Existing Rakesh policy conflicts with captured source' USING ERRCODE='23514';END IF;
  IF existing.content_revision<>p_content THEN UPDATE game_boss_room_catalogue SET content_revision=p_content WHERE boss_key=existing.boss_key;END IF;
 ELSE
  INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,
   lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds)
  VALUES('oteryn:boss.rakesh_moonfang',p_content,'oteryn:creature.rakesh_moonfang',1,15,1,7200,600,'[{"x":33039,"y":32964,"floor":8}]',
   'oteryn:room.rakesh_moonfang_source_portal','oteryn:reward.rakesh_moonfang',true,
   'source-portal-b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105',
   '[{"x":33045,"y":32988,"floor":-8}]','{"x":33040,"y":32962,"floor":8}','{"x":33042,"y":32993,"floor":-8}',0);
 END IF;
END $$;
ALTER FUNCTION game_summer_portal_publish(uuid,uuid,numeric,uuid,numeric,bytea,uuid,uuid,numeric,numeric,text,bytea,bytea,bytea) SET search_path=pg_catalog,public,pg_temp;
REVOKE ALL ON FUNCTION game_summer_portal_publish(uuid,uuid,numeric,uuid,numeric,bytea,uuid,uuid,numeric,numeric,text,bytea,bytea,bytea) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_summer_portal_publish(uuid,uuid,numeric,uuid,numeric,bytea,uuid,uuid,numeric,numeric,text,bytea,bytea,bytea) TO oteryn_game_runtime;
