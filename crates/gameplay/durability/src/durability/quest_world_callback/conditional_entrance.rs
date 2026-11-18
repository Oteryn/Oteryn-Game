//! Conditional entrances observe existing Character owners without writing progress.
use super::*;
use crate::durability::{
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::{CharacterProgressionError, assert_item_only_fence},
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    quest_state::soul_war::{SoulWarState, storage},
    runtime_scope_assignment::NodeIncarnationProof,
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    world_callbacks::conditional_entrance::{Entrance, MonthDay, Predicate, StorageSource},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Default)]
pub struct ReadSet {
    tracks: BTreeSet<String>,
    carried: BTreeSet<String>,
    soul_war: bool,
    calendar: bool,
}

impl ReadSet {
    pub fn for_entrance(entrance: &Entrance) -> Self {
        let mut reads = Self::default();
        for guard in &entrance.guards {
            guard.when.visit(&mut |p| {
                if let Predicate::Storage { range } = p {
                    reads.storage(&range.source);
                }
                if let Predicate::DistinctFlags { flags, .. } = p {
                    for flag in flags {
                        reads.storage(&flag.range.source);
                    }
                }
                if let Predicate::Carried { item, .. } = p {
                    reads.carried.insert(item.clone());
                }
                reads.calendar |= matches!(p, Predicate::Calendar { .. });
            });
        }
        reads
    }

    fn storage(&mut self, source: &StorageSource) {
        if let StorageSource::Track { track } = source {
            self.tracks.insert(track.clone());
        } else {
            self.soul_war = true;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty() && self.carried.is_empty() && !self.soul_war && !self.calendar
    }
}

#[derive(Default)]
pub struct Facts {
    pub tracks: BTreeMap<String, i64>,
    pub carried: BTreeMap<String, i64>,
    pub soul_war: SoulWarState,
    pub date: Option<MonthDay>,
}

impl Facts {
    pub fn storage(&self, source: &StorageSource) -> Option<i64> {
        match source {
            StorageSource::Track { track } => self.tracks.get(track).copied(),
            StorageSource::SoulWarBoss { boss } => {
                Some(i64::from(self.soul_war.defeated_bosses & boss.bit() != 0))
            }
            StorageSource::SoulWarCompleted => Some(i64::from(self.soul_war.megalomania_completed)),
        }
    }
}

impl super::super::DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "read the entrance inputs under the same Character and World fence as other callbacks"
    )]
    pub async fn read_conditional_entrance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        holder: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        catalogue: Option<Arc<QuestStateCatalogue>>,
        permit: &SpellLanePermit,
        reads: ReadSet,
        world: QuestWorldContext,
    ) -> Result<Facts, CharacterProgressionError> {
        let holder = holder.clone();
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let mut context = (permit, world);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |db, deadline, (permit, world)| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(db, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if let Err(error) = assert_item_only_fence(&mut tx, &fence.item_fence(), &holder).await? {
                    return Ok(Err(error));
                }
                if !context_is_current(&mut tx, permit, fence, world).await? {
                    return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                }
                let mut facts = Facts::default();
                let id = fence.character_id.as_bytes().as_slice();
                if !reads.tracks.is_empty() {
                    let mut tracks = Vec::with_capacity(reads.tracks.len());
                    for key in reads.tracks {
                        let Some(track) = catalogue.as_ref().and_then(|c| c.track(&key)) else {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        };
                        facts.tracks.insert(key.clone(), track.initial);
                        tracks.push(key);
                    }
                    let rows: Vec<(String, i64)> = sqlx::query_as(
                        "SELECT track_key,value FROM game_character_quest_tracks                          WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2)",
                    ).bind(id).bind(&tracks).fetch_all(&mut *tx).await?;
                    facts.tracks.extend(rows);
                }
                if !reads.carried.is_empty() {
                    let items: Vec<_> = reads.carried.into_iter().collect();
                    let rows: Vec<(String, i64)> = sqlx::query_as(
                        "SELECT i.definition_production_key,least(sum(i.quantity),100)::bigint                            FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c                            JOIN game_item_instances i ON i.item_instance_id=c.item                           WHERE i.lifecycle=1 AND i.quantity>0 AND i.definition_family='Item'                             AND i.definition_production_key=ANY($3) GROUP BY i.definition_production_key",
                    ).bind(id).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                        .bind(&items).fetch_all(&mut *tx).await?;
                    facts.carried.extend(rows);
                }
                if reads.soul_war {
                    facts.soul_war = storage::load(&mut tx, fence.character_id).await?;
                }
                if reads.calendar {
                    // Use one observation of the server's UTC calendar for both
                    // Chyllfroest callbacks (movements_prohibit_entry.lua:9-15,
                    // actions_reward.lua:4-8).
                    let (month, day): (i16, i16) = sqlx::query_as(
                        "SELECT extract(month FROM d)::smallint,extract(day FROM d)::smallint                            FROM (SELECT transaction_timestamp() AT TIME ZONE 'UTC' AS d) sample",
                    ).fetch_one(&mut *tx).await?;
                    facts.date = Some(MonthDay(
                        month.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
                        day.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
                    ));
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(facts))
            })
        }).await?
    }
}
