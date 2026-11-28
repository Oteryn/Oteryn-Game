INSERT INTO game_character_account_guards(account_id)
VALUES ('00000000-0000-7000-8000-000000000002');
INSERT INTO game_character_roots(character_id, account_id, world_id, lifecycle, character_revision,
    profile_revision, ruleset_revision, content_revision, starter_template_revision, name)
VALUES ('00000000-0000-7000-8000-000000000001', '00000000-0000-7000-8000-000000000002',
    '00000000-0000-7000-8000-000000000003', 1, 1, 'profile-1', 'ruleset-1', 'content-1',
    'starter-1', 'Legacy Forge');
INSERT INTO game_content_activations(world_id, channel_id, activation_sequence,
    server_artifact_digest, client_artifact_digest, frame_binding_digest, issued_by, issued_at)
VALUES ('00000000-0000-7000-8000-000000000003', '00000000-0000-7000-8000-000000000004',
    1, decode(repeat('07',32),'hex'), decode(repeat('08',32),'hex'),
    decode(repeat('09',32),'hex'), session_user, 1800000000000);
INSERT INTO game_character_forge_dust_entries(entry_id, character_id, kind, cause,
    cause_occurrence_id, transaction_id, amount, lost_amount, balance_before, balance_after,
    dust_limit_before, dust_limit_after)
VALUES ('00000000-0000-7000-8000-000000000009', '00000000-0000-7000-8000-000000000001',
    'GAIN', 'creature_kill', '00000000-0000-7000-8000-00000000000a',
    '00000000-0000-7000-8000-00000000000b', 100, 0, 0, 100, 100, 100);
INSERT INTO game_forge_dust_limit_reservations(character_id, occurrence_id, world_id,
    channel_id, transaction_id, event_id, entry_id, occurred_at, expected_upgrade_count,
    content_digest, ruleset_revision, simulation_profile_revision, work_units_used)
VALUES ('00000000-0000-7000-8000-000000000001', '00000000-0000-7000-8000-000000000005',
    '00000000-0000-7000-8000-000000000003', '00000000-0000-7000-8000-000000000004',
    '00000000-0000-7000-8000-000000000006', '00000000-0000-7000-8000-000000000007',
    '00000000-0000-7000-8000-000000000008', 1800000000000, 0,
    decode(repeat('07',32),'hex'), 'global-forge-v2', 1, 1);
INSERT INTO game_character_forge_dust_entries(entry_id, character_id, previous_entry_id, kind,
    cause, cause_occurrence_id, transaction_id, amount, lost_amount, balance_before,
    balance_after, dust_limit_before, dust_limit_after)
SELECT entry_id, character_id, '00000000-0000-7000-8000-000000000009', 'LIMIT_RAISE',
    'forge_dust_limit', occurrence_id, transaction_id, 25, 0, 100, 75, 100, 101
FROM game_forge_dust_limit_reservations;
INSERT INTO game_character_forge_dust(character_id, balance, dust_limit, last_entry_id)
SELECT character_id, 75, 101, entry_id FROM game_forge_dust_limit_reservations;
