//! Durable owner-only placement of the one personal banker hireling in a house.
use super::{DurabilityError, DurabilityRoot, item_transfer::CurrentCharacterItemFence};
use sqlx::Row;

impl DurabilityRoot {
    pub async fn commit_house_hireling(
        &self,
        fence: CurrentCharacterItemFence,
        house: &str,
        command: u64,
        position: Option<(u16, u16, i8)>,
        dismiss: bool,
    ) -> Result<Option<(u16, u16, i8)>, DurabilityError> {
        if house.is_empty() || command == 0 || position.is_some() == dismiss {
            return Err(DurabilityError::InvalidStoredState);
        }
        let house = house.to_owned();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
            let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_character_roots r USING(character_id) WHERE s.game_session_id=encode($1,'hex')::uuid AND s.character_id=encode($2,'hex')::uuid AND s.current_generation=$3::text::numeric AND s.character_lease_generation=$4::text::numeric AND s.scope_ownership_generation=$5::text::numeric AND s.runtime_scope_kind=2 AND s.runtime_scope_house_key=$6 AND s.session_state IN(1,2) AND r.lifecycle=1)")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string()).bind(fence.scope_ownership_generation.get().to_string()).bind(&house).fetch_one(&mut *tx).await?;
            if !current { return Err(DurabilityError::InvalidStoredState); }
            let owner: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=(SELECT world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) AND house_key=$2 AND game_house_effective_owner(world_id,house_key)=encode($1,'hex')::uuid FOR UPDATE)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&house).fetch_one(&mut *tx).await?;
            if !owner { return Err(DurabilityError::InvalidStoredState); }
            let replay = sqlx::query("SELECT active,x,y,floor FROM game_house_hireling_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).fetch_optional(&mut *tx).await?;
            let result = if let Some(row) = replay {
                if row.try_get::<bool,_>("active")? {
                    Some((u16::try_from(row.try_get::<i32,_>("x")?).map_err(|_| DurabilityError::InvalidStoredState)?, u16::try_from(row.try_get::<i32,_>("y")?).map_err(|_| DurabilityError::InvalidStoredState)?, i8::try_from(row.try_get::<i16,_>("floor")?).map_err(|_| DurabilityError::InvalidStoredState)?))
                } else { None }
            } else {
                let current: Option<(i32,i32,i16)> = if dismiss {
                    sqlx::query_as("DELETE FROM game_house_hirelings WHERE world_id=(SELECT world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) AND house_key=$2 RETURNING x,y,floor").bind(fence.character_id.as_bytes().as_slice()).bind(&house).fetch_optional(&mut *tx).await?
                } else if let Some((x,y,floor)) = position {
                    sqlx::query_as("INSERT INTO game_house_hirelings(world_id,house_key,owner_character_id,x,y,floor) VALUES((SELECT world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid),$2,encode($1,'hex')::uuid,$3,$4,$5) ON CONFLICT(world_id,house_key) DO UPDATE SET owner_character_id=EXCLUDED.owner_character_id,x=EXCLUDED.x,y=EXCLUDED.y,floor=EXCLUDED.floor RETURNING x,y,floor").bind(fence.character_id.as_bytes().as_slice()).bind(&house).bind(i32::from(x)).bind(i32::from(y)).bind(i16::from(floor)).fetch_optional(&mut *tx).await?
                } else {
                    sqlx::query_as("SELECT x,y,floor FROM game_house_hirelings WHERE world_id=(SELECT world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) AND house_key=$2").bind(fence.character_id.as_bytes().as_slice()).bind(&house).fetch_optional(&mut *tx).await?
                };
                let result = match current {
                    Some((x, y, floor)) => Some((
                        u16::try_from(x).map_err(|_| DurabilityError::InvalidStoredState)?,
                        u16::try_from(y).map_err(|_| DurabilityError::InvalidStoredState)?,
                        i8::try_from(floor).map_err(|_| DurabilityError::InvalidStoredState)?,
                    )),
                    None => None,
                };
                sqlx::query("INSERT INTO game_house_hireling_receipts(game_session_id,command_id,active,x,y,floor) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$5,$6)").bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).bind(result.is_some()).bind(result.map_or(0,|p|i32::from(p.0))).bind(result.map_or(0,|p|i32::from(p.1))).bind(result.map_or(0,|p|i16::from(p.2))).execute(&mut *tx).await?;
                result
            };
            super::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }
}
