//! Maelyrra's combined kill requirement is checked by the durable owner.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestStateCatalogue, threatened_dreams};
use std::sync::Arc;

const REPORT: &str = "oteryn:quest-transition/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders/npc_2";
const BASE: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/";

#[test]
fn quest_l4_maelyrra_sum_and_boss_are_atomic_and_replayable() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUEST_L4_SUM_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let started = std::time::Instant::now();
            let result = run(async |admin| {
                let h = Harness::create_with_admission_seed(admin, "quest_l4_sum", false, false)
                    .await?;
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let (controller, _) =
                    super::death_ml_loss_composed::production_generation_with_pin(
                        deps.admitted.world_id,
                        "content-1",
                    )?;
                let generation = controller.active().ok_or("generation")?;
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(generation);
                deps.initialize(&owner).await;
                let mut admitted = deps.admitted;
                owner.admit_progression_session(&mut admitted).await;
                let embedded =
                    oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                        .map_err(debug)?;
                let source = embedded.catalogue();
                let keys = [
                    threatened_dreams::NIGHTMARE.to_owned(),
                    format!("{BASE}enfeebled_count"),
                    format!("{BASE}frazzlemaws_count"),
                    format!("{BASE}kroazur_kill"),
                ];
                // Boundary snapshots stand for previously committed native deaths. No
                // successful report is seeded; the production writer must create it.
                for (case, values, succeeds) in [
                    (0, [1, 99, 100, 1], false),
                    (1, [1, 100, 100, 0], false),
                    (2, [1, 100, 100, 1], true),
                ] {
                    let tracks = keys
                        .iter()
                        .zip(values)
                        .map(|(key, initial)| {
                            let mut track = source.track(key).ok_or("source track")?.clone();
                            track.initial = initial;
                            Ok(track)
                        })
                        .collect::<TestResult<Vec<_>>>()?;
                    let transitions = source.transition(REPORT).cloned().into_iter().collect();
                    let catalogue = Arc::new(
                        QuestStateCatalogue::new("content-1", tracks, transitions)
                            .map_err(debug)?,
                    );
                    let fence = owner
                        .current_quest_fence(admitted.game_session_id)
                        .await
                        .map_err(|()| "fence")?
                        .ok_or("fence")?;
                    let request = QuestTransitionRequest {
                        transition_key: REPORT.into(),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(9100 + case).map_err(debug)?,
                        )),
                    };
                    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                    let result = slot
                        .commit_quest_transition(
                            owner.root,
                            owner.character,
                            owner.holder,
                            fence,
                            request.clone(),
                            Arc::clone(&catalogue),
                        )
                        .await;
                    eprintln!(
                        "QUEST_L4_SUM POSTGRES_ASSERTION_REACHED case={case} duration_ms={}",
                        started.elapsed().as_millis()
                    );
                    let outcome = result.map_err(debug)?;
                    if succeeds {
                        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
                        let replay = slot
                            .commit_quest_transition(
                                owner.root,
                                owner.character,
                                owner.holder,
                                fence,
                                request,
                                catalogue,
                            )
                            .await
                            .map_err(debug)?;
                        assert!(matches!(
                            replay,
                            QuestTransitionOutcome::AlreadyCommitted(_)
                        ));
                        let copy = owner
                            .root
                            .read_character_quest_state(&authority, fence.character_id)
                            .await
                            .map_err(debug)?;
                        assert_eq!(copy.tracks().get(threatened_dreams::NIGHTMARE), Some(&2));
                    } else {
                        assert_eq!(
                            outcome,
                            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
                        );
                        let copy = owner
                            .root
                            .read_character_quest_state(&authority, fence.character_id)
                            .await
                            .map_err(debug)?;
                        assert!(
                            copy.tracks().is_empty(),
                            "a refused report must write no tracks"
                        );
                    }
                }
                let receipts: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key = $1",
                )
                .bind(REPORT)
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(receipts, 1, "only the successful report creates a receipt");
                Ok(())
            });
            eprintln!(
                "QUEST_L4_SUM POSTGRES_CASE_COMPLETED duration_ms={} success={}",
                started.elapsed().as_millis(),
                result.is_ok()
            );
            result.map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "sum scenario panicked")??;
    Ok(())
}
