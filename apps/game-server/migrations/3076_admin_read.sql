-- ADM-G3 curates Game data for an independently provisioned read login.
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'oteryn_game_admin_read') THEN
        CREATE ROLE oteryn_game_admin_read NOLOGIN;
    END IF;
EXCEPTION WHEN duplicate_object OR unique_violation THEN
    -- Roles are cluster-wide and another Game database may be migrating.
    NULL;
END $$;

CREATE SCHEMA admin_read;
REVOKE ALL ON SCHEMA admin_read FROM PUBLIC;

CREATE VIEW admin_read.character_roots WITH (security_barrier = true) AS
SELECT r.character_id, r.account_id, r.world_id, r.name, r.name_key, r.creation_vocation,
       EXISTS (SELECT 1 FROM public.game_durability_reconnect_sessions s
               WHERE s.character_id = r.character_id AND s.session_state IN (1, 2))
       OR EXISTS (SELECT 1 FROM public.game_durability_admission_character_guards c
                  WHERE c.character_id = r.character_id AND c.holder_game_session_id IS NOT NULL)
       OR EXISTS (SELECT 1 FROM public.game_durability_admission_account_guards a
                  WHERE a.account_id = r.account_id AND a.presence_character_id = r.character_id) AS busy
FROM public.game_character_roots r
WHERE r.lifecycle = 1
  -- Same detachment predicate as game_account_closure_detached. Keep access to
  -- the closure records and function out of the read role.
  AND NOT EXISTS (
      SELECT 1 FROM public.game_account_closure_members m
      WHERE m.character_id = r.character_id AND
          (m.step IN (1, 3) OR (m.step = 2 AND EXISTS (
              SELECT 1 FROM public.game_account_closures c
              WHERE c.account_id = m.account_id AND c.close_operation_id = m.close_operation_id
                AND c.state <> 'CANCELLED'))));

CREATE VIEW admin_read.online_characters AS
SELECT s.character_id, s.world_id
FROM public.game_durability_reconnect_sessions s
JOIN public.game_durability_admission_character_guards c ON c.character_id = s.character_id
JOIN public.game_durability_admission_account_guards a ON a.account_id = s.account_id
JOIN admin_read.character_roots r ON r.character_id = s.character_id
    AND r.world_id = s.world_id AND r.account_id = s.account_id
WHERE s.session_state = 2 AND s.current_transport_ref IS NOT NULL
  AND c.eligible AND c.world_id = s.world_id AND c.account_id = s.account_id
  AND c.lease_generation = s.character_lease_generation AND c.holder_game_session_id = s.game_session_id
  AND a.presence_character_id = s.character_id AND a.holder_game_session_id = s.game_session_id;

CREATE VIEW admin_read.character_progression AS
SELECT character_id, level FROM public.game_character_progression_state;
CREATE VIEW admin_read.character_build AS
SELECT character_id, vocation, magic_level, fist_level, club_level, sword_level,
       axe_level, distance_level, shielding_level, fishing_level
FROM public.game_character_build_state;
CREATE VIEW admin_read.character_positions AS
SELECT character_id, world_id, tile_x, tile_y, tile_floor FROM public.game_character_last_positions;
CREATE VIEW admin_read.bank_balances AS
SELECT account_id, world_id, balance FROM public.game_account_bank_balances;
CREATE VIEW admin_read.premium_account_fences AS
SELECT account_id, authority_revision, conflict_authority_revision FROM public.game_premium_account_fence;
CREATE VIEW admin_read.premium_evidence AS
SELECT account_id, authority_revision, entitlement_state, effective_from_us, effective_until_us,
       authority_issued_at_us, authority_valid_until_us FROM public.game_premium_evidence;
CREATE VIEW admin_read.guild_members AS
SELECT guild_id, character_id FROM public.game_guild_members;
CREATE VIEW admin_read.guilds AS
SELECT guild_id, world_id, name, state FROM public.game_guilds;
CREATE VIEW admin_read.house_properties AS
SELECT world_id, owner_character_id, house_key FROM public.game_house_properties;
CREATE VIEW admin_read.online_intervals AS
SELECT character_id, owner_account_id, world_id, state, withheld, expires_ms, started_ms, statistics_id
FROM public.game_character_online_intervals;

CREATE VIEW admin_read.container_slots AS
SELECT character_id, world_id, item_instance_id FROM public.game_item_container_slots;
CREATE VIEW admin_read.equipment_slots AS
SELECT character_id, world_id, item_instance_id, slot FROM public.game_character_equipment_slots;
CREATE VIEW admin_read.depot_locations AS
SELECT character_id, world_id, item_instance_id FROM public.game_item_character_depot_locations;
CREATE VIEW admin_read.inbox_locations AS
SELECT character_id, world_id, item_instance_id FROM public.game_item_character_inbox_locations;
CREATE VIEW admin_read.container_entries AS
SELECT character_id, world_id, item_instance_id, parent_item_instance_id FROM public.game_item_container_entries;
CREATE VIEW admin_read.nested_entries AS
SELECT world_id, item_instance_id, parent_item_instance_id FROM public.game_item_nested_entries;
CREATE VIEW admin_read.item_instances AS
SELECT item_instance_id, world_id, lifecycle, definition_production_key, quantity FROM public.game_item_instances;
CREATE VIEW admin_read.stash_balances AS
SELECT character_id, world_id, quantity,
       CASE WHEN type_id <= 4294967295 THEN type_id ELSE 0 END AS type_id
FROM (
    SELECT character_id, world_id, quantity, coalesce(
        substring(definition_key FROM '^oteryn:item[.](?:tibia[.])?i([0-9]{1,10})$')::bigint, 0
    ) AS type_id
    FROM public.game_character_stash_balances
    WHERE quantity > 0
) balances;

CREATE VIEW admin_read.sanctions AS
SELECT 'platform/' || p.sanction_stream_id AS key, p.sanction_stream_id AS id,
       p.character_id, p.account_id, r.world_id,
       (ARRAY['mute','jail','character_ban','account_ban','namelock','notation'])[p.kind] AS kind,
       p.starts_at_ms, p.ends_at_ms, p.revoked AS lifted
FROM public.game_account_sanction_projections p
LEFT JOIN admin_read.character_roots r ON r.character_id = p.character_id AND r.account_id = p.account_id
WHERE p.character_id IS NULL OR r.character_id IS NOT NULL
UNION ALL
SELECT 'staff/' || s.sanction_id || '/' || s.environment, s.sanction_id,
       s.character_id, r.account_id, r.world_id, 'mute',
       s.starts_at_ms, s.ends_at_ms, s.lifted_at_ms IS NOT NULL
FROM public.game_character_sanctions s JOIN admin_read.character_roots r USING (character_id)
UNION ALL
-- The originating Character in a Game staff ban is audit provenance.
-- Admission enforces the ban against the account, including other Characters.
SELECT 'staff-ban/' || b.sanction_id || '/' || b.environment, b.sanction_id,
       NULL::uuid, b.account_id, NULL::uuid, 'account_ban',
       b.starts_at_ms, b.ends_at_ms, b.lifted_at_ms IS NOT NULL
FROM public.game_staff_bans b;

CREATE VIEW admin_read.node_registrations AS
SELECT node_id, registration_revision, state, registered_at FROM public.game_node_registrations;
CREATE VIEW admin_read.runtime_scope_assignments AS
SELECT world_id, channel_id, scope_kind, scope_key, state, holder_node_id,
       holder_registration_revision, ownership_generation FROM public.game_runtime_scope_assignments;
CREATE VIEW admin_read.runtime_guards AS
SELECT scope_key, ownership_generation, ready FROM public.game_durability_admission_runtime_guards;
CREATE VIEW admin_read.migrations AS
SELECT version, description, success, checksum FROM public._sqlx_migrations;
CREATE VIEW admin_read.ledger AS
SELECT ledger_id, request_id, admin_operation_id, operation, admin_id, risk_class, target,
       status, error_code, started_at, finished_at FROM public.admin_command_ledger;

REVOKE ALL ON ALL TABLES IN SCHEMA admin_read FROM PUBLIC;
GRANT USAGE ON SCHEMA admin_read TO oteryn_game_admin_read;
GRANT SELECT ON ALL TABLES IN SCHEMA admin_read TO oteryn_game_admin_read;
