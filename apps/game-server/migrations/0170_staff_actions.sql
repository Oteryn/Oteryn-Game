-- GM-1: signed operators transported by a logged-in session. Trust is supplied
-- from reviewed off-host history at process start, never from these mirrors.
CREATE TABLE game_staff_rosters (
    environment text NOT NULL CHECK (environment ~ '^[A-Za-z0-9_.@-]{1,64}$'),
    activation_sha256 text NOT NULL CHECK (activation_sha256 ~ '^[0-9a-f]{64}$'),
    roster_sha256 text NOT NULL CHECK (roster_sha256 ~ '^[0-9a-f]{64}$'),
    principal text NOT NULL CHECK (principal ~ '^[A-Za-z0-9_.@-]{1,64}$'),
    key_fingerprint text NOT NULL,
    role text NOT NULL CHECK (role IN ('GM','SENIOR_GM','ADMIN')),
    PRIMARY KEY(environment, activation_sha256, principal)
);
CREATE TABLE game_staff_control_links (
    environment text NOT NULL,
    control_seq bigint NOT NULL CHECK (control_seq > 0),
    kind text NOT NULL CHECK (kind IN ('ACTIVATION','REQUEST','RECEIPT')),
    link_sha256 text NOT NULL CHECK (link_sha256 ~ '^[0-9a-f]{64}$'),
    previous_link_sha256 text NOT NULL CHECK (previous_link_sha256 ~ '^[0-9a-f]{64}$'),
    activation_sha256 text NOT NULL CHECK (activation_sha256 ~ '^[0-9a-f]{64}$'),
    roster_sha256 text NOT NULL CHECK (roster_sha256 ~ '^[0-9a-f]{64}$'),
    principal text NOT NULL,
    key_fingerprint text NOT NULL,
    canonical bytea NOT NULL CHECK (octet_length(canonical) BETWEEN 1 AND 8192),
    signature bytea NOT NULL CHECK (octet_length(signature) BETWEEN 1 AND 4096),
    operation_id uuid CHECK (game_character_is_uuid_v7(operation_id)),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(environment, control_seq),
    UNIQUE(environment, link_sha256),
    UNIQUE(environment, kind, operation_id),
    FOREIGN KEY(environment, activation_sha256, principal)
        REFERENCES game_staff_rosters(environment, activation_sha256, principal),
    CHECK ((kind = 'ACTIVATION') = (operation_id IS NULL))
);
CREATE TABLE game_staff_control_heads (
    environment text PRIMARY KEY,
    control_seq bigint NOT NULL,
    link_sha256 text NOT NULL,
    activation_sha256 text NOT NULL,
    pending_operation_id uuid,
    FOREIGN KEY(environment,control_seq) REFERENCES game_staff_control_links(environment,control_seq)
);
CREATE TABLE game_staff_operations (
    environment text NOT NULL,
    operation_id uuid NOT NULL CHECK (game_character_is_uuid_v7(operation_id)),
    control_seq bigint NOT NULL,
    kind text NOT NULL CHECK (kind IN ('INSPECT','MUTE','UNMUTE')),
    target_character_id uuid NOT NULL CHECK (game_character_is_uuid_v7(target_character_id)),
    world_id uuid NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id uuid NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    starts_at_ms bigint NOT NULL CHECK (starts_at_ms > 0),
    duration_ms bigint NOT NULL CHECK (duration_ms BETWEEN 0 AND 86400000),
    result smallint NOT NULL CHECK (result IN (1,2)),
    outcome bytea NOT NULL CHECK (octet_length(outcome) BETWEEN 1 AND 512),
    outcome_sha256 text NOT NULL CHECK (outcome_sha256 ~ '^[0-9a-f]{64}$'),
    requires_receipt boolean NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(environment, operation_id),
    FOREIGN KEY(environment,control_seq) REFERENCES game_staff_control_links(environment,control_seq),
    CHECK (world_id <> channel_id),
    CHECK ((kind='MUTE') = (duration_ms>0)),
    CHECK (requires_receipt = (kind<>'INSPECT' AND result=1))
);
CREATE TABLE game_staff_action_receipts (
    environment text NOT NULL,
    operation_id uuid NOT NULL,
    control_seq bigint NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(environment,operation_id),
    FOREIGN KEY(environment,operation_id) REFERENCES game_staff_operations(environment,operation_id),
    FOREIGN KEY(environment,control_seq) REFERENCES game_staff_control_links(environment,control_seq)
);
CREATE TABLE game_character_sanctions (
    environment text NOT NULL,
    sanction_id uuid NOT NULL,
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    starts_at_ms bigint NOT NULL CHECK (starts_at_ms>0),
    ends_at_ms bigint NOT NULL,
    lift_operation_id uuid,
    lifted_at_ms bigint CHECK (lifted_at_ms>0),
    PRIMARY KEY(environment,sanction_id),
    UNIQUE(environment,lift_operation_id),
    FOREIGN KEY(environment,sanction_id) REFERENCES game_staff_operations(environment,operation_id),
    FOREIGN KEY(environment,lift_operation_id) REFERENCES game_staff_operations(environment,operation_id),
    CHECK (ends_at_ms>starts_at_ms AND ends_at_ms-starts_at_ms<=86400000),
    CHECK ((lift_operation_id IS NULL) = (lifted_at_ms IS NULL))
);
CREATE INDEX game_character_sanctions_active ON game_character_sanctions(environment,character_id,ends_at_ms DESC) WHERE lifted_at_ms IS NULL;
CREATE INDEX game_character_sanctions_character_active ON game_character_sanctions(character_id,ends_at_ms DESC) INCLUDE (environment) WHERE lifted_at_ms IS NULL;
CREATE TABLE game_staff_action_outbox (
    event_id uuid PRIMARY KEY DEFAULT game_character_uuid_v7() CHECK (game_character_is_uuid_v7(event_id)),
    environment text NOT NULL,
    operation_id uuid NOT NULL,
    event_type_id integer NOT NULL DEFAULT 5 CHECK (event_type_id=5),
    schema_revision integer NOT NULL DEFAULT 1 CHECK (schema_revision=1),
    retention_profile_id text NOT NULL DEFAULT 'STAFF_ACTION_AUDIT_RETENTION_V1' CHECK (retention_profile_id='STAFF_ACTION_AUDIT_RETENTION_V1'),
    payload bytea NOT NULL CHECK (octet_length(payload) BETWEEN 1 AND 16384),
    starts_at_ms bigint NOT NULL CHECK (starts_at_ms>0),
    expires_at_ms bigint NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(environment,operation_id),
    FOREIGN KEY(environment,operation_id) REFERENCES game_staff_operations(environment,operation_id),
    CHECK (expires_at_ms=starts_at_ms+31536000000)
);
CREATE FUNCTION game_staff_head_successor() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE l game_staff_control_links;
BEGIN
    SELECT * INTO STRICT l FROM game_staff_control_links
    WHERE environment=NEW.environment AND control_seq=NEW.control_seq;
    IF l.created_xact_id<>pg_current_xact_id() OR l.link_sha256<>NEW.link_sha256 OR l.activation_sha256<>NEW.activation_sha256 THEN
        RAISE EXCEPTION 'staff head needs same-transaction link' USING ERRCODE='23514';
    END IF;
    IF TG_OP='INSERT' THEN
        IF l.kind<>'ACTIVATION' OR l.control_seq<>1 OR l.previous_link_sha256<>repeat('0',64) OR NEW.pending_operation_id IS NOT NULL THEN
            RAISE EXCEPTION 'invalid staff bootstrap' USING ERRCODE='23514';
        END IF;
    ELSE
        IF NEW.environment<>OLD.environment OR NEW.control_seq<>OLD.control_seq+1 OR l.previous_link_sha256<>OLD.link_sha256
           OR (l.kind<>'ACTIVATION' AND NEW.activation_sha256<>OLD.activation_sha256)
           OR (l.kind='ACTIVATION' AND OLD.pending_operation_id IS NOT NULL)
           OR (OLD.pending_operation_id IS NOT NULL AND (l.kind<>'RECEIPT' OR l.operation_id<>OLD.pending_operation_id OR NEW.pending_operation_id IS NOT NULL)) THEN
            RAISE EXCEPTION 'invalid staff successor or pending receipt' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_staff_head_successor BEFORE INSERT OR UPDATE ON game_staff_control_heads FOR EACH ROW EXECUTE FUNCTION game_staff_head_successor();
-- Traverse actual memberships; pg_has_role treats a superuser as a member of
-- every role, including executor roles it has never joined.
CREATE FUNCTION game_staff_is_executor_member() RETURNS boolean LANGUAGE sql STABLE SECURITY INVOKER AS $$
    WITH RECURSIVE actual_roles(oid) AS (
        SELECT oid FROM pg_roles WHERE rolname IN (session_user,current_user)
        UNION
        SELECT m.roleid FROM pg_auth_members m JOIN actual_roles r ON r.oid=m.member
    )
    SELECT EXISTS(SELECT 1 FROM actual_roles a JOIN pg_roles r USING(oid) WHERE r.rolname IN ('oteryn_game_runtime','oteryn_game_control'))
$$;
CREATE FUNCTION game_staff_activation_owner_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.kind='ACTIVATION' AND
       game_staff_is_executor_member() THEN
        RAISE EXCEPTION 'staff activation needs owner bootstrap' USING ERRCODE='42501';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_staff_activation_owner_only BEFORE INSERT ON game_staff_control_links FOR EACH ROW EXECUTE FUNCTION game_staff_activation_owner_only();
-- Separate trigger avoids accessing a nonexistent kind field on roster rows.
CREATE FUNCTION game_staff_roster_owner_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF game_staff_is_executor_member() THEN
        RAISE EXCEPTION 'staff roster needs owner bootstrap' USING ERRCODE='42501';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_staff_roster_owner_only BEFORE INSERT ON game_staff_rosters FOR EACH ROW EXECUTE FUNCTION game_staff_roster_owner_only();
CREATE FUNCTION game_staff_operation_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE l game_staff_control_links; o game_staff_operations; h game_staff_control_heads;
BEGIN
    SELECT * INTO STRICT o FROM game_staff_operations WHERE environment=NEW.environment AND operation_id=NEW.operation_id;
    SELECT * INTO STRICT l FROM game_staff_control_links WHERE environment=o.environment AND control_seq=o.control_seq;
    SELECT * INTO STRICT h FROM game_staff_control_heads WHERE environment=o.environment;
    IF o.created_xact_id<>pg_current_xact_id() OR l.created_xact_id<>pg_current_xact_id() OR l.kind<>'REQUEST' OR l.operation_id<>o.operation_id
       OR (convert_from(l.canonical,'UTF8')::jsonb->'action'->>'kind') IS DISTINCT FROM o.kind
       OR replace(o.target_character_id::text,'-','') IS DISTINCT FROM (convert_from(l.canonical,'UTF8')::jsonb->'action'->>'character_id')
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
    RETURN NULL;
END; $$;
CREATE CONSTRAINT TRIGGER game_staff_operation_bound AFTER INSERT ON game_staff_operations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_staff_operation_bound();
CREATE FUNCTION game_character_sanction_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_staff_operations; l game_staff_control_links;
BEGIN
    IF TG_OP='UPDATE' AND (ROW(NEW.environment,NEW.sanction_id,NEW.character_id,NEW.starts_at_ms,NEW.ends_at_ms) IS DISTINCT FROM ROW(OLD.environment,OLD.sanction_id,OLD.character_id,OLD.starts_at_ms,OLD.ends_at_ms) OR OLD.lift_operation_id IS NOT NULL OR NEW.lift_operation_id IS NULL) THEN
        RAISE EXCEPTION 'sanction evidence immutable' USING ERRCODE='23514';
    END IF;
    SELECT * INTO STRICT o FROM game_staff_operations WHERE environment=NEW.environment AND operation_id=CASE WHEN TG_OP='INSERT' THEN NEW.sanction_id ELSE NEW.lift_operation_id END;
    SELECT * INTO STRICT l FROM game_staff_control_links WHERE environment=o.environment AND control_seq=o.control_seq;
    IF o.created_xact_id<>pg_current_xact_id() OR o.result<>1 OR o.target_character_id<>NEW.character_id OR NOT EXISTS(SELECT 1 FROM game_character_roots c WHERE c.character_id=NEW.character_id AND c.world_id=o.world_id) OR NOT EXISTS(SELECT 1 FROM game_staff_rosters r WHERE r.environment=l.environment AND r.activation_sha256=l.activation_sha256 AND r.roster_sha256=l.roster_sha256 AND r.principal=l.principal AND r.key_fingerprint=l.key_fingerprint) THEN
        RAISE EXCEPTION 'sanction needs same-transaction signed operation' USING ERRCODE='23514';
    END IF;
    IF TG_OP='INSERT' THEN
        IF o.kind<>'MUTE' OR o.starts_at_ms<>NEW.starts_at_ms OR o.duration_ms<>NEW.ends_at_ms-NEW.starts_at_ms OR NEW.lift_operation_id IS NOT NULL THEN
            RAISE EXCEPTION 'mute binding mismatch' USING ERRCODE='23514';
        END IF;
    ELSIF o.kind<>'UNMUTE' OR o.starts_at_ms<>NEW.lifted_at_ms OR replace(NEW.sanction_id::text,'-','')<>convert_from(l.canonical,'UTF8')::jsonb->'action'->>'sanction_id' THEN
        RAISE EXCEPTION 'unmute binding mismatch' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_character_sanction_bound BEFORE INSERT OR UPDATE ON game_character_sanctions FOR EACH ROW EXECUTE FUNCTION game_character_sanction_bound();
DO $$
DECLARE t text; n text;
BEGIN
    FOREACH t IN ARRAY ARRAY['game_staff_rosters','game_staff_control_links','game_staff_operations','game_staff_action_receipts'] LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',t||'_immutable',t);
    END LOOP;
    FOREACH t IN ARRAY ARRAY['game_staff_rosters','game_staff_control_links','game_staff_control_heads','game_staff_operations','game_staff_action_receipts','game_character_sanctions','game_staff_action_outbox'] LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I EXECUTE FUNCTION game_item_reject_truncate()',t||'_no_truncate',t);
        EXECUTE format('REVOKE ALL ON %I FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',t);
        EXECUTE format('GRANT SELECT ON %I TO oteryn_game_runtime,oteryn_game_control',t);
    END LOOP;
    FOREACH t IN ARRAY ARRAY['game_staff_control_links','game_staff_operations','game_staff_action_receipts','game_staff_action_outbox'] LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE INSERT ON %I FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id()',t||'_stamp',t);
        EXECUTE format('GRANT INSERT ON %I TO oteryn_game_runtime',t);
    END LOOP;
    FOREACH n IN ARRAY ARRAY['game_staff_is_executor_member','game_staff_head_successor','game_staff_activation_owner_only','game_staff_roster_owner_only','game_staff_operation_bound','game_character_sanction_bound'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',n,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
    END LOOP;
END; $$;
GRANT UPDATE ON game_staff_control_heads TO oteryn_game_runtime;
GRANT INSERT ON game_character_sanctions TO oteryn_game_runtime;
GRANT UPDATE(lift_operation_id,lifted_at_ms) ON game_character_sanctions TO oteryn_game_runtime;

CREATE FUNCTION game_staff_link_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE h game_staff_control_heads; o game_staff_operations; r game_staff_control_links; body jsonb;
BEGIN
    SELECT * INTO STRICT h FROM game_staff_control_heads WHERE environment=NEW.environment;
    body=convert_from(NEW.canonical,'UTF8')::jsonb;
    IF h.control_seq<>NEW.control_seq OR h.link_sha256<>NEW.link_sha256 OR
       encode(sha256(NEW.canonical),'hex')<>NEW.link_sha256 OR
       (body->>'environment') IS DISTINCT FROM NEW.environment OR
       (body->>'control_seq') IS DISTINCT FROM NEW.control_seq::text OR
       (body->>'previous_link_sha256') IS DISTINCT FROM NEW.previous_link_sha256 OR
       (body->>'roster_sha256') IS DISTINCT FROM NEW.roster_sha256 OR
       NOT EXISTS(SELECT 1 FROM game_staff_rosters x WHERE x.environment=NEW.environment AND x.activation_sha256=NEW.activation_sha256 AND x.roster_sha256=NEW.roster_sha256 AND x.principal=NEW.principal AND x.key_fingerprint=NEW.key_fingerprint) THEN
        RAISE EXCEPTION 'staff link mirror mismatch' USING ERRCODE='23514';
    END IF;
    IF NEW.kind='ACTIVATION' THEN RETURN NULL; END IF;
    SELECT * INTO STRICT o FROM game_staff_operations WHERE environment=NEW.environment AND operation_id=NEW.operation_id;
    IF NEW.kind='REQUEST' THEN
        IF o.control_seq<>NEW.control_seq OR o.created_xact_id<>pg_current_xact_id() OR
           replace(o.world_id::text,'-','') IS DISTINCT FROM body->>'world_id' OR
           replace(o.channel_id::text,'-','') IS DISTINCT FROM body->>'channel_id' OR
           (o.kind='MUTE' AND o.duration_ms::text IS DISTINCT FROM body->'action'->>'duration_ms') THEN
            RAISE EXCEPTION 'staff request mirror mismatch' USING ERRCODE='23514';
        END IF;
    ELSE
        SELECT * INTO STRICT r FROM game_staff_control_links WHERE environment=o.environment AND control_seq=o.control_seq;
        IF NOT EXISTS(SELECT 1 FROM game_staff_action_receipts a WHERE a.environment=NEW.environment AND a.operation_id=NEW.operation_id AND a.control_seq=NEW.control_seq AND a.created_xact_id=pg_current_xact_id()) OR
           r.principal<>NEW.principal OR r.key_fingerprint<>NEW.key_fingerprint OR
           body->>'request_sha256' IS DISTINCT FROM r.link_sha256 OR
           body->>'outcome_sha256' IS DISTINCT FROM o.outcome_sha256 OR
           body->>'disposition' IS DISTINCT FROM (CASE WHEN o.result=1 THEN 'COMMITTED' ELSE 'REFUSED' END) OR
           o.kind='INSPECT' OR h.pending_operation_id IS NOT NULL THEN
            RAISE EXCEPTION 'staff receipt mirror mismatch' USING ERRCODE='23514';
        END IF;
    END IF;
    IF body->>'operation_id' IS DISTINCT FROM replace(NEW.operation_id::text,'-','') OR body->>'activation_sha256' IS DISTINCT FROM NEW.activation_sha256 THEN
        RAISE EXCEPTION 'staff operation identity mismatch' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END; $$;
CREATE CONSTRAINT TRIGGER game_staff_link_bound AFTER INSERT ON game_staff_control_links DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_staff_link_bound();
CREATE FUNCTION game_staff_receipt_bound() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM game_staff_control_links l WHERE l.environment=NEW.environment AND l.operation_id=NEW.operation_id AND l.control_seq=NEW.control_seq AND l.kind='RECEIPT' AND l.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'staff receipt has no matching link' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END; $$;
CREATE CONSTRAINT TRIGGER game_staff_receipt_bound AFTER INSERT ON game_staff_action_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_staff_receipt_bound();
CREATE TRIGGER game_character_sanction_no_delete BEFORE DELETE ON game_character_sanctions FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
DO $$ DECLARE n text; BEGIN
    FOREACH n IN ARRAY ARRAY['game_staff_link_bound','game_staff_receipt_bound'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',n,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
    END LOOP;
END; $$;

CREATE FUNCTION game_staff_audit_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP<>'DELETE' OR current_user<>(SELECT pg_get_userbyid(c.relowner) FROM pg_class c WHERE c.oid=TG_RELID) OR
       OLD.expires_at_ms+900000>floor(extract(epoch FROM clock_timestamp())*1000)::bigint OR
       EXISTS(SELECT 1 FROM game_staff_operations o WHERE o.environment=OLD.environment AND o.operation_id=OLD.operation_id AND o.requires_receipt AND NOT EXISTS(SELECT 1 FROM game_staff_action_receipts r WHERE r.environment=o.environment AND r.operation_id=o.operation_id)) THEN
        RAISE EXCEPTION 'staff audit is immutable before expiry' USING ERRCODE='23514';
    END IF;
    RETURN OLD;
END; $$;
CREATE TRIGGER game_staff_audit_immutable BEFORE UPDATE OR DELETE ON game_staff_action_outbox FOR EACH ROW EXECUTE FUNCTION game_staff_audit_immutable();
CREATE FUNCTION game_staff_expire_action_audit() RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE deleted bigint;
BEGIN
    DELETE FROM game_staff_action_outbox WHERE event_id IN (
        SELECT a.event_id FROM game_staff_action_outbox a
        WHERE a.expires_at_ms+900000<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint
        AND NOT EXISTS(SELECT 1 FROM game_staff_operations o WHERE o.environment=a.environment AND o.operation_id=a.operation_id AND o.requires_receipt AND NOT EXISTS(SELECT 1 FROM game_staff_action_receipts r WHERE r.environment=o.environment AND r.operation_id=o.operation_id))
        ORDER BY a.expires_at_ms,a.event_id LIMIT 64 FOR UPDATE);
    GET DIAGNOSTICS deleted=ROW_COUNT;
    RETURN deleted;
END; $$;
DO $$ DECLARE n text; BEGIN
    FOREACH n IN ARRAY ARRAY['game_staff_audit_immutable','game_staff_expire_action_audit'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',n,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
    END LOOP;
END; $$;
GRANT EXECUTE ON FUNCTION game_staff_expire_action_audit() TO oteryn_game_runtime;

GRANT EXECUTE ON FUNCTION game_staff_is_executor_member() TO oteryn_game_runtime,oteryn_game_control;

-- Neither a detached audit insert nor replacing expired evidence can masquerade
-- as a newly committed action. The reciprocal operation guard requires this row.
CREATE FUNCTION game_staff_action_audit_bound() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM game_staff_operations o
                  WHERE o.environment=NEW.environment AND o.operation_id=NEW.operation_id
                  AND o.created_xact_id=pg_current_xact_id()
                  AND NEW.created_xact_id=pg_current_xact_id()
                  AND o.starts_at_ms=NEW.starts_at_ms) THEN
        RAISE EXCEPTION 'staff audit needs same-transaction operation' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END; $$;
CREATE CONSTRAINT TRIGGER game_staff_action_audit_bound AFTER INSERT ON game_staff_action_outbox
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_staff_action_audit_bound();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_staff_action_audit_bound() SET search_path=%I,pg_temp',current_schema());
END; $$;
REVOKE ALL ON FUNCTION game_staff_action_audit_bound() FROM PUBLIC;
