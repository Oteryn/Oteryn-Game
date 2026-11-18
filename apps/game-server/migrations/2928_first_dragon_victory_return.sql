-- Crystal 201189f8, creaturescripts_death_first_dragon.lua:21-30.
-- Only a recorded victory in this exact activity selects the reward room.
-- The deferred return guard may see the room retired by the last return.
CREATE FUNCTION game_first_dragon_reward_room(p_admission UUID)
RETURNS TABLE (x INTEGER, y INTEGER, floor SMALLINT)
LANGUAGE sql STABLE SET search_path=pg_catalog,public AS $$
    SELECT 33617, 31020, 13::smallint FROM public.game_boss_room_activities
    WHERE admission=p_admission AND boss_key='oteryn:boss.the_first_dragon'
      AND state IN (2,4) AND defeat_id IS NOT NULL
$$;
REVOKE ALL ON FUNCTION game_first_dragon_reward_room(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_first_dragon_reward_room(UUID) TO oteryn_game_runtime;

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
AND (NEW.reason=5 OR
    ARRAY[NEW.return_x,NEW.return_y,NEW.return_floor]=
    CASE WHEN NEW.reason=3 THEN COALESCE(
        (SELECT ARRAY[x,y,floor] FROM game_first_dragon_reward_room(g.handoff_id)),
        ARRAY[q.exit_x,q.exit_y,q.exit_floor])
    ELSE ARRAY[q.exit_x,q.exit_y,q.exit_floor] END)
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
AND b.lifted_at_ms IS NULL AND b.starts_at_ms<=NEW.returned_at AND b.ends_at_ms>NEW.returned_at)
OR game_security_session_revoked(s.game_session_id))
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
AND (EXISTS(SELECT 1 FROM game_staff_disconnects k WHERE k.game_session_id=s.game_session_id)
OR game_security_session_revoked(s.game_session_id))
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

-- First Dragon victory is due immediately. Other boss schedules remain exact.
CREATE OR REPLACE FUNCTION game_boss_room_runtime_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$ BEGIN
    IF TG_OP = 'INSERT' THEN
        PERFORM 1 FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id)
            WHERE g.handoff_id=NEW.admission AND g.state=2 AND g.world_id=NEW.world_id AND g.instance_id=NEW.instance_id
              AND g.activity_key=NEW.boss_key AND g.content_revision=NEW.content_revision
              AND e.deadline=NEW.deadline AND g.prepared_at=NEW.opened_at;
        IF NOT FOUND OR NEW.state<>1 OR NEW.boss_creature_key IS NULL OR NEW.defeat_id IS NOT NULL THEN
            RAISE EXCEPTION 'boss room requires its complete committed activity' USING ERRCODE='23514';
        END IF;
    ELSIF TG_OP='UPDATE' THEN
        IF (to_jsonb(NEW)-'state'-'terminal_at'-'exit_deadline'-'defeat_id') IS DISTINCT FROM
           (to_jsonb(OLD)-'state'-'terminal_at'-'exit_deadline'-'defeat_id') THEN
            RAISE EXCEPTION 'boss admission identity and policy are immutable' USING ERRCODE='23514';
        END IF;
        IF OLD.state=1 AND NEW.state=2 AND NEW.defeat_id IS NOT NULL
           AND NEW.terminal_at BETWEEN OLD.opened_at AND OLD.deadline
           AND CASE WHEN OLD.boss_key='oteryn:boss.the_first_dragon'
                    THEN NEW.exit_deadline=NEW.terminal_at
                    ELSE ((OLD.after_kill_ms=0 AND NEW.exit_deadline IS NULL)
                       OR (OLD.after_kill_ms>0 AND NEW.exit_deadline=NEW.terminal_at+OLD.after_kill_ms))
               END THEN RETURN NEW; END IF;
        IF OLD.state=1 AND NEW.state=3 AND NEW.defeat_id IS NULL
           AND NEW.terminal_at>=OLD.deadline AND NEW.exit_deadline=NEW.terminal_at THEN RETURN NEW; END IF;
        IF OLD.state IN(1,2,3) AND NEW.state=4 AND NOT EXISTS(
            SELECT 1 FROM game_activity_handoff_participants p WHERE p.handoff_id=OLD.admission
              AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE (r.handoff_id,r.character_id)=(p.handoff_id,p.character_id)))
           AND (OLD.defeat_id IS NOT DISTINCT FROM NEW.defeat_id) THEN RETURN NEW; END IF;
        RAISE EXCEPTION 'boss room transition is not current or complete' USING ERRCODE='23514';
    ELSE
        RAISE EXCEPTION 'boss room history is retained' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
