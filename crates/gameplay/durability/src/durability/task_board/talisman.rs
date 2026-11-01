//! The Task Board owner's read-only talisman projection. Equipment custody, the
//! active bounty and the four existing quest tracks are read under the caller's
//! current Character root lock; a missing board never activates a weekly task.

use std::collections::BTreeMap;

use oteryn_gameplay_quests::quest::task_board::{
    self,
    full::{BountyStatus, TaskBoardData},
};
use sqlx::{Postgres, Row, Transaction};

use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BountyTalismanSnapshot {
    race: Option<String>,
    bonuses: [i64; 4],
}

impl BountyTalismanSnapshot {
    #[must_use]
    pub fn from_board(
        data: &TaskBoardData,
        tracks: &BTreeMap<String, i64>,
        equipped: bool,
    ) -> Self {
        let Some(active) = data.bounty.active.as_ref().filter(|active| {
            equipped
                && data.bounty.state == BountyStatus::Active
                && active.current < active.required
        }) else {
            return Self::default();
        };
        Self {
            race: Some(active.race.clone()),
            bonuses: data.active_talisman_bonuses(tracks, &active.race, equipped),
        }
    }

    #[must_use]
    pub fn bonuses(&self, race: &str) -> [i64; 4] {
        if self.race.as_deref() == Some(race) {
            self.bonuses
        } else {
            [0; 4]
        }
    }
}

/// The caller must already hold the current gameplay/Item authority and Character
/// root lock in this transaction. This is shared by combat and the Bestiary writer.
pub async fn read_talisman_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
) -> Result<BountyTalismanSnapshot, DurabilityError> {
    read_talisman_for_character(
        tx,
        fence.character_id.as_bytes(),
        fence.runtime_scope.world_id().as_bytes(),
    )
    .await
}

/// A timer continuation already holds the Item owner's current Character/root
/// proof. Its read fence has no progression-revision field, which this read does
/// not consume; never manufacture a revision just to project equipment effects.
pub async fn read_talisman_in_item_transaction(
    tx: &mut Transaction<'_, Postgres>,
    fence: super::super::item_transfer::CurrentCharacterItemFence,
) -> Result<BountyTalismanSnapshot, DurabilityError> {
    read_talisman_for_character(
        tx,
        fence.character_id.as_bytes(),
        fence.runtime_scope.world_id().as_bytes(),
    )
    .await
}

async fn read_talisman_for_character(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    world: &[u8; 16],
) -> Result<BountyTalismanSnapshot, DurabilityError> {
    let character = character.as_slice();
    let row = sqlx::query("SELECT body,uuid_send(world_id) AS world FROM game_character_task_boards WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(BountyTalismanSnapshot::default());
    };
    if row.try_get::<Vec<u8>, _>("world")? != world.as_slice() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let data: TaskBoardData = serde_json::from_value(row.try_get("body")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if data.bounty.state != BountyStatus::Active {
        return Ok(BountyTalismanSnapshot::default());
    }
    let levels = sqlx::query("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key LIMIT 4 FOR UPDATE")
        .bind(character).bind(task_board::TALISMAN_PATHS.as_slice()).fetch_all(&mut **tx).await?;
    let mut tracks = BTreeMap::new();
    for level in levels {
        let value: i64 = level.try_get("value")?;
        if !(0..=task_board::TALISMAN_MAX_LEVEL).contains(&value) {
            return Err(DurabilityError::InvalidStoredState);
        }
        tracks.insert(level.try_get("track_key")?, value);
    }
    // Crystal player.cpp:9913-9916: item 51978 must occupy the extra/ammo slot.
    // Slot custody can only change while its Character root is held. Lock the
    // item itself as the existing equipment reader does, without changing custody.
    let item = sqlx::query("SELECT i.definition_family,i.definition_production_key,i.lifecycle,uuid_send(i.world_id) AS item_world,uuid_send(s.world_id) AS slot_world FROM game_character_equipment_slots s JOIN game_item_instances i USING(item_instance_id) WHERE s.character_id=encode($1,'hex')::uuid AND s.slot=10 FOR UPDATE OF i")
        .bind(character).fetch_optional(&mut **tx).await?;
    let equipped = if let Some(item) = item {
        item.try_get::<i16, _>("lifecycle")? == 1
            && item.try_get::<String, _>("definition_family")? == "Item"
            && item.try_get::<String, _>("definition_production_key")? == task_board::TALISMAN_ITEM
            && item.try_get::<Vec<u8>, _>("item_world")? == world.as_slice()
            && item.try_get::<Vec<u8>, _>("slot_world")? == world.as_slice()
    } else {
        false
    };
    Ok(BountyTalismanSnapshot::from_board(&data, &tracks, equipped))
}

impl DurabilityRoot {
    pub async fn read_bounty_talisman(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<BountyTalismanSnapshot, CharacterProgressionError> {
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
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let snapshot = read_talisman_in_transaction(&mut tx, fence).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(snapshot))
                })
            })
            .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_quests::quest::task_board::full::BountyTask;

    #[test]
    fn talisman_requires_equipped_extra_slot_and_active_matching_bounty() {
        let mut board = TaskBoardData::default();
        let tracks = [
            (task_board::TALISMAN_DAMAGE.to_owned(), 1),
            (task_board::TALISMAN_BESTIARY.to_owned(), 355),
            (task_board::SELECTED.to_owned(), 1),
        ]
        .into();
        assert_eq!(
            BountyTalismanSnapshot::from_board(&board, &tracks, true)
                .bonuses("oteryn:creature.rat"),
            [0; 4]
        );
        board.bounty.state = BountyStatus::Active;
        board.bounty.active = Some(BountyTask {
            race: "oteryn:creature.rat".to_owned(),
            required: 25,
            current: 24,
            experience: 1850,
            points: 25,
            grade: 0,
            difficulty: 0,
            slot: 0,
        });
        assert_eq!(
            BountyTalismanSnapshot::from_board(&board, &tracks, false)
                .bonuses("oteryn:creature.rat"),
            [0; 4]
        );
        let snapshot = BountyTalismanSnapshot::from_board(&board, &tracks, true);
        assert_eq!(snapshot.bonuses("oteryn:creature.rat"), [300, 0, 0, 10_000]);
        assert_eq!(snapshot.bonuses("oteryn:creature.cave_rat"), [0; 4]);
        board.bounty.state = BountyStatus::Completed;
        assert_eq!(
            BountyTalismanSnapshot::from_board(&board, &tracks, true)
                .bonuses("oteryn:creature.rat"),
            [0; 4]
        );
        // A frozen lethal-hit snapshot survives the task's completing credit.
        assert_eq!(snapshot.bonuses("oteryn:creature.rat"), [300, 0, 0, 10_000]);
    }
}
