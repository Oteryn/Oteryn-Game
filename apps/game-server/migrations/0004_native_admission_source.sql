-- Durable, non-rollback bootstrap for the single native admission evidence source.
CREATE TABLE game_durability_native_source_registration (
    registration_id SMALLINT PRIMARY KEY CHECK (registration_id = 1),
    bootstrap_namespace TEXT NOT NULL CHECK (octet_length(bootstrap_namespace) BETWEEN 1 AND 256),
    bootstrap_provenance TEXT NOT NULL CHECK (octet_length(bootstrap_provenance) BETWEEN 1 AND 256),
    source_authority TEXT NOT NULL CHECK (
        octet_length(source_authority) BETWEEN 1 AND 128
        AND source_authority !~ '[^A-Za-z0-9._:/-]'
    ),
    descriptor_revision NUMERIC(20, 0) NOT NULL CHECK (descriptor_revision BETWEEN 1 AND 18446744073709551615),
    descriptor_facts BYTEA NOT NULL CHECK (octet_length(descriptor_facts) BETWEEN 1 AND 4096),
    initialized_at BIGINT NOT NULL CHECK (initialized_at >= 0),
    -- Exclusive custody holder: one #415 GameNode process incarnation. Every
    -- mutation additionally proves this incarnation is current in the same
    -- transaction; a stored holder alone never establishes custody.
    custody_node_id UUID NOT NULL REFERENCES game_node_registrations (node_id),
    custody_registration_revision NUMERIC(20, 0) NOT NULL
        CHECK (custody_registration_revision BETWEEN 1 AND 18446744073709551615)
);

CREATE TABLE game_durability_native_source_descriptor_history (
    registration_id SMALLINT NOT NULL REFERENCES game_durability_native_source_registration(registration_id),
    descriptor_revision NUMERIC(20, 0) NOT NULL CHECK (descriptor_revision BETWEEN 1 AND 18446744073709551615),
    descriptor_facts BYTEA NOT NULL CHECK (octet_length(descriptor_facts) BETWEEN 1 AND 4096),
    installed_at BIGINT NOT NULL CHECK (installed_at >= 0),
    PRIMARY KEY (registration_id, descriptor_revision)
);

CREATE TABLE game_durability_native_source_floors (
    registration_id SMALLINT NOT NULL REFERENCES game_durability_native_source_registration(registration_id),
    source_authority TEXT NOT NULL CHECK (
        octet_length(source_authority) BETWEEN 1 AND 128
        AND source_authority !~ '[^A-Za-z0-9._:/-]'
    ),
    operation TEXT NOT NULL CHECK (operation IN (
        'ReadAccountSecurityV1',
        'ReadFreshSigningTrustV1',
        'ReadRecoveryAccountSecurityV2',
        'ReadRecoverySigningTrustV2'
    )),
    floor_subject TEXT NOT NULL CHECK (octet_length(floor_subject) BETWEEN 1 AND 1024),
    observation_subject TEXT NOT NULL CHECK (octet_length(observation_subject) BETWEEN 1 AND 1280),
    signing_key_id TEXT NULL,
    source_revision NUMERIC(20, 0) NOT NULL CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL CHECK (octet_length(decision_identity) BETWEEN 1 AND 256),
    observed_at BIGINT NOT NULL CHECK (observed_at >= 0),
    semantic_facts BYTEA NOT NULL CHECK (octet_length(semantic_facts) BETWEEN 1 AND 8192),
    CHECK (
        (operation IN ('ReadAccountSecurityV1', 'ReadRecoveryAccountSecurityV2')
            AND signing_key_id IS NULL)
        OR
        (operation IN ('ReadFreshSigningTrustV1', 'ReadRecoverySigningTrustV2')
            AND octet_length(signing_key_id) BETWEEN 1 AND 64
            AND signing_key_id !~ '[^A-Za-z0-9._-]')
    ),
    PRIMARY KEY (registration_id, source_authority, floor_subject)
);

CREATE TABLE game_durability_native_source_observation_history (
    registration_id SMALLINT NOT NULL,
    source_authority TEXT NOT NULL CHECK (
        octet_length(source_authority) BETWEEN 1 AND 128
        AND source_authority !~ '[^A-Za-z0-9._:/-]'
    ),
    operation TEXT NOT NULL CHECK (operation IN (
        'ReadAccountSecurityV1',
        'ReadFreshSigningTrustV1',
        'ReadRecoveryAccountSecurityV2',
        'ReadRecoverySigningTrustV2'
    )),
    floor_subject TEXT NOT NULL CHECK (octet_length(floor_subject) BETWEEN 1 AND 1024),
    observation_subject TEXT NOT NULL CHECK (octet_length(observation_subject) BETWEEN 1 AND 1280),
    signing_key_id TEXT NULL,
    source_revision NUMERIC(20, 0) NOT NULL CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL CHECK (octet_length(decision_identity) BETWEEN 1 AND 256),
    observed_at BIGINT NOT NULL CHECK (observed_at >= 0),
    semantic_facts BYTEA NOT NULL CHECK (octet_length(semantic_facts) BETWEEN 1 AND 8192),
    CHECK (
        (operation IN ('ReadAccountSecurityV1', 'ReadRecoveryAccountSecurityV2')
            AND signing_key_id IS NULL)
        OR
        (operation IN ('ReadFreshSigningTrustV1', 'ReadRecoverySigningTrustV2')
            AND octet_length(signing_key_id) BETWEEN 1 AND 64
            AND signing_key_id !~ '[^A-Za-z0-9._-]')
    ),
    PRIMARY KEY (registration_id, source_authority, floor_subject, source_revision),
    FOREIGN KEY (registration_id) REFERENCES game_durability_native_source_registration(registration_id)
);

CREATE TABLE game_durability_native_source_publication_slots (
    registration_id SMALLINT NOT NULL REFERENCES game_durability_native_source_registration(registration_id),
    slot_id SMALLINT NOT NULL CHECK (slot_id IN (1, 2)),
    operation_binding BYTEA NULL CHECK (
        operation_binding IS NULL OR octet_length(operation_binding) BETWEEN 1 AND 16384
    ),
    checkpointed_at BIGINT NULL CHECK (checkpointed_at IS NULL OR checkpointed_at >= 0),
    PRIMARY KEY (registration_id, slot_id),
    CHECK ((operation_binding IS NULL) = (checkpointed_at IS NULL))
);

CREATE FUNCTION game_durability_native_source_immutable_history() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    RAISE EXCEPTION 'native source canonical history is immutable' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_native_descriptor_history_immutable BEFORE UPDATE OR DELETE ON game_durability_native_source_descriptor_history FOR EACH ROW EXECUTE FUNCTION game_durability_native_source_immutable_history();
CREATE TRIGGER game_native_observation_history_immutable BEFORE UPDATE OR DELETE ON game_durability_native_source_observation_history FOR EACH ROW EXECUTE FUNCTION game_durability_native_source_immutable_history();

CREATE FUNCTION game_durability_native_source_registration_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE' OR NEW.registration_id <> OLD.registration_id
       OR NEW.bootstrap_namespace <> OLD.bootstrap_namespace
       OR NEW.bootstrap_provenance <> OLD.bootstrap_provenance
       OR NEW.source_authority <> OLD.source_authority
       OR NEW.initialized_at <> OLD.initialized_at
       OR NEW.descriptor_revision < OLD.descriptor_revision
       OR (NEW.descriptor_revision = OLD.descriptor_revision
           AND (NEW.descriptor_facts <> OLD.descriptor_facts
                OR (NEW.custody_node_id = OLD.custody_node_id
                    AND NEW.custody_registration_revision = OLD.custody_registration_revision)))
       OR (NEW.descriptor_revision > OLD.descriptor_revision
           AND (NEW.custody_node_id <> OLD.custody_node_id
                OR NEW.custody_registration_revision <> OLD.custody_registration_revision)) THEN
        RAISE EXCEPTION 'native source registration cannot roll back or be recreated' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_native_registration_guard BEFORE UPDATE OR DELETE ON game_durability_native_source_registration FOR EACH ROW EXECUTE FUNCTION game_durability_native_source_registration_guard();

CREATE FUNCTION game_durability_native_source_floor_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE' OR NEW.registration_id <> OLD.registration_id
       OR NEW.source_authority <> OLD.source_authority
       OR NEW.floor_subject <> OLD.floor_subject
       OR NEW.source_revision <= OLD.source_revision THEN
        RAISE EXCEPTION 'native source floor cannot roll back or be rewritten' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_native_floor_guard BEFORE UPDATE OR DELETE ON game_durability_native_source_floors FOR EACH ROW EXECUTE FUNCTION game_durability_native_source_floor_guard();

CREATE FUNCTION game_durability_native_source_slot_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE' OR NEW.registration_id <> OLD.registration_id OR NEW.slot_id <> OLD.slot_id
       OR (OLD.operation_binding IS NOT NULL AND NEW.operation_binding IS NOT NULL
           AND NEW.operation_binding IS DISTINCT FROM OLD.operation_binding) THEN
        RAISE EXCEPTION 'native source publication slot binding is immutable until definitive clear' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_native_slot_guard BEFORE UPDATE OR DELETE ON game_durability_native_source_publication_slots FOR EACH ROW EXECUTE FUNCTION game_durability_native_source_slot_guard();
