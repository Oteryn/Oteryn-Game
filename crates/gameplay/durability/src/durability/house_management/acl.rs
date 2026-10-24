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
    // Canonical entries only. Legacy spell editors are not another live management truth.
    if !owner {
        let subowner:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_key='subowner' AND character_id=encode($3,'hex')::uuid)")
            .bind(world.as_bytes().as_slice()).bind(house).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if !subowner || list != HouseManagementList::Guest {
            return Err(HouseManagementError::Rejected);
        }
    }
    if expected != revision {
        return Err(HouseManagementError::Conflict);
    }
    let next = revision
        .checked_add(1)
        .ok_or(HouseManagementError::Conflict)?;
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
