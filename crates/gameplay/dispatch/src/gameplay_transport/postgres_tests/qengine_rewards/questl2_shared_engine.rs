//! Reuse QENGINE2's pinned Rook Citizen story to prove counters, clocks and foreign owners.
use super::*;

#[test]
fn questl2_shared_counter_timer_foreign_owner_player_journey() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_quest_reward(admin, "l2_shared_engine", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED l2-shared-engine-binding server_version={version}");
        let existing = oteryn_gameplay_quests::quest::reward_callbacks::load()?
            .into_iter()
            .next()
            .ok_or("callback")?;
        let mut value = serde_json::to_value(existing)?;
        value["state_ops"] = serde_json::json!([{"track":"oteryn:test/clock","from":{"kind":"any"},"effect":{"kind":"set_now"}}]);
        value["participants"] = serde_json::json!([{"quest":"oteryn:quest.test","storage":[],"state_ops":[{"track":"oteryn:test/counter","from":{"kind":"any"},"effect":{"kind":"add","value":1}}]}]);
        let supported = serde_json::from_value::<RewardCallback>(value).is_ok();
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE l2-shared-engine duration={:?}",
            started.elapsed()
        );
        assert!(
            supported,
            "shared counter, clock and foreign participant engine fields are absent"
        );
        Ok(())
    })?;
    scenario("qengine-rewards-l2-engine-shared")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_gameplay_quests::quest::world_callbacks::Offering;
    use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind};
    use std::collections::BTreeMap;
    const ROOT: &str = "oteryn:quest-progress/quest/u7_8/citizen_outfits_rook/";
    const REF: &str = "oteryn:quest-progress/outfit_quest/ref";
    const CLOCK: &str = "oteryn:quest-transition/l2-shared-proof/clock";
    let all = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let names = [
        "backpack-start",
        "backpack-materials",
        "backpack",
        "hat-start",
        "hat",
    ];
    let callbacks: Vec<_> = names
        .iter()
        .map(|name| {
            all.iter()
                .find(|c| c.key.ends_with(&format!("/l6-citizen-rook-{name}")))
                .cloned()
                .ok_or("source Citizen binding")
        })
        .collect::<Result<_, _>>()?;
    let canonical = owner.quest_catalogue.clone().ok_or("catalogue")?;
    let mut tracks = BTreeMap::new();
    let mut transitions = Vec::new();
    for callback in &callbacks {
        let mut keys = vec![callback.key.clone()];
        keys.extend(callback.participants.iter().enumerate().map(|(i, _)| {
            oteryn_gameplay_quests::quest::reward_callbacks::participant_key(&callback.key, i)
        }));
        for key in keys {
            let transition = canonical
                .transition(&key)
                .cloned()
                .ok_or("source transition")?;
            for effect in &transition.effects {
                tracks.insert(
                    effect.track.clone(),
                    canonical.track(&effect.track).cloned().ok_or("track")?,
                );
            }
            transitions.push(transition);
        }
    }
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    transitions.push(QuestTransition { world_change: None,
        key: CLOCK.into(),
        quest: "oteryn:quest.citizen_outfits_quest_on_rook".into(),
        effects: vec![QuestEffect {
            track: format!("{ROOT}addon_backpack_rook_timer"),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(now - 7202),
        }],
        completes: false,
        experience: None,
    });
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            tracks.values().cloned().collect(),
            transitions,
        )
        .map_err(debug)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_tracks")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let [amber, norma] = super::super::classic_quests::install_nearby(
        owner,
        admitted,
        ["oteryn:npc.amber", "oteryn:npc.norma"],
    )
    .await?;
    assert_eq!(
        wire_grant(owner, admitted, amber, &callbacks[0]).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT value FROM game_character_quest_tracks WHERE track_key=$1"
        )
        .bind(REF)
        .fetch_one(&h.pool)
        .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_quest_reward_participants")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    for (index, callback) in callbacks.iter().enumerate().skip(1) {
        let npc = if index >= 3 { norma } else { amber };
        if index == 2 {
            let (stamp,current):(i64,i64)=sqlx::query_as("SELECT value,floor(extract(epoch FROM now()))::bigint FROM game_character_quest_tracks WHERE track_key=$1").bind(format!("{ROOT}addon_backpack_rook_timer")).fetch_one(&h.pool).await?;
            assert!(stamp > 0 && stamp <= current && current - stamp < 7201);
            let before: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                wire_reward(owner, admitted, npc, callback, false).await?.0,
                NpcIntentDisposition::Rejected
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_account_appearance_unlock_receipts"
                )
                .fetch_one(&h.pool)
                .await?,
                before
            );
            let before:Vec<(String,i64)>=sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE track_key<>$1 ORDER BY track_key").bind(format!("{ROOT}addon_backpack_rook_timer")).fetch_all(&h.pool).await?;
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
                        transition_key: CLOCK.into(),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(9800).map_err(debug)?,
                        )),
                    },
                    owner.quest_catalogue.clone().ok_or("catalogue")?,
                )
                .await
                .map_err(debug)?;
            assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
            drop(slot);
            let after:Vec<(String,i64)>=sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE track_key<>$1 ORDER BY track_key").bind(format!("{ROOT}addon_backpack_rook_timer")).fetch_all(&h.pool).await?;
            assert_eq!(
                before, after,
                "clock simulation cannot seed any quest stage or foreign counter"
            );
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
        }
        for (item, amount) in callback.take.iter().map(|i| (&i.item, i.count)) {
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "fence")?
                .ok_or("fence")?;
            super::super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &Offering {
                    position: Position {
                        x: 33294,
                        y: 31677,
                        floor: -15,
                    },
                    item: item.clone(),
                    count: amount,
                },
                9900 + u64::try_from(index)? * 10 + u64::from(amount),
                true,
            )
            .await?;
        }
        assert_eq!(
            wire_grant(owner, admitted, npc, callback).await?.0,
            NpcIntentDisposition::Ok,
            "{}",
            callback.key
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT value FROM game_character_quest_tracks WHERE track_key=$1"
        )
        .bind(REF)
        .fetch_one(&h.pool)
        .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_quest_reward_participants")
            .fetch_one(&h.pool)
            .await?,
        2
    );
    let unlocks: Vec<String> = sqlx::query_scalar(
        "SELECT unlock_key FROM game_account_appearance_unlock_receipts ORDER BY unlock_key",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        unlocks,
        [
            "oteryn:outfit.citizen/quest-grant/0",
            "oteryn:outfit.citizen/quest-grant/1",
            "oteryn:outfit.citizen/quest-grant/2"
        ]
    );
    Ok(())
}
