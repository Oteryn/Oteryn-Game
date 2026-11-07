-- NX-1: owner-accepted donor GM/CM/God tools use the existing signed audit chain.
ALTER TABLE game_staff_operations DROP CONSTRAINT game_staff_operations_kind_check;
ALTER TABLE game_staff_operations ADD CONSTRAINT game_staff_operations_kind_check
    CHECK (kind IN ('INSPECT','MUTE','UNMUTE','KICK','BAN','UNBAN','TELEPORT','TELEPORT_TO_PLAYER','BRING','NOTIFY','BROADCAST','GHOST','INVISIBLE'));
ALTER TABLE game_staff_operations ADD COLUMN issuer_character_id uuid REFERENCES game_character_roots(character_id);
CREATE INDEX game_staff_operations_issuer_idx ON game_staff_operations(issuer_character_id);
DO $$
DECLARE c record;
BEGIN
    FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_staff_operations'::regclass
        AND contype='c' AND pg_get_constraintdef(oid) LIKE '%duration_ms%'
    LOOP EXECUTE format('ALTER TABLE game_staff_operations DROP CONSTRAINT %I',c.conname); END LOOP;
END; $$;
ALTER TABLE game_staff_operations ADD CONSTRAINT game_staff_operations_duration_ms_check
    CHECK (duration_ms BETWEEN 0 AND 31536000000);
ALTER TABLE game_staff_operations ADD CONSTRAINT game_staff_operations_duration_shape
    CHECK ((kind IN ('MUTE','BAN'))=(duration_ms>0));

CREATE TABLE game_staff_bans (
    environment text NOT NULL,
    sanction_id uuid NOT NULL,
    account_id uuid NOT NULL,
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    starts_at_ms bigint NOT NULL CHECK (starts_at_ms>0),
    ends_at_ms bigint NOT NULL CHECK (ends_at_ms>starts_at_ms AND ends_at_ms-starts_at_ms<=31536000000),
    lift_operation_id uuid,
    lifted_at_ms bigint,
    PRIMARY KEY(environment,sanction_id),
    FOREIGN KEY(environment,sanction_id) REFERENCES game_staff_operations(environment,operation_id),
    FOREIGN KEY(environment,lift_operation_id) REFERENCES game_staff_operations(environment,operation_id),
    CHECK ((lift_operation_id IS NULL)=(lifted_at_ms IS NULL)),
    CHECK (lifted_at_ms IS NULL OR lifted_at_ms>0)
);
CREATE INDEX game_staff_bans_account_active_idx ON game_staff_bans(account_id,ends_at_ms) WHERE lifted_at_ms IS NULL;
CREATE INDEX game_staff_bans_character_idx ON game_staff_bans(character_id);
CREATE INDEX game_staff_bans_lift_idx ON game_staff_bans(environment,lift_operation_id);

CREATE TABLE game_staff_disconnects (
    environment text NOT NULL,
    operation_id uuid NOT NULL,
    game_session_id uuid NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    PRIMARY KEY(environment,operation_id,game_session_id),
    FOREIGN KEY(environment,operation_id) REFERENCES game_staff_operations(environment,operation_id)
);
CREATE INDEX game_staff_disconnects_session_idx ON game_staff_disconnects(game_session_id);

CREATE TABLE game_staff_world_effects (
    environment text NOT NULL,
    operation_id uuid NOT NULL,
    target_character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    issuer_character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    target_game_session_id uuid NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    target_connection_generation numeric(20,0) NOT NULL CHECK(target_connection_generation>0),
    target_scope_ownership_generation numeric(20,0) NOT NULL CHECK(target_scope_ownership_generation>0),
    action jsonb NOT NULL CHECK (jsonb_typeof(action)='object' AND octet_length(action::text)<=8192),
    applied boolean NOT NULL DEFAULT false,
    success boolean,
    PRIMARY KEY(environment,operation_id),
    FOREIGN KEY(environment,operation_id) REFERENCES game_staff_operations(environment,operation_id),
    CHECK (applied=(success IS NOT NULL)),
    CHECK ((target_game_session_id IS NULL)=(target_connection_generation IS NULL)),
    CHECK ((target_game_session_id IS NULL)=(target_scope_ownership_generation IS NULL))
);
CREATE INDEX game_staff_world_effects_target_idx ON game_staff_world_effects(target_character_id);
CREATE INDEX game_staff_world_effects_issuer_idx ON game_staff_world_effects(issuer_character_id);
CREATE INDEX game_staff_world_effects_session_idx ON game_staff_world_effects(target_game_session_id);

CREATE TABLE game_staff_actor_flags (
    environment text NOT NULL,
    game_session_id uuid NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    ghost boolean NOT NULL DEFAULT false,
    invisible boolean NOT NULL DEFAULT false,
    operation_id uuid NOT NULL,
    PRIMARY KEY(environment,game_session_id),
    FOREIGN KEY(environment,operation_id) REFERENCES game_staff_operations(environment,operation_id)
);
CREATE INDEX game_staff_actor_flags_session_idx ON game_staff_actor_flags(game_session_id);
CREATE INDEX game_staff_actor_flags_character_idx ON game_staff_actor_flags(character_id);
CREATE INDEX game_staff_actor_flags_operation_idx ON game_staff_actor_flags(environment,operation_id);

CREATE TABLE game_staff_character_groups (
    environment text NOT NULL,
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    group_id smallint NOT NULL CHECK (group_id IN (4,5,6)),
    activation_sha256 text NOT NULL,
    principal text NOT NULL,
    operation_id uuid NOT NULL,
    PRIMARY KEY(environment,character_id),
    FOREIGN KEY(environment,operation_id) REFERENCES game_staff_operations(environment,operation_id),
    FOREIGN KEY(environment,activation_sha256,principal) REFERENCES game_staff_rosters(environment,activation_sha256,principal)
);
CREATE INDEX game_staff_character_groups_character_idx ON game_staff_character_groups(character_id);
CREATE INDEX game_staff_character_groups_operation_idx ON game_staff_character_groups(environment,operation_id);
CREATE INDEX game_staff_character_groups_roster_idx ON game_staff_character_groups(environment,activation_sha256,principal);

CREATE FUNCTION game_staff_ban_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_staff_operations; l game_staff_control_links;
BEGIN
    IF TG_OP='UPDATE' AND (ROW(NEW.environment,NEW.sanction_id,NEW.account_id,NEW.character_id,NEW.starts_at_ms,NEW.ends_at_ms)
        IS DISTINCT FROM ROW(OLD.environment,OLD.sanction_id,OLD.account_id,OLD.character_id,OLD.starts_at_ms,OLD.ends_at_ms)
        OR OLD.lift_operation_id IS NOT NULL OR NEW.lift_operation_id IS NULL) THEN
        RAISE EXCEPTION 'staff ban evidence immutable' USING ERRCODE='23514';
    END IF;
    SELECT * INTO STRICT o FROM game_staff_operations WHERE environment=NEW.environment
        AND operation_id=CASE WHEN TG_OP='INSERT' THEN NEW.sanction_id ELSE NEW.lift_operation_id END;
    IF o.created_xact_id<>pg_current_xact_id() OR o.result<>1 OR NOT EXISTS(
        SELECT 1 FROM game_character_roots c WHERE c.character_id=o.target_character_id AND c.account_id=NEW.account_id) THEN
        RAISE EXCEPTION 'ban lacks atomic signed account decision' USING ERRCODE='23514';
    END IF;
    SELECT * INTO STRICT l FROM game_staff_control_links WHERE environment=o.environment AND control_seq=o.control_seq;
    IF TG_OP='INSERT' THEN
        IF (convert_from(l.canonical,'UTF8')::jsonb->'action'->>'duration_ms') IS DISTINCT FROM (NEW.ends_at_ms-NEW.starts_at_ms)::text
            OR o.kind<>'BAN' OR o.target_character_id<>NEW.character_id OR o.starts_at_ms<>NEW.starts_at_ms
            OR o.duration_ms<>NEW.ends_at_ms-NEW.starts_at_ms THEN
            RAISE EXCEPTION 'ban binding mismatch' USING ERRCODE='23514';
        END IF;
    ELSIF o.kind<>'UNBAN' OR o.starts_at_ms<>NEW.lifted_at_ms THEN
        RAISE EXCEPTION 'unban binding mismatch' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_staff_ban_bound BEFORE INSERT OR UPDATE ON game_staff_bans FOR EACH ROW EXECUTE FUNCTION game_staff_ban_bound();
CREATE TRIGGER game_staff_ban_no_delete BEFORE DELETE ON game_staff_bans FOR EACH ROW EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_staff_disconnect_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_staff_operations;
BEGIN
    SELECT * INTO STRICT o FROM game_staff_operations WHERE environment=NEW.environment AND operation_id=NEW.operation_id;
    IF o.created_xact_id<>pg_current_xact_id() OR o.result<>1 OR o.kind NOT IN ('KICK','BAN') OR NOT EXISTS(
        SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_character_roots c ON c.character_id=o.target_character_id
        WHERE s.game_session_id=NEW.game_session_id AND s.session_state IN(1,2)
        AND ((o.kind='KICK' AND s.character_id=o.target_character_id) OR (o.kind='BAN' AND s.account_id=c.account_id))) THEN
        RAISE EXCEPTION 'disconnect lacks atomic captured target session' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_staff_disconnect_bound BEFORE INSERT ON game_staff_disconnects FOR EACH ROW EXECUTE FUNCTION game_staff_disconnect_bound();
CREATE TRIGGER game_staff_disconnect_no_update BEFORE UPDATE OR DELETE ON game_staff_disconnects FOR EACH ROW EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_staff_actor_flags_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE e game_staff_world_effects;
BEGIN
    SELECT * INTO STRICT e FROM game_staff_world_effects WHERE environment=NEW.environment AND operation_id=NEW.operation_id;
    IF NOT e.applied OR NOT e.success OR e.target_game_session_id IS DISTINCT FROM NEW.game_session_id
        OR e.target_character_id<>NEW.character_id OR e.action->>'kind' NOT IN ('GHOST','INVISIBLE')
        OR (TG_OP='UPDATE' AND ROW(NEW.environment,NEW.game_session_id,NEW.character_id) IS DISTINCT FROM ROW(OLD.environment,OLD.game_session_id,OLD.character_id))
        OR (e.action->>'kind'='GHOST' AND NEW.ghost IS DISTINCT FROM (e.action->>'enabled')::boolean)
        OR (e.action->>'kind'='INVISIBLE' AND NEW.invisible IS DISTINCT FROM (e.action->>'enabled')::boolean)
        OR (TG_OP='INSERT' AND ((e.action->>'kind'='GHOST' AND NEW.invisible) OR (e.action->>'kind'='INVISIBLE' AND NEW.ghost)))
        OR (TG_OP='UPDATE' AND ((e.action->>'kind'='GHOST' AND NEW.invisible<>OLD.invisible)
            OR (e.action->>'kind'='INVISIBLE' AND NEW.ghost<>OLD.ghost))) THEN
        RAISE EXCEPTION 'flags lack acknowledged signed session effect' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_staff_actor_flags_bound BEFORE INSERT OR UPDATE ON game_staff_actor_flags FOR EACH ROW EXECUTE FUNCTION game_staff_actor_flags_bound();
CREATE TRIGGER game_staff_actor_flags_no_delete BEFORE DELETE ON game_staff_actor_flags FOR EACH ROW EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_staff_character_group_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_staff_operations; l game_staff_control_links;
BEGIN
    SELECT * INTO STRICT o FROM game_staff_operations WHERE environment=NEW.environment AND operation_id=NEW.operation_id;
    SELECT * INTO STRICT l FROM game_staff_control_links WHERE environment=o.environment AND control_seq=o.control_seq;
    IF o.created_xact_id<>pg_current_xact_id() OR o.result<>1
        OR o.kind NOT IN ('INSPECT','TELEPORT','TELEPORT_TO_PLAYER','BRING','NOTIFY','BROADCAST','GHOST','INVISIBLE')
        OR o.target_character_id<>NEW.character_id OR o.issuer_character_id IS DISTINCT FROM NEW.character_id
        OR convert_from(l.canonical,'UTF8')::jsonb->>'issuer_character_id' IS DISTINCT FROM replace(NEW.character_id::text,'-','')
        OR l.principal<>NEW.principal OR l.activation_sha256<>NEW.activation_sha256
        OR (TG_OP='UPDATE' AND ROW(NEW.environment,NEW.character_id) IS DISTINCT FROM ROW(OLD.environment,OLD.character_id))
        OR NOT EXISTS(SELECT 1 FROM game_staff_rosters r WHERE r.environment=l.environment
            AND r.activation_sha256=l.activation_sha256 AND r.principal=l.principal AND r.key_fingerprint=l.key_fingerprint AND r.roster_sha256=l.roster_sha256
            AND NEW.group_id=CASE r.role WHEN 'GM' THEN 4 WHEN 'SENIOR_GM' THEN 5 WHEN 'ADMIN' THEN 6 END) THEN
        RAISE EXCEPTION 'group binding lacks exact signed issuer and roster' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_staff_character_group_bound BEFORE INSERT OR UPDATE ON game_staff_character_groups FOR EACH ROW EXECUTE FUNCTION game_staff_character_group_bound();
CREATE TRIGGER game_staff_character_group_no_delete BEFORE DELETE ON game_staff_character_groups FOR EACH ROW EXECUTE FUNCTION game_item_immutable();

REVOKE ALL ON game_staff_bans,game_staff_disconnects,game_staff_world_effects,game_staff_actor_flags,game_staff_character_groups FROM PUBLIC;
GRANT SELECT,INSERT ON game_staff_bans,game_staff_disconnects,game_staff_world_effects,game_staff_actor_flags,game_staff_character_groups TO oteryn_game_runtime;
GRANT UPDATE ON game_staff_bans,game_staff_world_effects,game_staff_actor_flags,game_staff_character_groups TO oteryn_game_runtime;
GRANT SELECT ON game_staff_bans,game_staff_disconnects,game_staff_world_effects,game_staff_actor_flags,game_staff_character_groups TO oteryn_game_control;
