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
    FROM game_character_roots AS owner
    WHERE owner.character_id = encode($3, 'hex')::uuid
      -- Reuse only the exact fixture and its intact ownership and slot.
      -- Any different property at this key still reaches the PK constraint.
      AND NOT EXISTS (
          SELECT 1 FROM game_house_properties AS existing
          JOIN game_house_ownership AS anchor USING (world_id, house_key)
          JOIN game_account_housing_slots AS housing USING (world_id, house_key)
          WHERE existing.world_id = encode($1, 'hex')::uuid AND existing.house_key = $2
            AND existing.kind = 'private_house' AND existing.state = 3
            AND existing.rent_gold = 0 AND existing.content_digest = decode(repeat('07', 32), 'hex')
            AND existing.owner_character_id = owner.character_id
            AND existing.owner_account_id = owner.account_id
            AND existing.ownership_generation = 1 AND existing.revision = 1
            AND existing.acl_revision = 1 AND existing.content_fence_operation IS NULL
            AND existing.paid_until > statement_timestamp()
            AND anchor.owner_character_id = owner.character_id
            AND anchor.ownership_revision = existing.ownership_generation
            AND anchor.acquisition_receipt_ref = existing.acquisition_receipt_ref
            AND housing.account_id = owner.account_id
      )
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
