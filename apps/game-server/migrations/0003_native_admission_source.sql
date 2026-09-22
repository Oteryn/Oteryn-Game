-- Durable, non-rollback bootstrap for the single native admission evidence source.
CREATE TABLE game_durability_native_source_registration (
    registration_id SMALLINT PRIMARY KEY CHECK (registration_id = 1),
    bootstrap_namespace TEXT NOT NULL CHECK (bootstrap_namespace <> ''),
    bootstrap_provenance TEXT NOT NULL CHECK (bootstrap_provenance <> ''),
    descriptor_revision NUMERIC(20, 0) NOT NULL CHECK (descriptor_revision BETWEEN 1 AND 18446744073709551615),
    descriptor_facts BYTEA NOT NULL CHECK (octet_length(descriptor_facts) > 0),
    initialized_at BIGINT NOT NULL CHECK (initialized_at >= 0)
);

CREATE TABLE game_durability_native_source_descriptor_history (
    registration_id SMALLINT NOT NULL REFERENCES game_durability_native_source_registration(registration_id),
    descriptor_revision NUMERIC(20, 0) NOT NULL CHECK (descriptor_revision BETWEEN 1 AND 18446744073709551615),
    descriptor_facts BYTEA NOT NULL CHECK (octet_length(descriptor_facts) > 0),
    installed_at BIGINT NOT NULL CHECK (installed_at >= 0),
    PRIMARY KEY (registration_id, descriptor_revision)
);

CREATE TABLE game_durability_native_source_floors (
    registration_id SMALLINT NOT NULL REFERENCES game_durability_native_source_registration(registration_id),
    source_authority TEXT NOT NULL CHECK (source_authority <> ''),
    operation TEXT NOT NULL CHECK (operation <> ''),
    semantic_namespace TEXT NOT NULL CHECK (semantic_namespace <> ''),
    source_revision NUMERIC(20, 0) NOT NULL CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL CHECK (decision_identity <> ''),
    observed_at BIGINT NOT NULL CHECK (observed_at >= 0),
    semantic_facts BYTEA NOT NULL CHECK (octet_length(semantic_facts) > 0),
    PRIMARY KEY (registration_id, source_authority, semantic_namespace)
);

CREATE TABLE game_durability_native_source_observation_history (
    registration_id SMALLINT NOT NULL,
    source_authority TEXT NOT NULL,
    operation TEXT NOT NULL,
    semantic_namespace TEXT NOT NULL,
    source_revision NUMERIC(20, 0) NOT NULL CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL CHECK (decision_identity <> ''),
    observed_at BIGINT NOT NULL CHECK (observed_at >= 0),
    semantic_facts BYTEA NOT NULL CHECK (octet_length(semantic_facts) > 0),
    PRIMARY KEY (registration_id, source_authority, semantic_namespace, source_revision),
    FOREIGN KEY (registration_id) REFERENCES game_durability_native_source_registration(registration_id)
);

CREATE TABLE game_durability_native_source_publication_slots (
    registration_id SMALLINT NOT NULL REFERENCES game_durability_native_source_registration(registration_id),
    slot_id SMALLINT NOT NULL CHECK (slot_id IN (1, 2)),
    operation_binding BYTEA NULL,
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
       OR NEW.initialized_at <> OLD.initialized_at
       OR NEW.descriptor_revision <= OLD.descriptor_revision THEN
        RAISE EXCEPTION 'native source registration cannot roll back or be recreated' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_native_registration_guard BEFORE UPDATE OR DELETE ON game_durability_native_source_registration FOR EACH ROW EXECUTE FUNCTION game_durability_native_source_registration_guard();

CREATE FUNCTION game_durability_native_source_floor_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE' OR NEW.registration_id <> OLD.registration_id
       OR NEW.source_authority <> OLD.source_authority
       OR NEW.semantic_namespace <> OLD.semantic_namespace
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
