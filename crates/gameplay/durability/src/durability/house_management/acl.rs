use super::*;
use oteryn_gameplay_housing::house_management::{HOUSE_ACL_MEMBERS_MAX, HouseManagementList};
use std::collections::BTreeSet;

#[allow(clippy::too_many_arguments)]
pub(super) async fn replace(
    tx: &mut Tx<'_>,
    fence: &CurrentCharacterItemFence,
    request: &HouseManagementRequest,
    _ownership: u64,
    owner: bool,
    revision: u64,
    list: HouseManagementList,
    expected: u64,
    members: &[[u8; 16]],
) -> Result<u64> {
    let key = list.key().ok_or(HouseManagementError::Rejected)?;
    if members.len() > HOUSE_ACL_MEMBERS_MAX
        || members.iter().copied().collect::<BTreeSet<_>>().len() != members.len()
        || members
            .iter()
            .any(|id| id[6] >> 4 != 7 || id[8] & 0xc0 != 0x80)
    {
        return Err(HouseManagementError::Rejected);
    }
    let world = request.house.world_id();
    let house = request.house.house_key();
    let next = next_revision(tx, fence, request, owner, list, expected, revision).await?;
    let members: Vec<String> = members
        .iter()
        .map(|id| id.iter().map(|b| format!("{b:02x}")).collect())
        .collect();
    let valid:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM unnest($1::text[]) m WHERE NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=m::uuid AND r.world_id=encode($2,'hex')::uuid AND r.lifecycle=1))")
        .bind(&members).bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(HouseManagementError::Rejected);
    }
    sqlx::query("DELETE FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_key=$3")
        .bind(world.as_bytes().as_slice()).bind(house).bind(&key).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_key,character_id) SELECT encode($1,'hex')::uuid,$2,$3,m::uuid FROM unnest($4::text[]) m")
        .bind(world.as_bytes().as_slice()).bind(house).bind(key).bind(&members).execute(&mut **tx).await?;
    Ok(next)
}

async fn next_revision(
    tx: &mut Tx<'_>,
    fence: &CurrentCharacterItemFence,
    request: &HouseManagementRequest,
    owner: bool,
    list: HouseManagementList,
    expected: u64,
    revision: u64,
) -> Result<u64> {
    if !owner {
        if list != HouseManagementList::Guest {
            return Err(HouseManagementError::Rejected);
        }
        let subowner: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid) WHERE role='SUBOWNER' AND NOT content_fenced)",
        )
        .bind(request.house.world_id().as_bytes().as_slice())
        .bind(request.house.house_key())
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_one(&mut **tx).await?;
        if !subowner {
            return Err(HouseManagementError::Rejected);
        }
    }
    if expected != revision {
        return Err(HouseManagementError::Conflict);
    }
    revision
        .checked_add(1)
        .ok_or(HouseManagementError::Conflict)
}

pub(super) async fn replace_guild(
    tx: &mut Tx<'_>,
    fence: &CurrentCharacterItemFence,
    request: &HouseManagementRequest,
    owner: bool,
    (list, expected, revision, rules): (HouseManagementList, u64, u64, &[String]),
    ownership: u64,
) -> Result<u64> {
    if !matches!(
        list,
        HouseManagementList::Guest | HouseManagementList::Subowner
    ) || rules.len() > 100
    {
        return Err(HouseManagementError::Rejected);
    }
    let next = next_revision(tx, fence, request, owner, list, expected, revision).await?;
    let mut guilds = Vec::with_capacity(rules.len());
    let mut ranks = Vec::with_capacity(rules.len());
    for line in rules {
        let (rank, guild) = line
            .trim()
            .split_once('@')
            .ok_or(HouseManagementError::Rejected)?;
        let (rank, guild) = (rank.trim(), guild.trim());
        if line.len() > 100 || guild.is_empty() || guild.contains('@') {
            return Err(HouseManagementError::Rejected);
        }
        guilds.push(guild);
        ranks.push((!rank.is_empty()).then_some(rank));
    }
    let world = request.house.world_id();
    let qualified: serde_json::Value = sqlx::query_scalar(
        "SELECT COALESCE(jsonb_agg(DISTINCT jsonb_build_object('guild', g.guild_id, 'rank', r.level, 'name', r.name_key)), '[]'::jsonb)
         FROM unnest($2::text[], $3::text[]) AS rule(guild, rank)
         JOIN game_guilds g ON g.world_id=encode($1,'hex')::uuid
           AND g.name_key=game_character_name_key(rule.guild) AND g.state IN(1,2)
         LEFT JOIN game_guild_ranks r ON r.guild_id=g.guild_id AND r.name_key=game_character_name_key(rule.rank)
         WHERE rule.rank IS NULL OR r.level IS NOT NULL",
    )
    .bind(world.as_bytes().as_slice()).bind(&guilds).bind(&ranks)
    .fetch_one(&mut **tx).await?;
    if qualified.as_array().map(Vec::len) != Some(rules.len()) {
        return Err(HouseManagementError::Rejected);
    }
    sqlx::query(
        "INSERT INTO game_house_guild_acl(world_id,house_key,list_key,ownership_generation,rules,last_session_id,last_command_id)
         VALUES(encode($1,'hex')::uuid,$2,$3,$4::text::numeric,$5,encode($6,'hex')::uuid,$7::text::numeric)
         ON CONFLICT(world_id,house_key,list_key) DO UPDATE SET ownership_generation=excluded.ownership_generation,
           rules=excluded.rules,last_session_id=excluded.last_session_id,last_command_id=excluded.last_command_id",
    )
    .bind(world.as_bytes().as_slice()).bind(request.house.house_key()).bind(list.key())
    .bind(ownership.to_string()).bind(qualified).bind(fence.game_session_id.as_bytes().as_slice())
    .bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
    Ok(next)
}
