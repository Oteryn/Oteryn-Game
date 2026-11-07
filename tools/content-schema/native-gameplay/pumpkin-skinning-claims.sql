-- HELD until actual native visit→static12816 USE_WITH RED.
CREATE TABLE game_pumpkin_skinning_receipts(
 transaction_id UUID PRIMARY KEY REFERENCES game_reward_claim_mint_receipts(transaction_id),character_id UUID NOT NULL,
 world_id UUID NOT NULL,death_id UUID NOT NULL,game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL,
 knife_id UUID NOT NULL,body_placement BYTEA NOT NULL CHECK(octet_length(body_placement)=40),body_revision NUMERIC(20,0) NOT NULL,
 map_digest BYTEA NOT NULL CHECK(octet_length(map_digest)=32),wire_binding BYTEA NOT NULL CHECK(octet_length(wire_binding)=32),
 used_at BIGINT NOT NULL CHECK(used_at>0),draw_index SMALLINT NOT NULL CHECK(draw_index BETWEEN 0 AND 12),
 source_item_id BIGINT NOT NULL,quantity BIGINT NOT NULL CHECK(quantity BETWEEN 1 AND 50),created_xact_id XID8 NOT NULL,
 UNIQUE(game_session_id,command_id),FOREIGN KEY(character_id,world_id,death_id) REFERENCES game_pumpkin_reward_visits(character_id,world_id,death_id));
CREATE INDEX game_pumpkin_skinning_personal_clock ON game_pumpkin_skinning_receipts(character_id,used_at DESC);
CREATE INDEX game_pumpkin_skinning_visit ON game_pumpkin_skinning_receipts(character_id,world_id,death_id);
CREATE FUNCTION game_pumpkin_skinning_draw(p_session UUID,p_nonce NUMERIC,p_source UUID,p_domain INTEGER,p_upper INTEGER)
RETURNS INTEGER LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE command_bytes BYTEA; digest BYTEA; value NUMERIC; zone NUMERIC; counter INTEGER; n INTEGER;
BEGIN
 IF p_nonce<>trunc(p_nonce) OR p_nonce NOT BETWEEN 0 AND 18446744073709551615 OR (p_domain,p_upper) NOT IN((213,13),(214,50)) THEN
  RAISE EXCEPTION 'invalid closed Pumpkin skinning RNG domain' USING ERRCODE='23514'; END IF;
 command_bytes:=int8send((CASE WHEN p_nonce>9223372036854775807 THEN p_nonce-18446744073709551616 ELSE p_nonce END)::bigint);
 zone:=18446744073709551615-mod(18446744073709551615,p_upper);
 FOR counter IN 0..15 LOOP
  digest:=sha256(convert_to('oteryn:reward-bag-draw:v1','UTF8')||uuid_send(p_session)||command_bytes
   ||uuid_send(p_source)||set_byte(decode('00','hex'),0,p_domain)||int4send(counter));
  value:=0;
  FOR n IN 0..7 LOOP value:=value*256+get_byte(digest,n); END LOOP;
  IF value<zone THEN RETURN mod(value,p_upper)::integer; END IF;
 END LOOP;
 RAISE EXCEPTION 'Pumpkin skinning RNG rejection resource bound' USING ERRCODE='23514';
END $$;
CREATE OR REPLACE FUNCTION game_pumpkin_source_arm(p_world UUID,p_channel UUID,p_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA,
 p_character UUID,p_session UUID,p_connection NUMERIC,p_lease NUMERIC,p_phase TEXT,p_death UUID,p_occurrence BYTEA,p_command NUMERIC,
 p_knife UUID,p_placement BYTEA,p_revision NUMERIC,p_map BYTEA,p_wire BYTEA)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE knife JSONB; now_ms BIGINT;
BEGIN
 IF p_secret IS NULL OR octet_length(p_secret)<>32 OR p_phase IS NULL OR p_phase NOT IN('visit','replay_visit','claim')
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
 ELSE
  IF p_command IS NULL OR p_command<1 OR p_command<>trunc(p_command) OR p_command>18446744073709551615
  OR p_knife IS NULL OR p_placement IS NULL OR octet_length(p_placement)<>40 OR p_revision IS NULL OR p_revision<0 OR p_revision>18446744073709551615 OR p_revision<>trunc(p_revision)
  OR p_map IS NULL OR octet_length(p_map)<>32 OR p_map=decode(repeat('00',32),'hex') OR p_wire IS NULL OR octet_length(p_wire)<>32
  OR p_occurrence IS NOT NULL THEN RAISE EXCEPTION 'invalid source-qualified12816 map body' USING ERRCODE='23514'; END IF;
  PERFORM 1 FROM game_pumpkin_reward_visits v WHERE v.character_id=p_character AND v.world_id=p_world AND v.death_id=p_death AND v.deadline_ms>now_ms FOR SHARE;
  IF NOT FOUND THEN RAISE EXCEPTION 'Halloween reward room visit expired or missing' USING ERRCODE='23514'; END IF;
  WITH RECURSIVE carried(id) AS (
   SELECT e.item_instance_id FROM game_item_container_entries e JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
   WHERE e.character_id=p_character AND e.world_id=p_world AND b.world_id=p_world
   UNION SELECT n.item_instance_id FROM game_item_nested_entries n JOIN carried c ON n.parent_item_instance_id=c.id WHERE n.world_id=p_world)
  SELECT to_jsonb(i) INTO knife FROM game_item_instances i WHERE i.item_instance_id=p_knife AND i.world_id=p_world AND i.lifecycle=1 AND i.quantity>0
   AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i5908' AND EXISTS(SELECT 1 FROM carried c WHERE c.id=i.item_instance_id) FOR UPDATE OF i;
  IF knife IS NULL THEN RAISE EXCEPTION 'Halloween skinning requires carried exact knife' USING ERRCODE='23514'; END IF;
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
CREATE FUNCTION game_pumpkin_visit_draw_nonce(p_occurrence BYTEA) RETURNS NUMERIC LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE n INTEGER; value NUMERIC:=0;
BEGIN
 IF octet_length(p_occurrence)<>32 THEN RAISE EXCEPTION 'invalid source visit occurrence' USING ERRCODE='23514'; END IF;
 FOR n IN 0..7 LOOP value:=value*256+get_byte(p_occurrence,n); END LOOP;RETURN value;
END $$;
CREATE FUNCTION game_pumpkin_skinning_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_pumpkin_source_context%ROWTYPE; ids BIGINT[]:=ARRAY[8032,8178,6491,6525,8177,6571,6570,6574,2977,3594,3599,6569,6574];
 counts BIGINT[]:=ARRAY[1,1,1,1,20,1,1,50,1,50,50,50,50]; idx INTEGER; chosen_count BIGINT; rng_session UUID; rng_occurrence BYTEA; nonce NUMERIC;
BEGIN
 SELECT * INTO c FROM game_pumpkin_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT game_pumpkin_skinning_current() OR c.phase IS DISTINCT FROM 'claim'
 OR(NEW.character_id,NEW.world_id,NEW.death_id,NEW.game_session_id,NEW.command_id,NEW.knife_id,NEW.body_placement,NEW.body_revision,NEW.map_digest,NEW.wire_binding)
 IS DISTINCT FROM(c.character_id,c.world_id,c.death_id,c.game_session_id,c.command_id,c.knife_id,c.body_placement,c.body_revision,c.map_digest,c.wire_binding) THEN
 RAISE EXCEPTION 'Halloween skinning lacks exact current static body and participant visit' USING ERRCODE='23514'; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:pumpkin-skinning:'||replace(NEW.character_id::text,'-',''),0));
 NEW.used_at:=floor(extract(epoch FROM public.game_daily_effect_now()))::bigint;
 IF EXISTS(SELECT 1 FROM game_pumpkin_skinning_receipts r WHERE r.character_id=NEW.character_id AND r.used_at>NEW.used_at-14400) THEN
 RAISE EXCEPTION 'Halloween skinning personal four-hour cooldown' USING ERRCODE='23514'; END IF;
 SELECT source_game_session_id,source_occurrence INTO rng_session,rng_occurrence FROM game_pumpkin_reward_visits
 WHERE(character_id,world_id,death_id)=(NEW.character_id,NEW.world_id,NEW.death_id);
 nonce:=game_pumpkin_visit_draw_nonce(rng_occurrence);
 idx:=game_pumpkin_skinning_draw(rng_session,nonce,NEW.death_id,213,13);
 chosen_count:=CASE WHEN ids[idx+1] IN(3594,6574) THEN game_pumpkin_skinning_draw(rng_session,nonce,NEW.death_id,214,50)+1 ELSE counts[idx+1] END;
 IF (NEW.draw_index,NEW.source_item_id,NEW.quantity) IS DISTINCT FROM(idx::smallint,ids[idx+1],chosen_count) THEN
 RAISE EXCEPTION 'Halloween skinning requires current quantity policy and exact uniform source entry' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
END $$;
CREATE TRIGGER game_pumpkin_skinning_admit BEFORE INSERT ON game_pumpkin_skinning_receipts FOR EACH ROW EXECUTE FUNCTION game_pumpkin_skinning_admit();
CREATE FUNCTION game_pumpkin_skinning_complete() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_pumpkin_source_context%ROWTYPE; m game_reward_claim_mint_receipts%ROWTYPE; current_knife JSONB;
BEGIN
 SELECT * INTO c FROM game_pumpkin_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 SELECT * INTO m FROM game_reward_claim_mint_receipts WHERE transaction_id=NEW.transaction_id;
 SELECT to_jsonb(i) INTO current_knife FROM game_item_instances i WHERE i.item_instance_id=NEW.knife_id;
 IF NOT game_pumpkin_skinning_current() OR c.phase IS DISTINCT FROM 'claim' OR NEW.created_xact_id<>pg_current_xact_id()
 OR (NEW.character_id,NEW.world_id,NEW.death_id,NEW.game_session_id,NEW.command_id,NEW.knife_id,NEW.body_placement,NEW.body_revision,NEW.map_digest,NEW.wire_binding)
 IS DISTINCT FROM(c.character_id,c.world_id,c.death_id,c.game_session_id,c.command_id,c.knife_id,c.body_placement,c.body_revision,c.map_digest,c.wire_binding)
 OR m.created_xact_id IS DISTINCT FROM pg_current_xact_id()
 OR (m.character_id,m.game_session_id,m.command_id,m.quantity,m.claim_family,m.claim_production_key,m.claim_revision_ref)
 IS DISTINCT FROM(NEW.character_id,NEW.game_session_id,NEW.command_id,NEW.quantity,'RewardClaim',
 'oteryn:reward-claim.pumpkin-skinning.'||replace(NEW.game_session_id::text,'-','')||':'||NEW.command_id::text,
 'pumpkin-skinning-wiki-c5fbb7ddf71779e795806b92038a209770324cb06e6dd610b5335708765ed742')
 OR m.contents_capacity IS NOT NULL OR m.contents_plan IS DISTINCT FROM '[]'::jsonb OR m.root_text IS NOT NULL
 OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=m.item_instance_id AND i.minted_transaction_id=m.transaction_id
 AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i'||NEW.source_item_id::text AND i.quantity=NEW.quantity AND i.lifecycle=1)
 OR NOT EXISTS(SELECT 1 FROM game_account_achievements a JOIN game_character_roots r ON r.account_id=a.account_id
 WHERE r.character_id=NEW.character_id AND a.achievement_key='oteryn:achievement/mutated_presents')
 OR NOT EXISTS(SELECT 1 FROM game_pumpkin_reward_visits v WHERE(v.character_id,v.world_id,v.death_id)=(NEW.character_id,NEW.world_id,NEW.death_id)
 AND v.deadline_ms>floor(extract(epoch FROM public.game_daily_effect_now())*1000)::bigint)
 OR current_knife IS DISTINCT FROM c.knife_before THEN
 RAISE EXCEPTION 'Halloween skinning requires same-XID native reward, live immutable visit and unchanged knife' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_pumpkin_skinning_complete AFTER INSERT ON game_pumpkin_skinning_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_pumpkin_skinning_complete();
CREATE FUNCTION game_pumpkin_skinning_mint_complete() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_pumpkin_skinning_receipts r WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
 RAISE EXCEPTION 'Halloween source MINT requires same-XID staticbody skinning receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_pumpkin_skinning_mint_complete AFTER INSERT ON game_reward_claim_mint_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
 WHEN(NEW.claim_production_key LIKE 'oteryn:reward-claim.pumpkin-skinning.%') EXECUTE FUNCTION game_pumpkin_skinning_mint_complete();
CREATE TRIGGER game_pumpkin_skinning_immutable BEFORE UPDATE OR DELETE ON game_pumpkin_skinning_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_pumpkin_skinning_no_truncate BEFORE TRUNCATE ON game_pumpkin_skinning_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_world_reset_history_immutable();
REVOKE ALL ON game_pumpkin_skinning_receipts,game_pumpkin_reward_visits FROM PUBLIC;
GRANT SELECT,INSERT ON game_pumpkin_skinning_receipts,game_pumpkin_reward_visits TO oteryn_game_runtime;
GRANT SELECT ON game_pumpkin_skinning_receipts,game_pumpkin_reward_visits TO oteryn_game_control;
DO $$ DECLARE signature TEXT; BEGIN
 FOREACH signature IN ARRAY ARRAY['game_pumpkin_skinning_current()',
 'game_pumpkin_source_arm(uuid,uuid,numeric,uuid,numeric,bytea,uuid,uuid,numeric,numeric,text,uuid,bytea,numeric,uuid,bytea,numeric,bytea,bytea)',
 'game_pumpkin_skinning_draw(uuid,numeric,uuid,integer,integer)','game_pumpkin_visit_draw_nonce(bytea)','game_pumpkin_visit_admit()',
 'game_pumpkin_skinning_admit()','game_pumpkin_skinning_complete()','game_pumpkin_skinning_mint_complete()'] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_pumpkin_source_arm(uuid,uuid,numeric,uuid,numeric,bytea,uuid,uuid,numeric,numeric,text,uuid,bytea,numeric,uuid,bytea,numeric,bytea,bytea) TO oteryn_game_runtime;
