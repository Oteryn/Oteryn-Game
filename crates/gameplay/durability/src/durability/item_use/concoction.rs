//! W22c Wealth Duplex: one use burns one unit through the item-use writer and starts a one-hour
//! effect row in the same transaction (migration 0582). The kill settle reads the effect from
//! the same server clock; a replayed CommandRef answers from its record and never burns twice.
//! Crystal ff7ede59 `data/libs/systems/concoctions.lua:179-199`, `config.lua.dist:310-311`.
//! UNVERIFIED for Global: the donor counts the hour only while online (`config.lua.dist:312`);
//! this effect runs on the wall clock.

use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::*;
use crate::domain::CharacterId;

pub const CONCOCTION_CAUSE: u32 = 527;
pub const WEALTH_DUPLEX_KEY: &str = "oteryn:item.tibia.i36727";

/// Crystal `concoctions.lua:180-185`: refused while the last activation is inside its 24-hour
/// cooldown (`config.lua.dist:310`); nothing is burned.
pub(super) async fn admit(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
) -> Result<()> {
    if request.source_definition.production_key != WEALTH_DUPLEX_KEY {
        return Err(ItemUseError::InvalidInput);
    }
    let cooling: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_concoction_effects \
          WHERE character_id=encode($1,'hex')::uuid AND concoction='wealth_duplex' \
            AND started_at+interval '24 hours'>public.game_daily_effect_now())",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(connection)
    .await?;
    if cooling {
        return Err(ItemUseError::Cooldown);
    }
    Ok(())
}

/// The effect row of the burn `request` just wrote.
pub(super) async fn start(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO game_character_concoction_effects\
         (transaction_id,character_id,concoction,started_at,expires_at) \
         SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,'wealth_duplex',at,at+interval '1 hour' \
           FROM (SELECT public.game_daily_effect_now() AS at) instant",
    )
    .bind(request.transaction_id.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .execute(connection)
    .await?;
    Ok(())
}

impl DurabilityRoot {
    /// Whether `character_id` has a live Wealth Duplex now, on the effect clock.
    pub async fn wealth_duplex_active(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<bool> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let active: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM game_character_concoction_effects \
                          WHERE character_id=encode($1,'hex')::uuid AND concoction='wealth_duplex' \
                            AND started_at<=public.game_daily_effect_now() \
                            AND expires_at>public.game_daily_effect_now())",
                    )
                    .bind(character_id.as_bytes().as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(active))
                })
            })
            .await?
    }
}
