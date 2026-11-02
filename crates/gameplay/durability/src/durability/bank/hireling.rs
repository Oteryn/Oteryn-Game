//! Narrow named-House admission for the existing NPC Bank owner.
use super::*;

pub(super) async fn acting_npc_fence(
    tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,node:&NodeIncarnationProof,
    fence:&CurrentCharacterItemFence,key:Option<&str>,
)->std::result::Result<Option<Acting>,DurabilityError>{
    if matches!(fence.runtime_scope,RuntimeScopeRefV1::Channel{..}) {
        // A Hireling is never a Channel NPC, even if it happens to share coordinates.
        if key.and_then(super::super::house_hireling::npc_identity).is_some(){return Ok(None)}
        return acting_fence(tx,node,fence).await;
    }
    let Some(id)=key.and_then(super::super::house_hireling::npc_identity) else{return Ok(None)};
    let Some(session)=sqlx::query("SELECT account_id::text,origin_channel_id::text AS origin,runtime_scope_house_key AS house FROM game_durability_reconnect_sessions \
        WHERE game_session_id=encode($1,'hex')::uuid AND runtime_scope_kind=2 AND runtime_scope_house_key IS NOT NULL FOR SHARE")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await? else{return Ok(None)};
    let origin=uuid_text(&session.try_get::<String,_>("origin")?)?;
    if !super::super::house_hireling::fence::assert_house_inventory_fence(tx,node,fence,origin).await? {return Ok(None)}
    let banker:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_hirelings h \
        JOIN game_durability_reconnect_sessions s ON s.game_session_id=encode($2,'hex')::uuid \
        JOIN game_hireling_entitlements e ON e.character_id=h.owner_character_id AND e.entitlement_id=1001 \
        JOIN game_house_ownership o ON o.world_id=h.world_id AND o.house_key=h.house_key \
        WHERE h.hireling_id=encode($1,'hex')::uuid AND h.active AND h.house_key=s.runtime_scope_house_key \
          AND h.world_id=s.world_id AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id \
        FOR SHARE OF h,e,o)")
        .bind(id.as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if !banker{return Ok(None)}
    Ok(Some(Acting{character_id:fence.character_id,account_id:uuid_text(&session.try_get::<String,_>("account_id")?)?,
        world_id:fence.runtime_scope.world_id(),channel_id:ChannelId::decode(&origin).map_err(|_|DurabilityError::InvalidStoredState)?,
        scope_ownership_generation:fence.scope_ownership_generation.get(),hireling_house:Some((session.try_get("house")?,id)),
        hireling_connection_generation:Some(fence.connection_generation.get())}))
}
