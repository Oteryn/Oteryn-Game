//! Crystal9f storkus.lua:99-199, SHA8f1b7eb209871b9a61e561714cf678e92640352cb96c49ac4540d555cbcf03cd.
//! Eligible Inquisition prerequisite only; Vampire rank, doll and access start absent.
use super::*;

#[test]
fn questl2_vampire_six_token_tiers_full_player_journey() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_quest_reward(admin, "l2_vampire", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED l2-vampire-binding server_version={version}");
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let tiers = callbacks
            .iter()
            .filter(|c| c.key.contains("/l2-vampire-tier-"))
            .count();
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE l2-vampire duration={:?}",
            started.elapsed()
        );
        assert_eq!(tiers, 6, "Storkus six token-tier bindings are absent");
        Ok(())
    })?;
    scenario("qengine-rewards-l2-vampire")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::questl2_journeys::{quantity, use_at};
    use oteryn_gameplay_quests::quest::world_callbacks::Offering;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    use std::collections::BTreeMap;
    const RANK: &str = "oteryn:quest-progress/quest/u8_2/vampire_hunter_quest/rank";
    const DOOR: &str = "oteryn:quest-progress/quest/u8_2/vampire_hunter_quest/door";
    const PRIOR: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
    const ELIGIBLE: &str = "oteryn:quest-transition/l2-vampire-proof/eligible-inquisition";
    let callbacks: Vec<_> = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .filter(|c| c.key.contains("/l2-vampire-tier-"))
        .collect();
    assert_eq!(callbacks.len(), 6);
    let canonical = owner.quest_catalogue.clone().ok_or("catalogue")?;
    let mut tracks = BTreeMap::new();
    let mut transitions = Vec::new();
    for callback in &callbacks {
        let transition = canonical
            .transition(&callback.key)
            .cloned()
            .ok_or("tier transition")?;
        for key in transition
            .effects
            .iter()
            .map(|e| e.track.as_str())
            .chain(callback.requires.iter().map(|r| r.track.as_str()))
        {
            tracks.insert(
                key.to_owned(),
                canonical.track(key).cloned().ok_or("tier track")?,
            );
        }
        transitions.push(transition);
    }
    transitions.push(QuestTransition {
        key: ELIGIBLE.into(),
        quest: "oteryn:quest.the_inquisition_quest".into(),
        effects: vec![QuestEffect {
            track: PRIOR.into(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(11),
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
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let door = Position {
        x: 32953,
        y: 31460,
        floor: -9,
    };
    use_at(owner, admitted, door, UseDisposition::Locked).await?;
    let [storkus] =
        super::super::classic_quests::install_nearby(owner, admitted, ["oteryn:npc.storkus"])
            .await?;
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
                transition_key: ELIGIBLE.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(12000).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("catalogue")?,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_tracks WHERE quest_key='oteryn:quest.vampire_hunter_quest'").fetch_one(&h.pool).await?,0);
    let before: i64 =
        sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    for (index, callback) in callbacks.iter().enumerate() {
        assert_eq!(
            wire_reward(owner, admitted, storkus, callback, false)
                .await?
                .0,
            NpcIntentDisposition::Rejected,
            "missing tokens must refuse tier {}",
            index + 1
        );
        assert_eq!(quantity(h, 8191).await?, 0);
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
                    x: 32952,
                    y: 31460,
                    floor: -9,
                },
                item: "oteryn:item.tibia.i8192".into(),
                count: callback.take[0].count,
            },
            12100 + u64::try_from(index)?,
            true,
        )
        .await?;
        assert_eq!(
            wire_grant(owner, admitted, storkus, callback).await?.0,
            NpcIntentDisposition::Ok
        );
        assert_eq!(quantity(h, 8192).await?, 0);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT value FROM game_character_quest_tracks WHERE track_key=$1"
            )
            .bind(RANK)
            .fetch_one(&h.pool)
            .await?,
            i64::try_from(index + 1)?
        );
    }
    let after: i64 =
        sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(after - before, 186000);
    assert_eq!(quantity(h, 8191).await?, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT value FROM game_character_quest_tracks WHERE track_key=$1"
        )
        .bind(DOOR)
        .fetch_one(&h.pool)
        .await?,
        1
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/razing'").fetch_one(&h.pool).await?,1);
    use_at(owner, admitted, door, UseDisposition::Committed).await?;
    Ok(())
}
