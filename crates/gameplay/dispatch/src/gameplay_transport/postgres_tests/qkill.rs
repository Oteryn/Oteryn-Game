//! General creature objectives through the production native lethal queue and quest writer.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use std::sync::Arc;

#[test]
fn qkill_declared_creature_and_stage_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario("qengine-rewards-qkill-stage")
}
#[test]
fn qkill_duplicate_death_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario("qengine-rewards-qkill-replay")
}
#[test]
fn qkill_threshold_advances_stage_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario("qengine-rewards-qkill-threshold")
}
#[test]
fn qkill_removes_only_declared_creatures_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario("qengine-rewards-qkill-removal")
}

#[test]
fn qkill_last_hit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario("qengine-rewards-qkill-last_hit")
}
#[test]
fn qkill_most_damage_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario("qengine-rewards-qkill-most_damage")
}
#[test]
fn qkill_damaging_players_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario("qengine-rewards-qkill-damaging_players")
}
#[test]
fn qkill_recovery_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario("qengine-rewards-qkill-recovery")
}
#[test]
fn qkill_rookie_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario("qengine-rewards-qkill-rookie")
}

#[test]
fn qkill_declared_stage_removal_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario("qengine-rewards-qkill-stage_removal")
}

const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const PHASE: &str = "oteryn:quest-progress/authored-kills/phase";
const COUNT: &str = "oteryn:quest-progress/authored-kills/count";

async fn value(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2),0)")
        .bind(QUEST).bind(track).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    if name.ends_with("-rookie") {
        return rookie(owner, admitted, h).await;
    }
    let mut document: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quests/src/quest/authored_bindings/fixtures/kill.json"
    )))?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    document["quests"][0]["stages"][1]["trigger"]["counter"]["remove_on_advance"] = serde_json::json!([{
        "creature": "oteryn:creature.rat",
        "area": {"minimum": [at.x,at.y,i32::from(at.floor)],
                 "maximum": [at.x+4,at.y+4,i32::from(at.floor)]}
    }]);
    for credit in ["last_hit", "most_damage", "damaging_players"] {
        if name.ends_with(credit) {
            document["quests"][0]["stages"][1]["trigger"]["counter"]["credit"] = credit.into();
        }
    }
    if name.ends_with("-stage_removal") {
        let removals = document["quests"][0]["stages"][1]["trigger"]["counter"]
            .as_object_mut()
            .ok_or("counter declaration")?
            .remove("remove_on_advance")
            .ok_or("removals")?;
        document["quests"][0]["creature_removals"] = serde_json::json!([{
            "track": PHASE, "value": 5, "creatures": removals
        }]);
    }
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    let start = bindings.reward_callbacks().next().cloned().ok_or("start")?;
    let base = QuestStateCatalogue::empty(
        owner
            .quest_catalogue
            .as_ref()
            .ok_or("catalogue")?
            .content_revision(),
    )
    .ok_or("base")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.cledwyn"]).await?;
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.rat", 98200).await?;
    assert_eq!(
        value(h, COUNT).await?,
        0,
        "inactive stage cannot earn credit"
    );
    assert_eq!(
        super::qengine_rewards::wire_reward(owner, admitted, npc, &start, false)
            .await?
            .0,
        NpcIntentDisposition::Ok
    );
    super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.dragon", 98201).await?;
    assert_eq!(
        value(h, COUNT).await?,
        0,
        "undeclared creature cannot earn credit"
    );
    let targets = {
        let mut runtime = owner.runtime.lock().await;
        let mut targets = Vec::new();
        for (creature, x, y) in [
            ("oteryn:creature.rat", at.x + 2, at.y + 2),
            ("oteryn:creature.dragon", at.x + 3, at.y + 2),
            ("oteryn:creature.rat", at.x + 5, at.y + 2),
        ] {
            let policy = runtime.companion_policy(creature).map_err(debug)?;
            let target = runtime
                .admit_encounter_actor(
                    creature,
                    policy.maximum_health,
                    MovementLocalPosition {
                        x,
                        y,
                        floor: at.floor,
                    },
                )
                .map_err(debug)?;
            runtime
                .install_creature_policy(target, creature)
                .map_err(debug)?;
            targets.push(target);
        }
        targets
    };
    for count in 1..=3 {
        super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.rat", 98201 + count)
            .await?;
        assert_eq!(value(h, COUNT).await?, i64::try_from(count)?);
        assert_eq!(value(h, PHASE).await?, if count == 3 { 5 } else { 4 });
        if count < 3 {
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .contains_live_creature(targets[0])
            );
        }
    }
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            !runtime.contains_live_creature(targets[0]),
            "declared creature retires at threshold"
        );
        assert!(
            runtime.contains_live_creature(targets[1]),
            "different species survives in area"
        );
        assert!(
            runtime.contains_live_creature(targets[2]),
            "same species survives outside area"
        );
    }
    super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.rat", 98205).await?;
    assert_eq!(
        value(h, COUNT).await?,
        3,
        "advanced stage cannot earn extra credit"
    );
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND transition_key=$2")
        .bind(QUEST).bind("oteryn:quest-transition/native-kills/authored-engine-rat").fetch_one(&h.pool).await?;
    assert_eq!(
        receipts, 3,
        "duplicate native projections and drains credit once per death"
    );
    if name.ends_with("-recovery") {
        let restored = {
            let mut runtime = owner.runtime.lock().await;
            let policy = runtime
                .companion_policy("oteryn:creature.rat")
                .map_err(debug)?;
            let actor = runtime
                .admit_encounter_actor(
                    "oteryn:creature.rat",
                    policy.maximum_health,
                    MovementLocalPosition {
                        x: at.x + 2,
                        y: at.y + 2,
                        floor: at.floor,
                    },
                )
                .map_err(debug)?;
            runtime
                .install_creature_policy(actor, "oteryn:creature.rat")
                .map_err(debug)?;
            actor
        };
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        assert!(
            !owner.runtime.lock().await.contains_live_creature(restored),
            "persisted stage reconciles after projection reconstruction"
        );
        assert_eq!(
            value(h, COUNT).await?,
            3,
            "recovery creates no kill increment"
        );
    }
    Ok(())
}

/// Mission acceptance is a fixture precondition using the real lowered transition.
/// Every counted kill below uses the production lethal capture/drain and immutable binding.
async fn rookie(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    const COUNT: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/rat_kills";
    let catalogue = Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?);
    assert!(
        catalogue
            .transition("oteryn:quest-transition/native-kills/qkill-rookie-rats")
            .is_some()
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key:
                    "oteryn:quest-transition/quest/u9_1/the_rookie_guard/mission03/npc_1".into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(98400).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    for count in 1..=6 {
        super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.rat", 98400 + count)
            .await?;
        let stored:i64=sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(COUNT).fetch_one(&h.pool).await?;
        assert_eq!(
            stored,
            i64::try_from(count.min(5))?,
            "Rookie Guard source cap and duplicate-death protection"
        );
    }
    let stage: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind("oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission03")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        stage, 1,
        "Rookie Guard retains its source NPC-owned mission hand-in"
    );
    Ok(())
}
