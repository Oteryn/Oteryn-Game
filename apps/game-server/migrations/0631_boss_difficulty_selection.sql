-- W21 / PR #2769 ANSWER 6542714759: durable draft choice, never a gameplay profile.
-- A context is server-issued and scoped to the exact admitted character/session/fence.
CREATE TABLE game_boss_difficulty_contexts (
    context_id UUID PRIMARY KEY,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    connection_generation NUMERIC(20,0) NOT NULL CHECK (connection_generation BETWEEN 1 AND 18446744073709551615),
    character_lease_generation NUMERIC(20,0) NOT NULL CHECK (character_lease_generation BETWEEN 1 AND 18446744073709551615),
    scope_key BYTEA NOT NULL CHECK (octet_length(scope_key) = 33),
    scope_ownership_generation NUMERIC(20,0) NOT NULL CHECK (scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    boss_key TEXT NOT NULL REFERENCES game_boss_room_catalogue(boss_key),
    content_revision TEXT NOT NULL CHECK (octet_length(content_revision) BETWEEN 1 AND 512),
    selected_difficulty SMALLINT NOT NULL DEFAULT 0 CHECK (selected_difficulty BETWEEN 0 AND 25),
    closed BOOLEAN NOT NULL DEFAULT false
);
CREATE INDEX game_boss_difficulty_contexts_character ON game_boss_difficulty_contexts(character_id);
CREATE INDEX game_boss_difficulty_contexts_boss ON game_boss_difficulty_contexts(boss_key);
CREATE UNIQUE INDEX game_boss_difficulty_contexts_one_open
    ON game_boss_difficulty_contexts(character_id) WHERE NOT closed;

CREATE TABLE game_boss_difficulty_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    connection_generation NUMERIC(20,0) NOT NULL CHECK (connection_generation BETWEEN 1 AND 18446744073709551615),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    request BYTEA NOT NULL CHECK (octet_length(request) BETWEEN 1 AND 1024),
    reply BYTEA NOT NULL CHECK (octet_length(reply) BETWEEN 1 AND 2048),
    PRIMARY KEY (character_id, game_session_id, connection_generation, command_id)
);

REVOKE ALL ON game_boss_difficulty_contexts, game_boss_difficulty_receipts FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_boss_difficulty_contexts TO oteryn_game_runtime;
-- Receipts cannot be updated or deleted by the runtime; replay returns the first committed reply.
GRANT SELECT, INSERT ON game_boss_difficulty_receipts TO oteryn_game_runtime;

-- Append to 0631: W02 native Instance casts retain the existing cost/receipt owner.
-- The physical Ground/custody owner is still Channel-only. No origin Channel is
-- substituted for an Instance; Instance receipts represent cost-only Player casts.
ALTER TABLE game_spell_item_receipts
    ADD COLUMN runtime_scope_kind SMALLINT NOT NULL DEFAULT 1,
    ADD COLUMN instance_id UUID,
    ALTER COLUMN channel_id DROP NOT NULL;

ALTER TABLE game_spell_item_receipts ADD CONSTRAINT game_spell_item_exact_runtime_scope CHECK ((
    (runtime_scope_kind = 1 AND channel_id IS NOT NULL AND instance_id IS NULL)
    OR
    (runtime_scope_kind = 2 AND channel_id IS NULL AND instance_id IS NOT NULL
        AND game_node_is_uuid_v7(instance_id)
        AND cause_kind = 0 AND spell_family = 'Spell' AND swing_ordinal IS NULL
        AND operation_count = 0
        AND convert_from(intent,'UTF8')::jsonb->>'version' = '2'
        AND convert_from(intent,'UTF8')::jsonb->>'runtime_scope_kind' = '2'
        AND convert_from(intent,'UTF8')::jsonb->'channel' = 'null'::jsonb
        AND convert_from(intent,'UTF8')::jsonb->>'instance' = replace(instance_id::text,'-','')
        AND convert_from(intent,'UTF8')::jsonb->'operations' = '[]'::jsonb
        AND convert_from(intent,'UTF8')::jsonb->'companion' = 'null'::jsonb
        AND COALESCE(convert_from(intent,'UTF8')::jsonb->'direct_companion','null'::jsonb) = 'null'::jsonb)
) IS TRUE);

-- Existing immutable receipts, exact cost/hash, line census, same-transaction audit,
-- and source-provenance triggers remain installed. No new role grant is required.
