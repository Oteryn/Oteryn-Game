-- T135: remote ownership selection also works while the actor is in a named
-- House. An origin Channel is never substituted for that House's assignment.
CREATE FUNCTION game_house_sale_scope_current(p JSONB,n UUID,nr NUMERIC) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w UUID:=(p->>'world')::uuid;ch UUID:=(p->>'channel')::uuid;
 inst UUID:=(p->>'instance')::uuid;
BEGIN
 IF inst IS NULL THEN
  RETURN ch IS NOT NULL AND EXISTS(SELECT 1 FROM game_runtime_scope_assignments ra
   JOIN game_durability_admission_runtime_guards rg USING(scope_key)
   WHERE ra.scope_key='\x01'::bytea||uuid_send(w)||uuid_send(ch) AND ra.scope_kind=1
    AND ra.world_id=w AND ra.channel_id=ch AND ra.state=1
    AND ra.ownership_generation=(p->>'generation')::numeric AND ra.holder_node_id=n
    AND ra.holder_registration_revision=nr AND rg.ready
    AND rg.ownership_generation=ra.ownership_generation FOR SHARE OF ra,rg);
 END IF;
 IF ch IS NOT NULL THEN RETURN FALSE; END IF;
 RETURN EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
  JOIN game_character_roots r USING(character_id)
  JOIN game_durability_admission_character_guards cg ON cg.character_id=s.character_id
  JOIN game_durability_admission_account_guards ag ON ag.account_id=s.account_id
  JOIN game_runtime_scope_assignments ra ON ra.scope_key=game_house_scope_key(w,s.runtime_scope_house_key)
  WHERE s.game_session_id=(p->>'session')::uuid AND s.character_id=(p->>'character')::uuid
   AND s.world_id=w AND s.runtime_scope_world_id=w AND s.runtime_scope_kind=2
   AND s.runtime_scope_channel_id IS NULL AND s.runtime_scope_house_key IS NOT NULL
   AND s.runtime_scope_instance_id=inst AND inst=game_house_scope_instance_id(w,s.runtime_scope_house_key)
   AND s.current_generation=(p->>'connection')::numeric AND s.character_lease_generation=(p->>'lease')::numeric
   AND s.scope_ownership_generation=(p->>'generation')::numeric AND s.session_state IN(1,2)
   AND s.session_state=2 AND s.current_transport_ref IS NOT NULL AND r.account_id=s.account_id AND r.world_id=w AND r.lifecycle=1
   AND cg.eligible AND cg.account_id=s.account_id AND cg.world_id=w
   AND cg.lease_generation=s.character_lease_generation AND cg.holder_game_session_id=s.game_session_id
   AND ag.presence_character_id=s.character_id AND ag.holder_game_session_id=s.game_session_id
   AND ra.scope_kind=2 AND ra.world_id=w AND ra.house_key=s.runtime_scope_house_key AND ra.state=1
   AND ra.ownership_generation=s.scope_ownership_generation AND ra.holder_node_id=n
   AND ra.holder_registration_revision=nr FOR SHARE OF s,r,cg,ag,ra);
END $$;
DO $patch$
DECLARE body TEXT;old TEXT;replacement TEXT;start_at INT;end_at INT;
BEGIN
 body:=pg_get_functiondef('game_house_sale_port(text,jsonb,uuid,numeric,bytea)'::regprocedure);
 body:=replace(body,$s$mode NOT IN('inspect','command')$s$,$s$mode NOT IN('inspect','command','fence')$s$);
 body:=replace(body,$s$IF action<>0 THEN$s$,$s$IF action<>0 AND mode<>'fence' THEN$s$);
 old:=$s$'after','premium_account','premium_allowed'$s$;
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing sale field boundary'; END IF;
 body:=replace(body,old,$s$'after','premium_account','premium_allowed','instance'$s$);
 start_at:=strpos(body,' IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments ra JOIN');
 end_at:=strpos(substr(body,start_at),'  OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions');
 IF start_at=0 OR end_at=0 THEN RAISE EXCEPTION 'missing sale runtime boundary'; END IF;
 body:=overlay(body placing E' IF NOT game_house_sale_scope_current(p,n,nr)\n' from start_at for end_at-1);
 old:=E'AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=w AND s.runtime_scope_channel_id=ch\n    AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL';
 replacement:=E'AND s.runtime_scope_world_id=w AND ((p->>''instance'' IS NULL AND s.runtime_scope_kind=1 AND s.runtime_scope_channel_id=ch AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL) OR (p->>''instance'' IS NOT NULL AND s.runtime_scope_kind=2 AND s.runtime_scope_channel_id IS NULL AND s.runtime_scope_instance_id=(p->>''instance'')::uuid))';
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing sale session boundary'; END IF;
 body:=replace(body,old,replacement);
 old:=$s$ IF action=0 THEN
  IF mode<>'inspect'$s$;
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing sale fence return boundary'; END IF;
 body:=replace(body,old,$s$ IF mode='fence' THEN RETURN 'true'::jsonb; END IF;
 IF action=0 THEN
  IF mode<>'inspect'$s$);
 EXECUTE body;
 body:=pg_get_functiondef('game_house_sale_audit_port(text,jsonb,uuid,numeric,bytea)'::regprocedure);
 start_at:=strpos(body,' IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments ra JOIN');
 end_at:=strpos(substr(body,start_at),' THEN RETURN NULL; END IF;');
 IF start_at=0 OR end_at=0 THEN RAISE EXCEPTION 'missing sale audit runtime boundary'; END IF;
 EXECUTE overlay(body placing ' IF NOT game_house_sale_scope_current(p,n,nr)' from start_at for end_at-1);
 -- The old private disposition port remains Channel-only for actor commands and
 -- World jobs. Only an audit for a new, independently fenced House command may
 -- use the House assignment; immutable same-XID disposition proofs still apply.
 body:=pg_get_functiondef('game_house_move_out_port(text,jsonb,uuid,numeric,bytea)'::regprocedure);
 old:=$s$'event','now','envelope'$s$;
 IF strpos(body,old)=0 THEN RAISE EXCEPTION 'missing disposition audit field boundary'; END IF;
 body:=replace(body,old,$s$'event','now','envelope','instance'$s$);
 start_at:=strpos(body,' IF NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments ra JOIN');
 end_at:=strpos(substr(body,start_at),' THEN RETURN NULL; END IF;');
 IF start_at=0 OR end_at=0 THEN RAISE EXCEPTION 'missing disposition runtime boundary'; END IF;
 replacement:=$s$ IF (p->>'instance' IS NOT NULL AND mode NOT IN('audit_snapshot','audit_insert')) OR NOT game_house_sale_scope_current(p,n,nr)$s$;
 EXECUTE overlay(body placing replacement from start_at for end_at-1);
 EXECUTE format('ALTER FUNCTION game_house_sale_scope_current(JSONB,UUID,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $patch$;
REVOKE ALL ON FUNCTION game_house_sale_scope_current(JSONB,UUID,NUMERIC)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
