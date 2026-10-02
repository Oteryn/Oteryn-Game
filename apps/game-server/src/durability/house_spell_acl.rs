//! One actual World-global House ACL owner used by GUI and Aleta. Allocation is
//! separate: a spell cannot create property ownership or commercial eligibility.
use super::spell_house_abi::{HouseAccess, HouseList, HousePresenceProof};
use super::spell_item_transaction::{SpellItemAuthority, SpellItemError, check_transaction};
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HouseEditor {
    pub(crate) editor_id: [u8; 16],
    pub(crate) house_key: String,
    pub(crate) list: HouseList,
    pub(crate) ownership_revision: u64,
    pub(crate) acl_revision: u64,
    pub(crate) allow_everyone: bool,
    pub(crate) members: Vec<[u8; 16]>,
    pub(crate) text: String,
}
fn list_id(list: HouseList) -> Result<i64, SpellItemError> {
    match list {
        HouseList::Guest => Ok(-1),
        HouseList::Subowner => Ok(-2),
        HouseList::Door(id) if id > 0 => Ok(i64::from(id)),
        _ => Err(SpellItemError::Rejected("zero House door")),
    }
}
fn number(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64, SpellItemError> {
    row.try_get::<String, _>(column)?
        .parse()
        .map_err(|_| SpellItemError::Rejected("stored House revision"))
}
fn permission(access: HouseAccess, list: HouseList) -> bool {
    access == HouseAccess::Owner || access == HouseAccess::Subowner && list == HouseList::Guest
}
async fn owner_access(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    house: &str,
) -> Result<(u64, HouseAccess), SpellItemError> {
    house_access_for_character(
        tx,
        authority.runtime_scope().world_id(),
        house,
        authority.character_id_bytes(),
    )
    .await
}
pub(crate) async fn house_access_for_character(
    tx: &mut Transaction<'_, Postgres>,
    world: crate::foundation::WorldId,
    house: &str,
    character: [u8; 16],
) -> Result<(u64, HouseAccess), SpellItemError> {
    let row=sqlx::query("SELECT uuid_send(owner_character_id) AS owner,ownership_revision::text FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR SHARE")
        .bind(world.as_bytes().as_slice()).bind(house).fetch_optional(&mut **tx).await?.ok_or(SpellItemError::Rejected("House ownership unavailable"))?;
    let ownership = number(&row, "ownership_revision")?;
    if row.try_get::<Vec<u8>, _>("owner")? == character {
        return Ok((ownership, HouseAccess::Owner));
    }
    let rows=sqlx::query("SELECT list_id,allow_everyone,encode($3,'hex')::uuid=ANY(members) AS member FROM game_house_acl WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_id IN(-1,-2) ORDER BY list_id FOR SHARE")
        .bind(world.as_bytes().as_slice()).bind(house).bind(character.as_slice()).fetch_all(&mut **tx).await?;
    let mut access = HouseAccess::NotInvited;
    for row in rows {
        if row.try_get::<bool, _>("allow_everyone")? || row.try_get::<bool, _>("member")? {
            access = access.max(if row.try_get::<i64, _>("list_id")? == -2 {
                HouseAccess::Subowner
            } else {
                HouseAccess::Guest
            });
        }
    }
    Ok((ownership, access))
}
fn check_presence(
    authority: &SpellItemAuthority,
    presence: &(impl HousePresenceProof + ?Sized),
) -> Result<(), SpellItemError> {
    if presence.world() != authority.runtime_scope().world_id()
        || presence.actor_session() != authority.game_session_id()
        || presence.scope_generation() != authority.scope_generation()
        || presence.content_digest() != authority.compatible_content_digest()
    {
        return Err(SpellItemError::Rejected("current House presence mismatch"));
    }
    Ok(())
}
pub(crate) async fn open_editor_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    presence: &(impl HousePresenceProof + ?Sized),
    list: HouseList,
) -> Result<HouseEditor, SpellItemError> {
    check_transaction(tx, authority).await?;
    check_presence(authority, presence)?;
    if let HouseList::Door(id) = list {
        if !presence.has_door(id) {
            return Err(SpellItemError::Rejected("unqualified House door"));
        }
    }
    open_editor_for_property(tx, authority, presence.house_key(), list, None).await
}

/// GUI management of an actual allocated property need not manufacture physical
/// interior presence. Door editors still need the qualified placement proof.
pub(crate) async fn open_house_gui_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    house: &str,
    list: HouseList,
) -> Result<HouseEditor, SpellItemError> {
    check_transaction(tx, authority).await?;
    if matches!(list, HouseList::Door(_)) {
        return Err(SpellItemError::Rejected(
            "Door GUI requires qualified House placement",
        ));
    }
    open_editor_for_property(tx, authority, house, list, None).await
}
pub(crate) async fn restage_editor_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    presence: &(impl HousePresenceProof + ?Sized),
    list: HouseList,
    editor: [u8; 16],
) -> Result<HouseEditor, SpellItemError> {
    check_transaction(tx, authority).await?;
    check_presence(authority, presence)?;
    if editor[6] >> 4 != 7 || editor[8] & 0xc0 != 0x80 {
        return Err(SpellItemError::Rejected("editor UUID"));
    }
    if let HouseList::Door(id) = list {
        if !presence.has_door(id) {
            return Err(SpellItemError::Rejected("unqualified House door"));
        }
    }
    open_editor_for_property(tx, authority, presence.house_key(), list, Some(editor)).await
}
async fn open_editor_for_property(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    house: &str,
    list: HouseList,
    expected_editor: Option<[u8; 16]>,
) -> Result<HouseEditor, SpellItemError> {
    let (ownership, access) = owner_access(tx, authority, house).await?;
    if !permission(access, list) {
        return Err(SpellItemError::Rejected("House ACL edit denied"));
    }
    let world = authority.runtime_scope().world_id();
    let id = list_id(list)?;
    let row=sqlx::query("SELECT revision::text,allow_everyone,ARRAY(SELECT uuid_send(member) FROM unnest(members) member) AS members FROM game_house_acl WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_id=$3 FOR SHARE")
        .bind(world.as_bytes().as_slice()).bind(house).bind(id).fetch_optional(&mut **tx).await?;
    let (revision, allow_everyone, members) = if let Some(row) = row {
        (
            number(&row, "revision")?,
            row.try_get("allow_everyone")?,
            decode_members(row.try_get("members")?)?,
        )
    } else {
        (0, false, vec![])
    };
    let mut text = if allow_everyone {
        "*\n".to_owned()
    } else {
        String::new()
    };
    for member in &members {
        let row = sqlx::query("SELECT name,name_key FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR SHARE")
            .bind(member.as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
            .ok_or(SpellItemError::Rejected("stored House member identity unavailable"))?;
        let raw: String = row.try_get("name")?;
        let key: String = row.try_get("name_key")?;
        let name = crate::domain::character_name::CharacterName::parse(&raw)
            .map_err(|_| SpellItemError::Rejected("stored House member name"))?;
        if name.comparison_key() != key {
            return Err(SpellItemError::Rejected("stored House name binding"));
        }
        text.push_str(&raw);
        text.push('\n');
    }
    let command = authority.command();
    let character = authority.character_id_bytes();
    let editor:Vec<u8>=sqlx::query_scalar("INSERT INTO game_house_editors(editor_id,world_id,house_key,list_id,character_id,game_session_id,opening_command_id,ownership_revision,acl_revision,scope_generation,content_digest,expires_at) VALUES(CASE WHEN $11::bytea IS NULL THEN game_character_uuid_v7() ELSE encode($11,'hex')::uuid END,encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric(20,0),$7::text::numeric(20,0),$8::text::numeric(20,0),$9::text::numeric(20,0),$10,statement_timestamp()+interval '10 minutes') RETURNING uuid_send(editor_id)")
        .bind(world.as_bytes().as_slice()).bind(house).bind(id).bind(character.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(ownership.to_string()).bind(revision.to_string()).bind(authority.scope_generation().to_string()).bind(authority.compatible_content_digest().as_slice()).bind(expected_editor.map(|e|e.to_vec())).fetch_one(&mut **tx).await?;
    Ok(HouseEditor {
        editor_id: editor
            .try_into()
            .map_err(|_| SpellItemError::Rejected("stored editor identity"))?,
        house_key: house.into(),
        list,
        ownership_revision: ownership,
        acl_revision: revision,
        allow_everyone,
        members,
        text,
    })
}
fn decode_members(raw: Vec<Vec<u8>>) -> Result<Vec<[u8; 16]>, SpellItemError> {
    raw.into_iter()
        .map(|id| {
            let id: [u8; 16] = id
                .try_into()
                .map_err(|_| SpellItemError::Rejected("stored House member"))?;
            crate::domain::CharacterId::from_bytes(id)
                .map_err(|_| SpellItemError::Rejected("stored House member"))?;
            Ok(id)
        })
        .collect()
}
/// Resolve explicit native names in the owning World. Source comments and `*`
/// are supported; guild/rank lines refuse until an actual guild owner is admitted.
async fn resolve_list(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    text: &str,
) -> Result<(bool, Vec<[u8; 16]>), SpellItemError> {
    if text.len() > 10_000 {
        return Err(SpellItemError::Rejected("House list bound"));
    }
    let mut allow = false;
    let mut members = BTreeSet::new();
    let lines: Vec<_> = text.lines().collect();
    if lines.len() > 100 {
        return Err(SpellItemError::Rejected("House list line bound"));
    }
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "*" {
            allow = true;
            continue;
        }
        if line.contains('@') {
            return Err(SpellItemError::Rejected(
                "House guild authority unavailable",
            ));
        }
        if line.len() > 100 || line.contains(['!', '*', '?']) {
            continue;
        }
        let name = crate::domain::character_name::CharacterName::parse(line)
            .map_err(|_| SpellItemError::Rejected("invalid native House member name"))?;
        let id:Option<Vec<u8>>=sqlx::query_scalar("SELECT uuid_send(character_id) FROM game_character_roots WHERE world_id=encode($1,'hex')::uuid AND name_key=$2 AND lifecycle=1 FOR SHARE")
            .bind(authority.runtime_scope().world_id().as_bytes().as_slice()).bind(name.comparison_key()).fetch_optional(&mut **tx).await?;
        if let Some(id) = id {
            let id: [u8; 16] = id
                .try_into()
                .map_err(|_| SpellItemError::Rejected("stored House member"))?;
            members.insert(id);
        }
    }
    Ok((allow, members.into_iter().collect()))
}
pub(crate) async fn save_editor_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    editor: [u8; 16],
    text: &str,
) -> Result<(String, HouseList, u64), SpellItemError> {
    check_transaction(tx, authority).await?;
    let row=sqlx::query("SELECT house_key,list_id,ownership_revision::text,acl_revision::text,scope_generation::text,content_digest FROM game_house_editors WHERE editor_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND game_session_id=encode($3,'hex')::uuid AND world_id=encode($4,'hex')::uuid AND NOT consumed AND expires_at>statement_timestamp() FOR UPDATE")
        .bind(editor.as_slice()).bind(authority.character_id_bytes().as_slice()).bind(authority.game_session_id().as_bytes().as_slice()).bind(authority.runtime_scope().world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(SpellItemError::Rejected("House editor/session expired"))?;
    if number(&row, "scope_generation")? != authority.scope_generation()
        || row.try_get::<Vec<u8>, _>("content_digest")? != authority.compatible_content_digest()
    {
        return Err(SpellItemError::Rejected("stale House editor generation"));
    }
    let house: String = row.try_get("house_key")?;
    let id: i64 = row.try_get("list_id")?;
    let list = match id {
        -1 => HouseList::Guest,
        -2 => HouseList::Subowner,
        id if id > 0 => HouseList::Door(
            u32::try_from(id).map_err(|_| SpellItemError::Rejected("stored House door"))?,
        ),
        _ => return Err(SpellItemError::Rejected("stored House list")),
    };
    let (ownership, access) = owner_access(tx, authority, &house).await?;
    if ownership != number(&row, "ownership_revision")? || !permission(access, list) {
        return Err(SpellItemError::Rejected(
            "House access changed after editor opening",
        ));
    }
    let world = authority.runtime_scope().world_id();
    // Serialize absent and existing lists on the one actual property row before
    // reading/replacing a list. This also fences concurrent Guest/Subowner saves.
    sqlx::query("SELECT house_key FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(&house).fetch_one(&mut **tx).await?;
    let current:Option<String>=sqlx::query_scalar("SELECT revision::text FROM game_house_acl WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_id=$3 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(&house).bind(id).fetch_optional(&mut **tx).await?;
    let revision = current
        .map(|s| {
            s.parse::<u64>()
                .map_err(|_| SpellItemError::Rejected("stored House revision"))
        })
        .transpose()?
        .unwrap_or(0);
    if revision != number(&row, "acl_revision")? {
        return Err(SpellItemError::Rejected("stale House ACL revision"));
    }
    let next = revision
        .checked_add(1)
        .ok_or(SpellItemError::Rejected("House ACL revision exhausted"))?;
    let (allow, members) = resolve_list(tx, authority, text).await?;
    let members: Vec<String> = members
        .iter()
        .map(|id| id.iter().map(|b| format!("{b:02x}")).collect())
        .collect();
    let command = authority.command();
    let receipt:Vec<u8>=sqlx::query_scalar("INSERT INTO game_house_acl_receipts(receipt_id,world_id,house_key,list_id,character_id,game_session_id,command_id,ownership_revision,revision_before,revision_after,allow_everyone,members) VALUES(CASE WHEN $11::bytea IS NULL THEN game_character_uuid_v7() ELSE encode($11,'hex')::uuid END,encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric(20,0),$7::text::numeric(20,0),$8::text::numeric(20,0),$9::text::numeric(20,0),$10,ARRAY(SELECT x::uuid FROM unnest($11::text[]) x)) RETURNING uuid_send(receipt_id)")
        .bind(world.as_bytes().as_slice()).bind(&house).bind(id).bind(authority.character_id_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(ownership.to_string()).bind(revision.to_string()).bind(next.to_string()).bind(allow).bind(&members).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO game_house_acl(world_id,house_key,list_id,revision,allow_everyone,members,last_receipt) VALUES(encode($1,'hex')::uuid,$2,$3,$4::text::numeric(20,0),$5,ARRAY(SELECT x::uuid FROM unnest($6::text[]) x),encode($7,'hex')::uuid) ON CONFLICT(world_id,house_key,list_id) DO UPDATE SET revision=EXCLUDED.revision,allow_everyone=EXCLUDED.allow_everyone,members=EXCLUDED.members,last_receipt=EXCLUDED.last_receipt")
        .bind(world.as_bytes().as_slice()).bind(&house).bind(id).bind(next.to_string()).bind(allow).bind(&members).bind(receipt).execute(&mut **tx).await?;
    sqlx::query(
        "UPDATE game_house_editors SET consumed=TRUE WHERE editor_id=encode($1,'hex')::uuid",
    )
    .bind(editor.as_slice())
    .execute(&mut **tx)
    .await?;
    check_transaction(tx, authority).await?;
    Ok((house, list, next))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guest_only_subowner_and_no_zero_door() {
        assert!(permission(HouseAccess::Subowner, HouseList::Guest));
        assert!(!permission(HouseAccess::Subowner, HouseList::Subowner));
        assert!(!permission(HouseAccess::Subowner, HouseList::Door(1)));
        assert!(permission(HouseAccess::Owner, HouseList::Door(1)));
        assert!(list_id(HouseList::Door(0)).is_err());
    }
}
