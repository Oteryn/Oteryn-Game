CREATE FUNCTION game_staff_world_effect_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_staff_operations; l game_staff_control_links;
BEGIN
IF TG_OP='UPDATE' THEN
IF ROW(NEW.environment,NEW.operation_id,NEW.target_character_id,NEW.issuer_character_id,NEW.target_game_session_id,NEW.target_connection_generation,NEW.target_scope_ownership_generation,NEW.action)
IS DISTINCT FROM ROW(OLD.environment,OLD.operation_id,OLD.target_character_id,OLD.issuer_character_id,OLD.target_game_session_id,OLD.target_connection_generation,OLD.target_scope_ownership_generation,OLD.action)
OR OLD.applied OR NOT NEW.applied OR NEW.success IS NULL THEN
RAISE EXCEPTION 'world effect acknowledgement immutable' USING ERRCODE='23514';
END IF;
RETURN NEW;
END IF;
SELECT * INTO STRICT o FROM game_staff_operations WHERE environment=NEW.environment AND operation_id=NEW.operation_id;
SELECT * INTO STRICT l FROM game_staff_control_links WHERE environment=o.environment AND control_seq=o.control_seq;
IF o.created_xact_id<>pg_current_xact_id() OR o.result<>1
OR o.kind NOT IN ('TELEPORT','TELEPORT_TO_PLAYER','BRING','NOTIFY','BROADCAST','GHOST','INVISIBLE')
OR o.target_character_id<>NEW.target_character_id OR o.issuer_character_id IS DISTINCT FROM NEW.issuer_character_id
OR convert_from(l.canonical,'UTF8')::jsonb->'action' IS DISTINCT FROM NEW.action
OR NEW.applied OR NEW.success IS NOT NULL
OR NEW.target_game_session_id IS NULL OR NEW.target_connection_generation IS NULL OR NEW.target_scope_ownership_generation IS NULL
OR NOT EXISTS(
SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.game_session_id=NEW.target_game_session_id
AND s.character_id=NEW.target_character_id AND s.session_state=2
AND s.world_id=o.world_id AND s.runtime_scope_world_id=o.world_id AND s.runtime_scope_channel_id=o.channel_id
AND s.current_generation=NEW.target_connection_generation
AND s.scope_ownership_generation=NEW.target_scope_ownership_generation) THEN
RAISE EXCEPTION 'world effect lacks atomic signed operation' USING ERRCODE='23514';
END IF;
RETURN NEW;
END; $$;
CREATE TRIGGER game_staff_world_effect_bound BEFORE INSERT OR UPDATE ON game_staff_world_effects FOR EACH ROW EXECUTE FUNCTION game_staff_world_effect_bound();
CREATE TRIGGER game_staff_world_effect_no_delete BEFORE DELETE ON game_staff_world_effects FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE OR REPLACE FUNCTION game_staff_operation_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE l game_staff_control_links; o game_staff_operations; h game_staff_control_heads;
BEGIN
SELECT * INTO STRICT o FROM game_staff_operations WHERE environment=NEW.environment AND operation_id=NEW.operation_id;
SELECT * INTO STRICT l FROM game_staff_control_links WHERE environment=o.environment AND control_seq=o.control_seq;
SELECT * INTO STRICT h FROM game_staff_control_heads WHERE environment=o.environment;
IF o.created_xact_id<>pg_current_xact_id() OR l.created_xact_id<>pg_current_xact_id() OR l.kind<>'REQUEST' OR l.operation_id<>o.operation_id
OR (convert_from(l.canonical,'UTF8')::jsonb->'action'->>'kind') IS DISTINCT FROM o.kind
OR replace(o.target_character_id::text,'-','') IS DISTINCT FROM (convert_from(l.canonical,'UTF8')::jsonb->'action'->>'character_id')
OR (convert_from(l.canonical,'UTF8')::jsonb ? 'issuer_character_id' AND replace(o.issuer_character_id::text,'-','') IS DISTINCT FROM convert_from(l.canonical,'UTF8')::jsonb->>'issuer_character_id')
OR NOT EXISTS(SELECT 1 FROM game_staff_action_outbox a WHERE a.environment=o.environment AND a.operation_id=o.operation_id AND a.created_xact_id=pg_current_xact_id() AND a.starts_at_ms=o.starts_at_ms)
OR encode(sha256(o.outcome),'hex')<>o.outcome_sha256
OR h.control_seq<>o.control_seq OR (o.requires_receipt AND h.pending_operation_id IS DISTINCT FROM o.operation_id)
OR (NOT o.requires_receipt AND h.pending_operation_id IS NOT NULL) THEN
RAISE EXCEPTION 'staff operation missing atomic evidence' USING ERRCODE='23514';
END IF;
IF o.result=1 AND o.kind='MUTE' AND NOT EXISTS(SELECT 1 FROM game_character_sanctions s WHERE s.environment=o.environment AND s.sanction_id=o.operation_id) THEN
RAISE EXCEPTION 'mute has no effect' USING ERRCODE='23514';
END IF;
IF o.result=1 AND o.kind='UNMUTE' AND NOT EXISTS(SELECT 1 FROM game_character_sanctions s WHERE s.environment=o.environment AND s.lift_operation_id=o.operation_id) THEN
RAISE EXCEPTION 'unmute has no effect' USING ERRCODE='23514';
END IF;
IF o.result=1 AND o.kind='KICK' AND NOT EXISTS(SELECT 1 FROM game_staff_kicks k WHERE k.environment=o.environment AND k.operation_id=o.operation_id) THEN
RAISE EXCEPTION 'kick has no effect' USING ERRCODE='23514';
END IF;
IF o.result=1 AND o.kind='BAN' AND NOT EXISTS(SELECT 1 FROM game_staff_bans b WHERE b.environment=o.environment AND b.sanction_id=o.operation_id) THEN
RAISE EXCEPTION 'ban has no effect' USING ERRCODE='23514';
END IF;
IF o.result=1 AND o.kind='UNBAN' AND NOT EXISTS(SELECT 1 FROM game_staff_bans b WHERE b.environment=o.environment AND b.lift_operation_id=o.operation_id) THEN
RAISE EXCEPTION 'unban has no effect' USING ERRCODE='23514';
END IF;
IF o.result=1 AND o.kind IN ('TELEPORT','TELEPORT_TO_PLAYER','BRING','NOTIFY','BROADCAST','GHOST','INVISIBLE') AND NOT EXISTS(
SELECT 1 FROM game_staff_world_effects e WHERE e.environment=o.environment AND e.operation_id=o.operation_id) THEN
RAISE EXCEPTION 'world operation has no captured effect' USING ERRCODE='23514';
END IF;
RETURN NULL;
END; $$;
DO $$ DECLARE c record; BEGIN
FOR c IN SELECT conname FROM pg_constraint
WHERE conrelid='game_activity_runtime_returns'::regclass AND contype='c'
AND pg_get_constraintdef(oid) ~ '\mreason\M'
LOOP EXECUTE format('ALTER TABLE game_activity_runtime_returns DROP CONSTRAINT %I',c.conname); END LOOP;
END; $$;
ALTER TABLE game_activity_runtime_returns
ADD CONSTRAINT nx1_return_reason CHECK (reason BETWEEN 1 AND 11),
ADD CONSTRAINT nx1_return_server_command CHECK (command_id IS NOT NULL OR reason IN (2,3,4,5,6,7,8,9,10,11)),
ADD CONSTRAINT nx1_return_death CHECK ((reason=5)=(death_occurrence_id IS NOT NULL)),
ADD CONSTRAINT nx1_return_death_command CHECK (reason<>5 OR command_id IS NULL),
ADD CONSTRAINT nx1_return_idle CHECK (reason<>7 OR (command_id IS NULL AND destination_transport_ref IS NOT NULL)),
ADD CONSTRAINT nx1_return_transport CHECK ((reason IN (8,10))=(release_transport_ref IS NOT NULL)),
ADD CONSTRAINT nx1_return_loss CHECK ((reason IN (9,11))=(release_loss_epoch IS NOT NULL)),
ADD CONSTRAINT nx1_return_active CHECK (reason NOT IN (8,10) OR (command_id IS NULL AND destination_transport_ref IS NOT NULL)),
ADD CONSTRAINT nx1_return_offline CHECK (reason NOT IN (9,11) OR (command_id IS NULL AND destination_transport_ref IS NULL));
CREATE OR REPLACE FUNCTION game_activity_runtime_return_proven() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$ BEGIN
PERFORM 1 FROM game_activity_handoff_participants p
JOIN game_activity_handoff_groups g ON g.handoff_id = p.handoff_id
JOIN game_activity_runtime_placements q ON (q.handoff_id,q.character_id)=(p.handoff_id,p.character_id)
JOIN game_durability_reconnect_sessions s ON s.game_session_id = NEW.source_game_session_id
JOIN game_durability_reconnect_sessions d ON d.game_session_id = NEW.destination_game_session_id
WHERE p.handoff_id = NEW.handoff_id AND p.character_id = NEW.character_id AND g.state = 2
AND s.game_session_id = p.destination_game_session_id
AND s.character_id = p.character_id AND s.account_id = p.account_id
AND s.runtime_scope_kind = 2 AND s.runtime_scope_instance_id = g.instance_id
AND s.world_id = g.world_id AND s.runtime_scope_world_id = g.world_id
AND s.scope_ownership_generation = g.ownership_generation
AND s.current_generation = NEW.source_connection_generation
AND s.character_lease_generation = NEW.source_lease_generation
AND s.session_state = 3 AND s.current_transport_ref IS NULL AND s.prepared_attempt_ref IS NULL
AND d.character_id = p.character_id AND d.account_id = p.account_id
AND d.world_id = p.world_id AND d.runtime_scope_world_id = p.world_id
AND d.runtime_scope_kind = 1 AND d.runtime_scope_channel_id = p.origin_channel_id
AND d.runtime_scope_instance_id IS NULL AND d.runtime_scope_house_key IS NULL AND d.origin_channel_id IS NULL
AND d.scope_ownership_generation = NEW.destination_ownership_generation
AND d.character_lease_generation = NEW.source_lease_generation + 1 AND d.current_generation = 1
AND (NEW.return_coordinate_frame_digest,NEW.return_map_revision_digest,NEW.return_server_artifact_digest)
=(q.exit_coordinate_frame_digest,q.exit_map_revision_digest,q.exit_server_artifact_digest)
AND (NEW.reason=5 OR (NEW.return_x,NEW.return_y,NEW.return_floor)=(q.exit_x,q.exit_y,q.exit_floor))
FOR SHARE OF p, g, q, s, d;
IF NOT FOUND THEN
RAISE EXCEPTION 'activity return requires its terminal source and fresh origin destination' USING ERRCODE = '23514';
END IF;
IF NEW.reason=7 THEN
PERFORM 1 FROM game_durability_reconnect_sessions d
WHERE d.game_session_id=NEW.destination_game_session_id AND d.session_state=2
AND d.current_transport_ref=NEW.destination_transport_ref
AND d.control_loss_epoch IS NULL AND d.original_grace_deadline IS NULL
FOR SHARE OF d;
IF NOT FOUND THEN
RAISE EXCEPTION 'idle activity logout requires its active fresh destination' USING ERRCODE='23514';
END IF;
END IF;
IF NEW.reason IN (8,10) THEN
PERFORM 1 FROM game_durability_transport_ref_reservations t
JOIN game_durability_reconnect_sessions d ON d.game_session_id=NEW.destination_game_session_id
WHERE t.transport_ref=NEW.release_transport_ref AND t.game_session_id=NEW.source_game_session_id
AND d.session_state=2 AND d.current_transport_ref=NEW.destination_transport_ref
AND d.control_loss_epoch IS NULL AND d.original_grace_deadline IS NULL
FOR SHARE OF t,d;
IF NOT FOUND THEN
RAISE EXCEPTION 'abandoned activity return requires its exact issued transport and active fresh destination' USING ERRCODE='23514';
END IF;
END IF;
IF NEW.reason=10 THEN
PERFORM 1 FROM game_durability_reconnect_sessions s
WHERE s.game_session_id=NEW.source_game_session_id AND (
EXISTS (SELECT 1 FROM game_staff_disconnects k WHERE k.game_session_id=s.game_session_id)
OR EXISTS (SELECT 1 FROM game_staff_bans b WHERE b.account_id=s.account_id
AND b.lifted_at_ms IS NULL AND b.starts_at_ms<=NEW.returned_at AND b.ends_at_ms>NEW.returned_at))
FOR SHARE OF s;
IF NOT FOUND THEN
RAISE EXCEPTION 'staff activity return requires its independent signed decision' USING ERRCODE='23514';
END IF;
END IF;
IF NEW.reason=9 THEN
PERFORM 1 FROM game_durability_reconnect_sessions s
JOIN game_durability_reconnect_sessions d ON d.game_session_id=NEW.destination_game_session_id
JOIN game_durability_admission_lifecycle_receipts l
ON l.operation_key=convert_to('owning-loss-v1','UTF8')||uuid_send(s.game_session_id)
||decode(lpad(to_hex(trunc(NEW.release_loss_epoch/4294967296)::bigint),8,'0')
||lpad(to_hex(mod(NEW.release_loss_epoch,4294967296)::bigint),8,'0'),'hex')
WHERE s.game_session_id=NEW.source_game_session_id AND s.control_loss_epoch=NEW.release_loss_epoch
AND s.original_grace_deadline>NEW.returned_at/1000
AND d.session_state=1 AND d.current_transport_ref IS NULL
AND d.original_grace_deadline=s.original_grace_deadline AND d.control_loss_epoch=1
FOR SHARE OF s,d,l;
IF NOT FOUND THEN
RAISE EXCEPTION 'activity capability mismatch requires its recorded epoch and inherited grace' USING ERRCODE='23514';
END IF;
END IF;
IF NEW.reason=11 THEN
PERFORM 1 FROM game_durability_reconnect_sessions s
JOIN game_durability_reconnect_sessions d ON d.game_session_id=NEW.destination_game_session_id
JOIN game_durability_admission_lifecycle_receipts l
ON l.operation_key=convert_to('owning-loss-v1','UTF8')||uuid_send(s.game_session_id)
||decode(lpad(to_hex(trunc(NEW.release_loss_epoch/4294967296)::bigint),8,'0')
||lpad(to_hex(mod(NEW.release_loss_epoch,4294967296)::bigint),8,'0'),'hex')
WHERE s.game_session_id=NEW.source_game_session_id AND s.control_loss_epoch=NEW.release_loss_epoch
AND d.session_state=1 AND d.current_transport_ref IS NULL
AND d.original_grace_deadline=s.original_grace_deadline AND d.control_loss_epoch=1
AND EXISTS(SELECT 1 FROM game_staff_disconnects k WHERE k.game_session_id=s.game_session_id)
FOR SHARE OF s,d,l;
IF NOT FOUND THEN
RAISE EXCEPTION 'staff activity loss return requires its signed disconnect, recorded epoch and inherited grace' USING ERRCODE='23514';
END IF;
END IF;
IF NEW.reason=6 AND NEW.command_id IS NULL THEN
PERFORM 1 FROM game_durability_reconnect_sessions s
JOIN game_durability_reconnect_sessions d ON d.game_session_id=NEW.destination_game_session_id
WHERE s.game_session_id=NEW.source_game_session_id AND s.control_loss_epoch IS NOT NULL
AND s.original_grace_deadline IS NOT NULL AND s.original_grace_deadline<=NEW.returned_at/1000
AND NEW.destination_transport_ref IS NULL AND d.current_transport_ref IS NULL
AND d.control_loss_epoch IS NOT NULL AND d.original_grace_deadline=s.original_grace_deadline
FOR SHARE OF s,d;
IF NOT FOUND THEN
RAISE EXCEPTION 'autonomous activity logout requires its recorded expired loss' USING ERRCODE='23514';
END IF;
END IF;
IF NEW.reason=5 THEN
PERFORM 1 FROM game_character_pending_respawns p
JOIN game_character_death_receipts d ON (d.death_occurrence_id,d.character_id,d.respawn_position)
=(p.death_occurrence_id,p.character_id,p.respawn_position)
JOIN game_activity_handoff_groups g ON g.handoff_id=NEW.handoff_id
WHERE p.character_id=NEW.character_id AND p.death_occurrence_id=NEW.death_occurrence_id
AND d.death_scope_kind=2 AND d.death_world_id=g.world_id AND d.death_instance_id=g.instance_id
AND d.death_scope_generation=g.ownership_generation
AND p.respawn_position=int4send(NEW.return_x)||int4send(NEW.return_y)||int2send(NEW.return_floor)
AND game_player_death_items_complete(p.death_occurrence_id)
FOR SHARE OF p,d,g;
IF NOT FOUND THEN
RAISE EXCEPTION 'activity death return requires its complete retained death and pending temple' USING ERRCODE='23514';
END IF;
END IF;
RETURN NULL;
END; $$;
