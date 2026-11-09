//! Premium-gated chosen Barbarian Test starter through production TALK dispatch.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::authored_bindings::Trigger;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

#[test]
fn m01a_sven_premium_production_talk_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m01a-sven")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    _authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    exercise_route(
        owner,
        admitted,
        h,
        "oteryn:quest.barbarian_test",
        "oteryn:npc.sven",
    )
    .await
}

pub(super) async fn exercise_route(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    key: &str,
    npc: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M01A_PREMIUM_TALK_POSTGRES quest={key} npc={npc} {version}");
    if key == "oteryn:quest.barbarian_test" {
        eprintln!("M01A_SVEN_POSTGRES {version}");
    }
    let callback = {
        let bindings = owner
            .quest_catalogue
            .as_ref()
            .ok_or("production catalogue")?
            .authored_bindings()
            .ok_or("production registry")?;
        let quest = bindings
            .quests
            .iter()
            .find(|quest| quest.quest == key)
            .ok_or_else(|| format!("production TALK route is missing: {key}"))?;
        let stage = quest
            .stages
            .iter()
            .find(|stage| stage.key == "s1")
            .ok_or("briefing")?;
        assert!(!stage.completes);
        let Trigger::NpcDialogue { callback, .. } = &stage.trigger else {
            return Err("TALK route".into());
        };
        assert_eq!(callback.npc, npc);
        assert!(callback.requires_premium);
        callback.as_ref().clone()
    };
    let initial: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE quest_key=$1")
            .bind(key)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(initial, 0, "no chosen progress is seeded");
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
    let [target, other] =
        super::classic_quests::install(owner, admitted, [npc, "oteryn:npc.baxter"]).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let wrong = super::qengine_rewards::wire_reward(owner, admitted, other, &callback, false)
        .await?
        .0;
    assert_eq!(wrong, NpcIntentDisposition::Ok);
    assert_eq!(
        footprint(h, key).await?,
        (0, 0),
        "another NPC never advances this quest"
    );
    let premium = owner.premium_coordinator.take();
    let denied =
        super::qengine_rewards::wire_reward(owner, admitted, target, &callback, false).await;
    owner.premium_coordinator = premium;
    assert_eq!(
        denied?.0,
        NpcIntentDisposition::Rejected,
        "missing Premium owner fails closed"
    );
    assert_eq!(
        footprint(h, key).await?,
        (0, 0),
        "refusal has no durable effect"
    );
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    let (accepted, _, command) =
        super::qengine_rewards::wire_reward(owner, admitted, target, &callback, false).await?;
    assert_eq!(accepted, NpcIntentDisposition::Ok);
    assert_eq!(footprint(h, key).await?, (1, 1));
    let replay_payload =
        oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: target,
            text: callback.keywords[0].clone(),
        })
        .map_err(|error| format!("{error:?}"))?;
    let replay =
        super::quest_world_callbacks::wire(owner, admitted, command, 7, replay_payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay)
            .map_err(|error| format!("{error:?}"))?,
        NpcIntentDisposition::Ok,
        "the identical command returns its cached successful result"
    );
    assert_eq!(
        footprint(h, key).await?,
        (1, 1),
        "one nonterminal receipt, including literal replay"
    );
    let repeated = super::qengine_rewards::wire_reward(owner, admitted, target, &callback, false)
        .await?
        .0;
    assert_eq!(repeated, NpcIntentDisposition::Rejected);
    assert_eq!(footprint(h, key).await?, (1, 1));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let value: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
    )
    .bind(key)
    .bind(&callback.storage[0].track)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(value, 1);
    let completed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(completed, 0, "a briefing does not complete its quest");
    Ok(())
}

async fn footprint(h: &Harness, key: &str) -> Result<(i64, i64), sqlx::Error> {
    sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE quest_key=$1), (SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1)")
        .bind(key).fetch_one(&h.pool).await
}
