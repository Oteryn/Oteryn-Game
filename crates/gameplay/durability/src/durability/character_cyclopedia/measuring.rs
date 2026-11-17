//! MAPDISC: the Cyclopedia map discovery data and the Measuring Tibia quest progress fed by
//! the same canonical subarea discoveries. Crystal 201189f8 `discovery_system.lua:491-526`
//! (`subareaCounts`, `sendDiscoveryData`) and `:528-563` (`grantMilestones`): a main area is
//! complete when all its subareas are discovered; the percentage counts every main area.
//! Unconverted legacy progress counts nothing, as in the SRV6 title source.
use super::storage::CanonicalProgress;
use super::*;
use crate::durability::character_progression::assert_session_fence;
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use oteryn_protocol_oteryn::cyclopedia_discovery as wire;
use std::collections::BTreeMap;

/// The admitted bundle's area tree: main area -> its subareas, plus every main area count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryAreas {
    mains: BTreeMap<u32, Vec<u32>>,
    main_areas: u32,
}

impl DiscoveryAreas {
    /// The transport supplies the current admitted bundle's table, never client data.
    pub fn new(table: &oteryn_world_bundle::subarea::Table) -> Option<Self> {
        let pairs: Vec<_> = table
            .areas
            .iter()
            .filter_map(|area| area.parent.map(|parent| (parent, area.id)))
            .collect();
        let main_areas = table.areas.iter().filter(|a| a.parent.is_none()).count();
        if table.areas.len() > oteryn_world_bundle::subarea::MAX_AREAS
            || pairs.iter().any(|(parent, _)| {
                !table
                    .areas
                    .iter()
                    .any(|a| a.id == *parent && a.parent.is_none())
            })
        {
            return None;
        }
        Self::from_pairs(&pairs, u32::try_from(main_areas).ok()?)
    }

    fn from_pairs(pairs: &[(u32, u32)], main_areas: u32) -> Option<Self> {
        let mut mains: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for (parent, id) in pairs {
            mains.entry(*parent).or_default().push(*id);
        }
        let limit = u32::try_from(wire::MAX_AREAS).ok()?;
        (main_areas >= 1
            && main_areas <= limit
            && u32::try_from(mains.len()).ok()? <= main_areas
            && pairs.len() <= wire::MAX_AREAS)
            .then_some(Self { mains, main_areas })
    }

    fn discovered(progress: &CanonicalProgress, id: u32) -> bool {
        progress.subareas.get(&id).is_some_and(|p| p.discovered)
    }

    fn quest(&self, progress: &CanonicalProgress) -> wire::QuestProgress {
        let discovered = self
            .mains
            .values()
            .flatten()
            .filter(|id| Self::discovered(progress, **id))
            .count();
        let done = self
            .mains
            .values()
            .filter(|ids| !ids.is_empty() && ids.iter().all(|id| Self::discovered(progress, *id)))
            .count();
        let percent = done * 100 / self.main_areas as usize;
        let stage = match (done, percent) {
            (0, _) => 0,
            (_, 100..) => 3,
            (_, 50..) => 2,
            _ => 1,
        };
        wire::QuestProgress {
            discovered_subareas: discovered as u16,
            completed_areas: done as u16,
            main_areas: self.main_areas as u16,
            stage,
        }
    }

    fn discovery(&self, progress: &CanonicalProgress, quest: wire::QuestProgress) -> wire::Discovery {
        let mut value = wire::Discovery {
            quest,
            ..wire::Discovery::default()
        };
        for (main, ids) in &self.mains {
            let done = ids
                .iter()
                .filter(|id| Self::discovered(progress, **id))
                .count();
            let progress_percent = (done * 100 / ids.len()) as u8;
            value.mains.push(wire::MainArea {
                id: *main,
                status: if done == ids.len() {
                    wire::STATUS_COMPLETE
                } else {
                    wire::STATUS_DISCOVERING
                },
                progress: progress_percent,
            });
            for id in ids {
                if Self::discovered(progress, *id) {
                    value.discovered.push(*id);
                } else {
                    value.discoverable.push(*id);
                }
            }
        }
        value.discovered.sort_unstable();
        value.discoverable.sort_unstable();
        value
    }
}

/// Called by the advancing Cyclopedia save with the full canonical progress it stores, in the
/// same transaction as the committed receipt. A replay returns the stored receipt earlier and
/// never reaches this write.
pub(super) async fn record(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &MapRequest,
    progress: &CanonicalProgress,
) -> Result<(), DurabilityError> {
    let areas = DiscoveryAreas::from_pairs(&request.subareas, request.main_areas)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let quest = areas.quest(progress);
    sqlx::query("INSERT INTO game_character_measuring_tibia(character_id,discovered_subareas,completed_areas,main_areas,stage,occurrence) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,encode($6,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET discovered_subareas=EXCLUDED.discovered_subareas,completed_areas=EXCLUDED.completed_areas,main_areas=EXCLUDED.main_areas,stage=GREATEST(game_character_measuring_tibia.stage,EXCLUDED.stage),occurrence=EXCLUDED.occurrence")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(i32::from(quest.discovered_subareas))
        .bind(i32::from(quest.completed_areas))
        .bind(i32::from(quest.main_areas))
        .bind(i16::from(quest.stage))
        .bind(request.occurrence.as_slice())
        .execute(&mut ***tx)
        .await?;
    Ok(())
}

impl DurabilityRoot {
    /// MAPDISC command 6262: `Ok(None)` when the session fence is no longer current.
    pub async fn read_cyclopedia_discovery(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        areas: DiscoveryAreas,
    ) -> Result<Option<Vec<u8>>, DurabilityError> {
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                        return Ok(None);
                    }
                    let character = fence.character_id.as_bytes();
                    let row: Option<(Vec<u8>, String)> = sqlx::query_as("SELECT catalogue_digest,progress::text FROM game_character_cyclopedia WHERE character_id=encode($1,'hex')::uuid")
                        .bind(character.as_slice()).fetch_optional(&mut *tx).await?;
                    let progress = match row {
                        None => CanonicalProgress::default(),
                        Some((catalogue, text)) => {
                            if !BasicArea::reads_content_digest(&catalogue) || text.len() > 262_144 {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                            CanonicalProgress::decode(&text)?
                        }
                    };
                    let stored: Option<(i32, i32, i32, i16)> = sqlx::query_as("SELECT discovered_subareas,completed_areas,main_areas,stage FROM game_character_measuring_tibia WHERE character_id=encode($1,'hex')::uuid")
                        .bind(character.as_slice()).fetch_optional(&mut *tx).await?;
                    let quest = match stored {
                        None => wire::QuestProgress {
                            main_areas: areas.main_areas as u16,
                            ..wire::QuestProgress::default()
                        },
                        Some((discovered, completed, mains, stage)) => wire::QuestProgress {
                            discovered_subareas: u16::try_from(discovered)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                            completed_areas: u16::try_from(completed)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                            main_areas: u16::try_from(mains)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                            stage: u8::try_from(stage)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                        },
                    };
                    let bytes = wire::encode_reply(&areas.discovery(&progress, quest))
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(bytes))
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cyclopedia::MapProgress;

    fn progress(ids: &[u32]) -> CanonicalProgress {
        CanonicalProgress {
            legacy: MapProgress::default(),
            subareas: ids
                .iter()
                .map(|id| {
                    (
                        *id,
                        MapProgress {
                            discovered: true,
                            exploration: None,
                        },
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn mapdisc_quest_stages_follow_donor_milestones_over_all_main_areas()
    -> Result<(), &'static str> {
        // Four main areas; the fourth has no subareas and can never complete.
        let areas = DiscoveryAreas::from_pairs(&[(1, 11), (1, 12), (2, 21), (3, 31)], 4)
            .ok_or("areas")?;
        let stage = |ids: &[u32]| areas.quest(&progress(ids)).stage;
        assert_eq!(stage(&[]), 0);
        assert_eq!(stage(&[11]), 0);
        assert_eq!(stage(&[11, 12]), 1);
        assert_eq!(stage(&[11, 12, 21]), 2);
        assert_eq!(stage(&[11, 12, 21, 31]), 2);
        let full = DiscoveryAreas::from_pairs(&[(1, 11), (2, 21)], 2).ok_or("full")?;
        let quest = full.quest(&progress(&[11, 21]));
        assert_eq!((quest.stage, quest.completed_areas, quest.discovered_subareas), (3, 2, 2));
        assert!(DiscoveryAreas::from_pairs(&[(1, 11), (2, 21)], 1).is_none());
        assert!(DiscoveryAreas::from_pairs(&[], 0).is_none());
        Ok(())
    }

    #[test]
    fn mapdisc_discovery_lists_mains_with_subareas_and_both_subarea_lists()
    -> Result<(), &'static str> {
        let areas = DiscoveryAreas::from_pairs(&[(1, 12), (1, 11), (2, 21), (2, 22)], 3)
            .ok_or("areas")?;
        let value = areas.discovery(&progress(&[11, 12, 21, 99]), wire::QuestProgress::default());
        assert_eq!(
            value.mains,
            [
                wire::MainArea { id: 1, status: wire::STATUS_COMPLETE, progress: 100 },
                wire::MainArea { id: 2, status: wire::STATUS_DISCOVERING, progress: 50 },
            ]
        );
        assert_eq!(value.discovered, [11, 12, 21]);
        assert_eq!(value.discoverable, [22]);
        let bytes = wire::encode_reply(&value).map_err(|_| "encode")?;
        assert_eq!(wire::decode_reply(&bytes), Ok(value));
        Ok(())
    }
}
