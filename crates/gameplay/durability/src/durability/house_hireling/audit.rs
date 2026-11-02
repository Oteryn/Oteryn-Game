//! Read-only audit attribution after the existing exact named-House authority fence.
use super::super::{DurabilityError,item_transfer::CurrentCharacterItemFence,house_scope_handoff::HouseId};
use crate::foundation::{RuntimeScopeRefV1,WorldId};
use sqlx::PgConnection;

/// A world and canonical House key determine its actual instance UUID. No origin Channel is
/// substituted for that identity. The caller already holds the session/assignment fence locks.
pub(crate) async fn read_key(connection:&mut PgConnection,fence:&CurrentCharacterItemFence)->Result<String,DurabilityError> {
    let RuntimeScopeRefV1::Instance {world_id,instance_id}=fence.runtime_scope else {return Err(DurabilityError::InvalidStoredState)};
    let house:Option<String>=sqlx::query_scalar("SELECT runtime_scope_house_key FROM game_durability_reconnect_sessions \
        WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid \
        AND runtime_scope_kind=2 AND runtime_scope_house_key IS NOT NULL AND runtime_scope_instance_id=encode($4,'hex')::uuid \
        AND scope_ownership_generation=$5::text::numeric AND current_generation=$6::text::numeric AND character_lease_generation=$7::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .fetch_optional(connection).await?;
    let house=house.ok_or(DurabilityError::InvalidStoredState)?;
    if HouseId::new(world_id,&house).is_none_or(|id|id.instance_id()!=instance_id) {return Err(DurabilityError::InvalidStoredState)}
    Ok(house)
}

pub(crate) fn valid(world:&[u8],house:&str)->bool {
    WorldId::decode(world).ok().and_then(|world|HouseId::new(world,house)).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_house_audit_refuses_generic_instance_or_channel_labels() {
        let mut world=[0;16];world[6]=0x70;world[8]=0x80;
        assert!(valid(&world,"oteryn:content.house.admiral_s_avenue_1"));
        for key in ["oteryn:activity.instance.admiral_s_avenue_1","oteryn:channel.1","oteryn:content.house.","oteryn:content.house.Bad"] {
            assert!(!valid(&world,key));
        }
        assert!(!valid(&[0;16],"oteryn:content.house.valid"));
    }
}
