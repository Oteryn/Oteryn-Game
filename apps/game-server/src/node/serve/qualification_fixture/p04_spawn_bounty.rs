use crate::bestiary_postgres_harness::{Harness, TestResult, debug, fence};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::ExperienceRewardOccurrence;
use crate::durability::quest_state::QuestCause;
use crate::durability::task_board::{TaskBoardCatalogue, TaskBoardCommand};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, OwnerDamageCommand,
};
use oteryn_gameplay_quests::quest::task_board::full::{
    BountyBoard, BountyStatus, BountyTask, TaskBoardAction, TaskBoardData,
};
use std::sync::Arc;

const RAT: &str = "canary:creature/rat";

pub(super) struct SpawnBounty<'a> {
    harness: &'a Harness,
    catalogue: Arc<TaskBoardCatalogue>,
}

impl<'a> SpawnBounty<'a> {
    pub(super) async fn seed(
        harness: &'a Harness,
        room: &crate::content::QualifiedNativeEntryRoom,
        repetitions: u16,
    ) -> TestResult<Self> {
        let current = fence(1)?;
        let world = current.runtime_scope.world_id();
        let channel = current
            .runtime_scope
            .channel_id()
            .ok_or("Channel fixture")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4) ON CONFLICT DO NOTHING")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
            .execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
            .bind(room.compiled().server_digest().as_slice())
            .bind(room.compiled().client_digest().as_slice())
            .bind(room.frame_binding().digest().as_slice())
            .execute(&harness.pool).await?;
        sqlx::query("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
            .bind(world.as_bytes().as_slice())
            .bind(room.compiled().server_digest().as_slice())
            .bind("p04-spawn-bounty-map")
            .bind(room.compiled().client_digest().as_slice())
            .bind(room.frame_binding().digest().as_slice())
            .execute(&harness.pool).await?;
        let board = TaskBoardData {
            bounty: BountyBoard {
                state: BountyStatus::Active,
                active: Some(BountyTask {
                    elite: false,
                    dust: 0,
                    race: RAT.into(),
                    required: u32::from(repetitions),
                    current: 0,
                    experience: 1850,
                    points: 25,
                    grade: 0,
                    difficulty: 0,
                    slot: 0,
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_task_boards(character_id,world_id,body,committed_character_revision,cause_id,cause_ordinal) SELECT character_id,encode($2,'hex')::uuid,$3,character_revision,encode($4,'hex')::uuid,1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(current.character_id.as_bytes().as_slice())
            .bind(world.as_bytes().as_slice())
            .bind(serde_json::to_value(board)?)
            .bind(current.game_session_id.as_bytes().as_slice())
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Self {
            harness,
            catalogue: Arc::new(TaskBoardCatalogue {
                creatures: Vec::new(),
                items: Default::default(),
                gameplay_content_revision: "content-1".into(),
                progression: None,
                content_revision: String::new(),
                cosmetics: None,
            }),
        })
    }

    pub(super) fn kill(
        &self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        player: ExactActorRef,
        session: GameSessionId,
        sequence: u16,
    ) -> TestResult<ExperienceRewardOccurrence> {
        let health = runtime.companion_snapshot(target)?.health;
        let mut death = target.corpse_identity();
        death[6] = (death[6] & 0x0f) | 0x70;
        death[8] = (death[8] & 0x3f) | 0x80;
        let command = CommandRef::new(session, CommandId::new(u64::from(sequence))?);
        let damage = runtime
            .borrow_exact_actor_commit_at(u64::from(sequence))
            .commit_damage_for_bound_attacker(
                target,
                player,
                command,
                0,
                OwnerDamageCommand {
                    target: RAT.as_bytes(),
                    occurrence: &death,
                    binding: b"p04-spawn-bounty-hit",
                    damage: health,
                },
            )?;
        assert!(damage.applied && damage.health_after == 0);
        runtime.remove_test_actor(target)?;
        Ok(ExperienceRewardOccurrence::from_bytes(death).map_err(debug)?)
    }

    pub(super) async fn credit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: ExperienceRewardOccurrence,
        sequence: u16,
    ) -> TestResult {
        let id = fence(1)?.character_id;
        let revision: i64 = sqlx::query_scalar("SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id.as_bytes().as_slice()).fetch_one(&self.harness.pool).await?;
        let commit = self
            .harness
            .root
            .commit_task_board(
                authority,
                &self.harness.node,
                fence(u64::try_from(revision)?)?,
                TaskBoardCommand {
                    cause: QuestCause::CreatureDeath(occurrence),
                    action: TaskBoardAction::CreditKill { race: RAT.into() },
                    payload_binding: Vec::new(),
                    fee_change: None,
                },
                Arc::clone(&self.catalogue),
                None,
            )
            .await
            .map_err(debug)?;
        assert!(commit.refusal.is_none() && !commit.replayed);
        assert_eq!(
            commit
                .view
                .data
                .bounty
                .active
                .ok_or("active bounty")?
                .current,
            u32::from(sequence)
        );
        Ok(())
    }

    pub(super) async fn assert_credited(&self, repetitions: u16) -> TestResult {
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_task_board_receipts WHERE cause_kind='creature_death' AND refusal = 'null'::jsonb")
            .fetch_one(&self.harness.pool).await?;
        assert_eq!(receipts, i64::from(repetitions));
        println!("POSTGRES_CASE_COMPLETED p04_sequential_spawn_and_bounty kills={receipts}");
        Ok(())
    }
}
