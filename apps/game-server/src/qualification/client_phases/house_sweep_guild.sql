-- Give a guildhall bidder a distinct active guild, with the bidder as leader.
WITH actor AS (
    SELECT account_id, character_id, world_id, name
    FROM game_character_roots
    WHERE account_id = $1::uuid AND world_id = encode($2, 'hex')::uuid
), guild AS (
    INSERT INTO game_guilds(
        guild_id, world_id, name, state, formation_deadline, revision, created_at
    )
    SELECT uuidv7(), world_id, name, 2, 0, 1,
           floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint
    FROM actor
    RETURNING guild_id, created_at
), ranks AS (
    INSERT INTO game_guild_ranks(guild_id, level, name)
    SELECT guild_id, level, label
    FROM guild CROSS JOIN (VALUES (1, 'Leader'), (2, 'Vice Leader'), (3, 'Member')) AS rank(level, label)
    RETURNING guild_id, level
), leader AS (
    INSERT INTO game_guild_members(character_id, guild_id, rank_level, joined_at)
    SELECT actor.character_id, guild.guild_id, 1, guild.created_at
    FROM actor CROSS JOIN guild JOIN ranks USING (guild_id)
    WHERE ranks.level = 1
    RETURNING character_id, guild_id
)
INSERT INTO game_account_guild_leadership(account_id, guild_id, character_id)
SELECT actor.account_id, leader.guild_id, leader.character_id FROM actor JOIN leader USING (character_id)
