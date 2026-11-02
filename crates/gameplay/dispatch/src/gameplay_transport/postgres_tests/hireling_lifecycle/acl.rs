//! A previously admitted House guest must still hold the current ACL at each service use.
use super::{CurrentCharacterItemFence, Harness, ReconciledCharacterAuthority, TestResult,
    HOUSE, debug, id};
use oteryn_protocol_oteryn::hireling::management::{Refusal, Reply, Request};

pub(super) async fn scenario(h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
    current: CurrentCharacterItemFence, origin: crate::foundation::ChannelId) -> TestResult {
    let landlord = id(121);
    let hireling = id(122);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    // Historical fixture of a settled ownership transfer. The viewer's real
    // admitted session remains unchanged and its guest access is explicit.
    let landlord_root: String = sqlx::query_scalar("SELECT (to_jsonb(r)|| \
          jsonb_build_object('character_id',encode($2,'hex')::uuid,'name','Fixture Landlord'))::text \
        FROM game_character_roots r WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(landlord.as_slice()).fetch_one(&mut *tx).await?;
    super::insert_character_root_copy(&mut tx, &landlord_root).await?;
    sqlx::query("UPDATE game_house_properties SET owner_character_id=encode($1,'hex')::uuid,ownership_generation=ownership_generation+1,revision=revision+1,acl_revision=acl_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(landlord.as_slice()).bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid,ownership_revision=ownership_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(landlord.as_slice()).bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_key,character_id) VALUES(encode($1,'hex')::uuid,$2,'guest',encode($3,'hex')::uuid)")
        .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_hirelings(hireling_id,world_id,owner_character_id,name,sex,look_type,active,house_key,x,y,floor,legacy_grant) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,'Guest Banker',1,1108,TRUE,$4,32310,32805,-7,TRUE)")
        .bind(hireling.as_slice()).bind(id(42).as_slice()).bind(landlord.as_slice()).bind(HOUSE)
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_hireling_entitlements(character_id,entitlement_id) VALUES(encode($1,'hex')::uuid,1001)")
        .bind(landlord.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;

    let read = h.root.load_house_hireling_service(authority, &h.node, current,
        *origin.as_bytes(), hireling).await.map_err(debug)?.ok_or("admitted guest service")?;
    assert_eq!(read.owner, landlord);
    assert!(read.has_skill(1001), "the Hireling owner's Banker entitlement serves the guest");
    assert_access_holds_property_lock(h, current).await?;
    assert_eq!(h.root.query_hirelings(authority, &h.node, current,
        *origin.as_bytes(), 100).await.map_err(debug)?, Reply::State(Vec::new()),
        "service access does not transfer Hireling ownership");
    assert_eq!(h.root.set_hireling_outfit(authority, &h.node, current,
        *origin.as_bytes(), 101, Request::Outfit { hireling, look_type: 1108,
            colors: [1, 2, 3, 4] }).await.map_err(debug)?, Reply::Refused(Refusal::NotOwner));

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND character_id=encode($3,'hex')::uuid")
        .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_properties SET revision=revision+1,acl_revision=acl_revision+1 WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(h.root.load_house_hireling_service(authority, &h.node, current,
        *origin.as_bytes(), hireling).await.is_err(), "revocation must invalidate the unchanged session fence");
    assert!(h.root.read_house_hirelings(authority, &h.node, current,
        *origin.as_bytes()).await.is_err());
    assert_eq!(h.root.query_hirelings(authority, &h.node, current,
        *origin.as_bytes(), 102).await.map_err(debug)?, Reply::Refused(Refusal::Stale));

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_key,character_id) VALUES(encode($1,'hex')::uuid,$2,'guest',encode($3,'hex')::uuid)")
        .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_properties SET content_fence_operation=encode($1,'hex')::uuid,revision=revision+1,acl_revision=acl_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(id(123).as_slice()).bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(h.root.load_house_hireling_service(authority, &h.node, current,
        *origin.as_bytes(), hireling).await.is_err(), "content fencing remains effective despite a current guest grant");
    Ok(())
}

/// An ACL writer acquires this same property lock before changing a grant.
/// The current runtime service check must retain it through its transaction,
/// rather than releasing authority between validation and the native write.
async fn assert_access_holds_property_lock(h: &Harness,
    current: CurrentCharacterItemFence) -> TestResult {
    let mut service = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *service).await?;
    let node = h.node.fact();
    let allowed: bool = sqlx::query_scalar(
        "SELECT game_hireling_service_access_lock(encode($1,'hex')::uuid,encode($2,'hex')::uuid, \
          $3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid, \
          $7::text::numeric,encode($8,'hex')::uuid,$9)",
    ).bind(current.game_session_id.as_bytes().as_slice())
     .bind(current.character_id.as_bytes().as_slice())
     .bind(current.connection_generation.get().to_string())
     .bind(current.character_lease_generation.to_string())
     .bind(current.scope_ownership_generation.get().to_string())
     .bind(node.node_id().as_bytes().as_slice())
     .bind(node.registration_revision().to_string())
     .bind(current.runtime_scope.world_id().as_bytes().as_slice()).bind(HOUSE)
     .fetch_one(&mut *service).await?;
    assert!(allowed, "the genuine already-admitted guest keeps current native access");

    let mut revocation = h.pool.begin().await?;
    sqlx::query("SET LOCAL lock_timeout = '50ms'").execute(&mut *revocation).await?;
    let lock = sqlx::query("SELECT 1 FROM game_house_properties \
        WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE")
        .bind(current.runtime_scope.world_id().as_bytes().as_slice()).bind(HOUSE)
        .fetch_optional(&mut *revocation).await;
    let Err(error) = lock else {
        return Err("revocation overtook an in-flight service's ACL proof".into());
    };
    assert_eq!(error.as_database_error().and_then(|error| error.code()).as_deref(), Some("55P03"),
        "the real competing property lock is blocked, not rejected for a different reason");
    revocation.rollback().await?;
    service.rollback().await?;
    Ok(())
}
