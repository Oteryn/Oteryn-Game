//! Cross-quest access from Crystal 201189f movements_energy_entrance.lua:18-49.
use super::*;
use crate::durability::character_progression::assert_item_only_fence;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthsFaction {
    Dwarves,
    Gnomes,
    Scouts,
}

impl DepthsFaction {
    const fn points(self) -> &'static str {
        match self {
            Self::Dwarves => "oteryn:quest-progress/quest/u11_50/dangerous_depths/dwarves/points",
            Self::Gnomes => "oteryn:quest-progress/quest/u11_50/dangerous_depths/gnomes/points",
            Self::Scouts => "oteryn:quest-progress/quest/u11_50/dangerous_depths/scouts/points",
        }
    }
}

impl DurabilityRoot {
    /// Reads the current faction threshold under the same Character root lock
    /// as quest writers. Missing points are zero and missing flags grant no access.
    pub async fn check_depths_faction_access(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        faction: DepthsFaction,
    ) -> Result<bool> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await? {
                        return Ok(Err(error));
                    }
                    let (points, access): (Option<i64>, i64) = sqlx::query_as(
                        "SELECT max(value) FILTER (WHERE track_key=$2 AND quest_key='oteryn:quest.dangerous_depths_quest'), \
                                count(*) FILTER (WHERE track_key=ANY($3) AND quest_key='oteryn:quest.bigfoot_s_burden_quest' AND value=1) \
                         FROM game_character_quest_tracks \
                         WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($3 || ARRAY[$2]::text[])",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(faction.points())
                    .bind([
                        "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone1_access",
                        "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone2_access",
                        "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone3_access",
                    ].as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    let minimum = if access == 3 { 10 } else { 15 };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(points.unwrap_or_default() >= minimum))
                })
            })
            .await?
    }
}
