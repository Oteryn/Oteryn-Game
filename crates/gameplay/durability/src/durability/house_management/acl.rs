use super::*;
use oteryn_gameplay_housing::house_management::{HOUSE_ACL_MEMBERS_MAX, HouseManagementList};
use std::collections::BTreeSet;

pub(super) async fn replace(
    tx: &mut Tx<'_>,
    fence: &CurrentCharacterItemFence,
    request: &HouseManagementRequest,
    owner: bool,
    (list, expected, revision): (HouseManagementList, u64, u64),
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

pub(super) async fn next_revision(
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

pub(super) async fn qualify_guild_selectors(
    tx: &mut Tx<'_>,
    request: &HouseManagementRequest,
    rules: &[String],
    ordered: bool,
) -> Result<serde_json::Value> {
    const MAX_LINES: usize = 100;
    const MAX_CHARACTERS: usize = 1999;
    if rules.len() > MAX_LINES
        || rules.iter().map(|line| line.chars().count()).sum::<usize>()
            + rules.len().saturating_sub(1)
            > MAX_CHARACTERS
    {
        return Err(HouseManagementError::Rejected);
    }

    let mut selectors = Vec::with_capacity(rules.len());
    for line in rules {
        if line.contains('\0') || (ordered && line.contains(['\n', '\r'])) {
            return Err(HouseManagementError::Rejected);
        }
        if !ordered && (line.is_empty() || line.len() > 100) {
            return Err(HouseManagementError::Rejected);
        }
        let line = line.trim();
        if ordered && (line.is_empty() || line.starts_with('#')) {
            continue;
        }
        let (pattern, deny) = if ordered {
            (
                line.strip_prefix('!').unwrap_or(line).trim(),
                line.starts_with('!'),
            )
        } else {
            (line, false)
        };
        if pattern.is_empty() {
            return Err(HouseManagementError::Rejected);
        }
        if let Some((rank, guild)) = pattern.split_once('@') {
            let (rank, guild) = (rank.trim(), guild.trim());
            if guild.is_empty() || guild.contains('@') {
                return Err(HouseManagementError::Rejected);
            }
            selectors.push(serde_json::json!({
                "guild": guild, "rank": (!(rank.is_empty() || (ordered && rank == "*"))).then_some(rank),
                "deny": deny,
            }));
        } else if ordered {
            selectors.push(serde_json::json!({ "name": pattern, "deny": deny }));
        } else {
            return Err(HouseManagementError::Rejected);
        }
    }
    let expected = selectors.len();
    let world = request.house.world_id();
    let qualified: serde_json::Value = sqlx::query_scalar(
        "WITH qualified AS (
         SELECT rule.position,
            CASE WHEN rule.selector ? 'name' THEN rule.selector ELSE
                jsonb_build_object('guild', g.guild_id, 'rank', r.level,
                    'identity', r.house_acl_identity, 'deny', rule.selector->'deny')
            END AS rule
         FROM jsonb_array_elements($2::jsonb) WITH ORDINALITY AS rule(selector, position)
         LEFT JOIN game_guilds g ON g.world_id=encode($1,'hex')::uuid
           AND g.name_key=game_character_name_key(rule.selector->>'guild') AND g.state IN(1,2)
         LEFT JOIN game_guild_ranks r ON r.guild_id=g.guild_id AND r.name_key=game_character_name_key(rule.selector->>'rank')
         WHERE rule.selector ? 'name'
            OR (g.guild_id IS NOT NULL AND (rule.selector->>'rank' IS NULL OR r.level IS NOT NULL))
         )
         SELECT CASE WHEN $3 OR count(*) = count(DISTINCT rule)
                 THEN COALESCE(jsonb_agg(rule ORDER BY position), '[]'::jsonb)
                 ELSE '[]'::jsonb END FROM qualified",
    )
    .bind(world.as_bytes().as_slice())
    .bind(serde_json::Value::Array(selectors))
    .bind(ordered)
    .fetch_one(&mut **tx)
    .await?;
    if qualified.as_array().map(Vec::len) != Some(expected) {
        return Err(HouseManagementError::Rejected);
    }
    Ok(serde_json::json!({ "rules": qualified, "lines": rules, "ordered": ordered }))
}
