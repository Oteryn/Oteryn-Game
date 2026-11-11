//! Reward prerequisites are authoritative and precede every progress/value effect.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

fn run_case(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_ELIGIBILITY POSTGRES_CASE_COMPLETED case={name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn questpc7_reward_requirements_refuse_missing_native_premium() -> TestResult {
    run_case("questpc7-eligibility-no-premium")
}
#[test]
fn questpc7_reward_requirements_current_premium_and_low_level() -> TestResult {
    run_case("questpc7-eligibility-current")
}
#[test]
fn questpc7_reward_requirements_high_level_and_readonly_parent() -> TestResult {
    run_case("questpc7-eligibility-high")
}

async fn complete_parent(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    complete_parent_key(owner, admitted, "oteryn:quest.authored.make_believe_quest").await
}

pub(super) async fn complete_parent_key(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    parent: &str,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition,
    };
    let slug = parent.rsplit('.').next().ok_or("parent key")?;
    let track_value = format!("oteryn:quest-progress/fixture/prior-{slug}");
    let key_value = format!("oteryn:quest-transition/fixture/prior-{slug}-completed");
    let track = track_value.as_str();
    let key = key_value.as_str();
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: track.into(),
                quest: parent.into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![QuestTransition {
                key: key.into(),
                quest: parent.into(),
                effects: vec![QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Eq(0),
                    effect: QuestEffectKind::Set(1),
                }],
                completes: true,
                experience: None,
        world_change: None,
            }],
        )
        .map_err(debug)?,
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(1390).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_ELIGIBILITY POSTGRES_STARTED version={version}");
    let [elane, wayland] = super::classic_quests::install(
        owner,
        admitted,
        ["oteryn:npc.elane", "oteryn:npc.wayland_smythers"],
    )
    .await?;
    super::classic_quests::talk(owner, admitted, elane, 1300, "hi").await?;
    let before:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let result =
        super::qengine_rewards::talk(owner, admitted, elane, 1301, "hunter commission").await?;
    if name.ends_with("no-premium") {
        assert_eq!(
            result,
            NpcIntentDisposition::Rejected,
            "a premium-required quest cannot start with no native premium authority"
        );
    } else {
        assert_eq!(
            result,
            NpcIntentDisposition::Ok,
            "Current native premium authorizes the Hunter start"
        );
        let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        if name.ends_with("high") {
            assert_eq!(level, 1000);
        } else {
            assert!(level < 1000);
            complete_parent(owner, admitted).await?;
        }
        super::classic_quests::talk(owner, admitted, wayland, 1310, "hi").await?;
        let refused =
            super::qengine_rewards::talk(owner, admitted, wayland, 1311, "illuminated commission")
                .await?;
        assert_eq!(
            refused,
            NpcIntentDisposition::Rejected,
            "missing level or parent prerequisite must refuse"
        );
        if name.ends_with("high") {
            complete_parent(owner, admitted).await?;
            let parent_before:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.make_believe_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                super::qengine_rewards::talk(
                    owner,
                    admitted,
                    wayland,
                    1312,
                    "illuminated commission"
                )
                .await?,
                NpcIntentDisposition::Ok
            );
            let parent_after:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.make_believe_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                parent_after, parent_before,
                "eligibility never rewrites its parent completion fact"
            );
        }
    }
    let after:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    if name.ends_with("no-premium") {
        assert_eq!(
            after, before,
            "denied eligibility creates no quest receipt or progress"
        );
    }
    Ok(())
}
