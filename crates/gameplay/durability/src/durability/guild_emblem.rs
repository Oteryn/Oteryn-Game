//! GUILD-EMBLEM-1: Tibia's observer-relative guild emblem of each viewed character, from
//! live guild membership and the World's active guild wars (state 2, as war kills use).
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_session_fence;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::guild_emblem::Emblem;
use sqlx::Row;

impl DurabilityRoot {
    /// The emblem of every character in `viewed` that is in a guild of the fence's World, seen
    /// by the fence's character. `None` when the fence is no longer current. Writes nothing.
    pub async fn read_guild_emblems(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house_origin: Option<[u8; 16]>,
        viewed: Vec<[u8; 16]>,
    ) -> Result<Option<Vec<([u8; 16], Emblem)>>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    let current = if let Some(origin) = house_origin {
                        super::db::lock_admission_relations(&mut tx).await?;
                        super::house_hireling::fence::assert_house_custody_fence(
                            &mut tx, &node, &fence, origin,
                        )
                        .await?
                        .is_some()
                    } else {
                        assert_session_fence(&mut tx, &fence, &node).await?.is_ok()
                    };
                    if !current {
                        tx.rollback().await?;
                        return Ok(None);
                    }
                    let world = fence.runtime_scope.world_id();
                    let viewed: Vec<Vec<u8>> = viewed.iter().map(|id| id.to_vec()).collect();
                    // Bounded by the Channel's positioned players passed in `viewed`.
                    let rows = sqlx::query(
                        "WITH observer AS ( \
                           SELECT m.guild_id FROM game_guild_members m \
                             JOIN game_guilds g USING (guild_id) \
                            WHERE m.character_id = encode($1,'hex')::uuid AND g.state < 4 \
                              AND g.world_id = encode($3,'hex')::uuid \
                         ) \
                         SELECT uuid_send(m.character_id) AS character, \
                                COALESCE(m.guild_id = (SELECT guild_id FROM observer), false) AS own, \
                                EXISTS (SELECT 1 FROM game_guild_wars w \
                                         WHERE w.world_id = g.world_id AND w.state = 2 \
                                           AND m.guild_id IN (w.declaring_guild, w.opposing_guild)) AS in_war, \
                                EXISTS (SELECT 1 FROM game_guild_wars w, observer o \
                                         WHERE w.world_id = g.world_id AND w.state = 2 \
                                           AND ((w.declaring_guild = m.guild_id AND w.opposing_guild = o.guild_id) \
                                             OR (w.opposing_guild = m.guild_id AND w.declaring_guild = o.guild_id))) AS enemy \
                           FROM game_guild_members m JOIN game_guilds g USING (guild_id) \
                          WHERE m.character_id IN (SELECT encode(v,'hex')::uuid FROM unnest($2::bytea[]) AS v) \
                            AND g.state < 4 AND g.world_id = encode($3,'hex')::uuid \
                          ORDER BY m.character_id",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(&viewed)
                    .bind(world.as_bytes().as_slice())
                    .fetch_all(&mut *tx)
                    .await?;
                    tx.rollback().await?;
                    let mut emblems = Vec::with_capacity(rows.len());
                    for row in rows {
                        let character: [u8; 16] = row
                            .try_get::<Vec<u8>, _>("character")?
                            .try_into()
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                        let emblem = Emblem::relation(
                            true,
                            row.try_get("own")?,
                            row.try_get("in_war")?,
                            row.try_get("enemy")?,
                        )
                        .ok_or(DurabilityError::InvalidStoredState)?;
                        emblems.push((character, emblem));
                    }
                    Ok(Some(emblems))
                })
            })
            .await
    }
}
