use super::fixture::Harness;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::character_authority::ReconciledCharacterAuthority;

use oteryn_gameplay_quests::quest::authored_bindings::Trigger;

use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M01A_SVEN_POSTGRES {version}");
    let key = "oteryn:quest.barbarian_test";
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
            .ok_or("Sven production route is missing")?;
        let stage = quest
            .stages
            .iter()
            .find(|stage| stage.key == "s1")
            .ok_or("briefing")?;
        assert!(!stage.completes);
        let Trigger::NpcDialogue { callback, .. } = &stage.trigger else {
            return Err("TALK route".into());
        };
        assert_eq!(callback.npc, "oteryn:npc.sven");
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
    let [sven, other] = super::classic_quests_fixtures::install(
        owner,
        admitted,
        ["oteryn:npc.sven", "oteryn:npc.baxter"],
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let wrong =
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, other, &callback, false)
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
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, sven, &callback, false).await;
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
    let accepted =
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, sven, &callback, true)
            .await?
            .0;
    assert_eq!(accepted, NpcIntentDisposition::Ok);
    assert_eq!(
        footprint(h, key).await?,
        (1, 1),
        "one nonterminal receipt, including literal replay"
    );
    let repeated =
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, sven, &callback, false)
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
    assert_eq!(
        completed, 0,
        "starting the test does not complete the quest"
    );
    use oteryn_gameplay_quests::quest::{barbarian_mead, reward_callbacks, world_callbacks};
    let callbacks =
        reward_callbacks::offered_for(owner.quest_catalogue.as_ref().ok_or("catalogue")?)?;
    let reward = |suffix: &str| {
        callbacks
            .iter()
            .find(|r| r.npc == "oteryn:npc.sven" && r.key.ends_with(suffix))
            .ok_or_else(|| format!("Sven reward {suffix}"))
    };
    let start = reward("/sven-start")?;
    let honeycomb = reward("/sven-honeycomb")?;
    let report = reward("/sven-mammoth-report")?;
    assert_eq!(
        super::qengine_rewards_fixtures::wire_conversation(
            owner,
            admitted,
            sven,
            start,
            false,
            vec!["hi", "barbarian", "test", "yes", "yes"],
        )
        .await?
        .0,
        NpcIntentDisposition::Ok
    );
    let words = || vec!["hi", "mead", "yes"];
    assert_eq!(
        super::qengine_rewards_fixtures::wire_conversation(
            owner,
            admitted,
            sven,
            honeycomb,
            false,
            words(),
        )
        .await?
        .0,
        NpcIntentDisposition::Rejected
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let at = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .tile(33293, 31677, -15);
    assert!(at.is_some(), "fixture offering source");
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &world_callbacks::Offering {
            position: world_callbacks::Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: "oteryn:item.tibia.i5902".into(),
            count: 1,
        },
        61,
        true,
    )
    .await?;
    assert_eq!(
        super::qengine_rewards_fixtures::wire_conversation(
            owner,
            admitted,
            sven,
            honeycomb,
            false,
            words(),
        )
        .await?
        .0,
        NpcIntentDisposition::Ok
    );
    let state = h
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(super::fixture::debug)?;
    assert_eq!(state.tracks().get(barbarian_mead::TRACKS[0]), Some(&2));
    assert_eq!(
        super::qengine_outfit_stories_fixtures::quantity(h, "oteryn:item.tibia.i5902").await?,
        0
    );
    for (completed, next, suffix) in [
        (3, 4, "/sven-mead-report"),
        (5, 6, "/sven-bear-report"),
        (7, 8, "/sven-mammoth-report"),
    ] {
        // Each world success has its own native USE proof. Arrange that earned
        // state here so this case proves the downstream NPC handoff.
        super::qm47_mead_fixtures::seed_sven_state(h, owner, admitted, [completed, 3, 9, 9])
            .await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        assert_eq!(
            super::qengine_rewards_fixtures::wire_reward(
                owner,
                admitted,
                sven,
                reward(suffix)?,
                false,
            )
            .await?
            .0,
            NpcIntentDisposition::Ok
        );
        let state = h
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(super::fixture::debug)?;
        assert_eq!(state.tracks().get(barbarian_mead::TRACKS[0]), Some(&next));
    }
    assert_eq!(
        super::qengine_outfit_stories_fixtures::quantity(h, "oteryn:item.tibia.i7140").await?,
        1
    );
    let terminal: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts
            WHERE quest_key=$1 AND transition_key=$2 AND completes),
            (SELECT count(*) FROM game_account_achievements
             WHERE achievement_key='oteryn:achievement/honorary_barbarian')",
    )
    .bind(key)
    .bind(&report.key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(terminal, (1, 1));
    Ok(())
}

async fn footprint(h: &Harness, key: &str) -> Result<(i64, i64), sqlx::Error> {
    sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE quest_key=$1), (SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1)")
        .bind(key).fetch_one(&h.pool).await
}
