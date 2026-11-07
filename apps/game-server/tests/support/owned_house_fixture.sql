-- Historical test DATA, not auction/acquisition qualification. HOUSE-1a makes
-- the property authoritative; its anchor and Account/World slot must agree at
-- commit. Bind World, house key and owner Character as $1, $2 and $3.
WITH property AS (
    INSERT INTO game_house_properties (
        world_id, house_key, kind, state, rent_gold, content_digest,
        owner_character_id, owner_account_id, acquisition_receipt_ref, paid_until
    )
    SELECT encode($1, 'hex')::uuid, $2, 'private_house', 3, 0,
        decode(repeat('07', 32), 'hex'), character_id, account_id,
        game_character_uuid_v7(), statement_timestamp() + interval '30 days'
    FROM game_character_roots WHERE character_id = encode($3, 'hex')::uuid
    RETURNING *
), slot AS (
    INSERT INTO game_account_housing_slots (account_id, world_id, house_key)
    SELECT owner_account_id, world_id, house_key FROM property
)
INSERT INTO game_house_ownership (
    world_id, house_key, owner_character_id, ownership_revision, acquisition_receipt_ref
)
SELECT world_id, house_key, owner_character_id, ownership_generation, acquisition_receipt_ref
FROM property;
