-- Platform owns revocations. Game retains their identities and captures affected sessions.
CREATE TABLE game_security_revocations (
    revocation_id UUID PRIMARY KEY CHECK (get_byte(uuid_send(revocation_id),6)>>4=7 AND get_byte(uuid_send(revocation_id),8)&192=128),
    account_id UUID NOT NULL CHECK (get_byte(uuid_send(account_id),6)>>4=7 AND get_byte(uuid_send(account_id),8)&192=128),
    native_security_generation BIGINT NOT NULL CHECK (native_security_generation>0),
    cause SMALLINT NOT NULL CHECK (cause BETWEEN 1 AND 4),
    effective_at_ms BIGINT NOT NULL CHECK (effective_at_ms>0)
);
CREATE INDEX game_security_revocations_account ON game_security_revocations(account_id);
CREATE TABLE game_security_revocation_floors (
    account_id UUID PRIMARY KEY,
    native_security_generation BIGINT NOT NULL CHECK (native_security_generation>0)
);
ALTER TABLE game_durability_reconnect_sessions
    ADD COLUMN admission_security_generation NUMERIC(20,0) CHECK (admission_security_generation>0),
    ADD COLUMN security_revoked BOOLEAN NOT NULL DEFAULT false;
REVOKE ALL ON game_security_revocations,game_security_revocation_floors FROM PUBLIC;
GRANT SELECT,INSERT ON game_security_revocations TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_security_revocation_floors TO oteryn_game_runtime;
GRANT SELECT ON game_security_revocations,game_security_revocation_floors TO oteryn_game_control;

-- Includes native successors of a captured activity session; these never revive authority.
CREATE FUNCTION game_security_session_revoked(p_session UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SET search_path=pg_catalog,public AS $$
    SELECT EXISTS(SELECT 1 FROM public.game_durability_reconnect_sessions s
        WHERE s.game_session_id=p_session AND s.security_revoked)
        OR EXISTS(SELECT 1 FROM public.game_activity_runtime_returns a
        JOIN public.game_durability_reconnect_sessions s ON s.game_session_id=a.source_game_session_id
        WHERE a.destination_game_session_id=p_session AND s.security_revoked)
$$;
REVOKE ALL ON FUNCTION game_security_session_revoked(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_security_session_revoked(UUID) TO oteryn_game_runtime;

-- Admission and stream application both hold the canonical admission relation fence.
-- Defend PREPARE and COMMIT (current and candidate proofs) at the durable mutation boundary.
CREATE FUNCTION game_security_session_fence() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$
BEGIN
    IF TG_OP='UPDATE' THEN
        IF OLD.security_revoked AND NOT NEW.security_revoked THEN
            RAISE EXCEPTION 'security revocation is terminal';
        END IF;
        IF public.game_security_session_revoked(OLD.game_session_id) AND NEW.session_state<>3
            AND (NEW.current_transport_ref IS DISTINCT FROM OLD.current_transport_ref AND NEW.current_transport_ref IS NOT NULL
                OR NEW.current_generation<>OLD.current_generation
                OR NEW.prepared_attempt_ref IS DISTINCT FROM OLD.prepared_attempt_ref AND NEW.prepared_attempt_ref IS NOT NULL) THEN
            RAISE EXCEPTION 'security revoked reconnect';
        END IF;
    ELSE
        IF NEW.admission_security_generation IS NOT NULL AND EXISTS(
            SELECT 1 FROM public.game_security_revocation_floors f WHERE f.account_id=NEW.account_id
            AND f.native_security_generation>NEW.admission_security_generation) THEN
            RAISE EXCEPTION 'security generation revoked';
        END IF;
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_security_session_fence() FROM PUBLIC;
CREATE TRIGGER game_security_session_fence BEFORE INSERT OR UPDATE ON game_durability_reconnect_sessions
    FOR EACH ROW EXECUTE FUNCTION game_security_session_fence();

-- Captured Platform revocations prove staff terminal returns under the existing transport/epoch fences.
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
