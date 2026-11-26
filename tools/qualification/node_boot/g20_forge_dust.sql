-- G20 input fixture for the disposable node-boot database. All dust-chain
-- constraints stay enabled. The offline q_g20 character must have no dust row.
WITH actor AS (
    SELECT character_id
    FROM game_character_roots
    WHERE character_id = encode($1, 'hex')::uuid
      AND account_id = '01934f10-7c00-7000-8000-000000000020'::uuid
      AND world_id = encode($2, 'hex')::uuid
      AND NOT EXISTS (
          SELECT 1 FROM game_durability_reconnect_sessions s
          WHERE s.character_id = game_character_roots.character_id AND s.session_state <> 3
      )
    FOR UPDATE
), credit AS (
    INSERT INTO game_character_forge_dust_entries (
        entry_id, character_id, kind, cause, cause_occurrence_id, transaction_id,
        amount, lost_amount, balance_before, balance_after, dust_limit_before, dust_limit_after
    )
    SELECT game_character_uuid_v7(), character_id, 'GAIN', 'creature_kill',
           game_character_uuid_v7(), game_character_uuid_v7(), $3, 0, 0, $3, 100, 100
    FROM actor
    WHERE NOT EXISTS (
        SELECT 1 FROM game_character_forge_dust d WHERE d.character_id = actor.character_id
    )
    RETURNING character_id, entry_id, balance_after, dust_limit_after
)
INSERT INTO game_character_forge_dust (character_id, last_entry_id, balance, dust_limit)
SELECT character_id, entry_id, balance_after, dust_limit_after
FROM credit
RETURNING balance
