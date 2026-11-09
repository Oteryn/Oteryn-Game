//! Source Siflind grant -> three registered seals, with real native vial MINT.
//! Earlier quest missions and map-click admission are separate qualifications.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::durability::spell_owner_commit::SpellLanePermit;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack, ice_islands_seals as seals, reward_callbacks, world_callbacks,
};
use std::sync::Arc;

#[test]
fn engnpc2_paint_postgres_native_grant_three_seals_and_replay() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        f::runtime().map_err(f::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = f::Harness::create(admin, "engnpc2_paint_grant", true).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("PG_EXECUTED engnpc2_paint_postgres_native_grant_three_seals_and_replay duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(f::debug)
    })?.join().map_err(|_| "paint grant PG thread failed")?.map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let grant = reward_callbacks::load()
        .map_err(f::debug)?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-reward/ice-islands/siflind-paint")
        .ok_or("Siflind paint callback")?;
    let callbacks: Vec<_> = [0, 63, 197]
        .into_iter()
        .map(|seal| {
            world_callbacks::load()?
                .into_iter()
                .find(|c| c.native_rule == Some(world_callbacks::NativeRule::IceSealPaint { seal }))
                .ok_or("registered seal".into())
        })
        .collect::<Result<_, String>>()
        .map_err(f::debug)?;
    // Explicit prior mission snapshot; no historical vial or direct custody writes.
    let tracks = seals::TRACKS
        .into_iter()
        .zip([(7, 0, 9), (0, 0, (1_i64 << 48) - 1), (0, 0, 2)])
        .map(|(key, (initial, min, max))| QuestTrack {
            key: key.into(),
            quest: seals::QUEST.into(),
            initial,
            min,
            max,
        })
        .collect();
    let mut transitions = vec![reward_callbacks::primary_transition(&grant).map_err(f::debug)?];
    transitions.extend(
        callbacks
            .iter()
            .map(world_callbacks::primary_transition)
            .collect::<Result<Vec<_>, _>>()
            .map_err(f::debug)?,
    );
    let catalogue =
        Arc::new(QuestStateCatalogue::new("content-1", tracks, transitions).map_err(f::debug)?);
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native content")?;
    let weights = super::engitem_reward_text::Weights(native);
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let request = QuestTransitionRequest {
        transition_key: grant.key.clone(),
        cause: QuestCause::Command(super::engitem_reward_text::command(12000)?),
    };
    let grant_fence = f::fence(1)?;
    let apply = || {
        h.root.commit_fixture_quest_reward(
            &authority,
            &h.node,
            grant_fence,
            request.clone(),
            catalogue.clone(),
            admission,
            grant.clone(),
        )
    };
    assert!(
        matches!(
            apply().await.map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "no backpack must not partly grant paint"
    );
    assert_eq!(h.count("game_character_quest_receipts").await?, 0);
    super::engitem_reward_text::equip_backpack(h, &authority, native, &weights).await?;
    let outcome = apply().await.map_err(f::debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "{outcome:?}"
    );
    let progress: Vec<(String, i64)> = sqlx::query_as(
        "SELECT track_key,value FROM game_character_quest_tracks ORDER BY track_key",
    )
    .fetch_all(&h.pool)
    .await?;
    assert!(progress.contains(&(seals::QUESTLINE.into(), 8)));
    assert!(progress.contains(&(seals::MISSION04.into(), 1)));
    let item: String = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1 AND quantity=1")
        .bind(seals::PAINT).fetch_one(&h.pool).await?;
    let stored: (String, String) = sqlx::query_as("SELECT definition_family,definition_revision_ref FROM game_item_instances WHERE item_instance_id=$1::uuid")
        .bind(&item).fetch_one(&h.pool).await?;
    let reference = native
        .item_index()
        .ok_or("native item index")?
        .definition_ref(&stored.0, seals::PAINT, &stored.1)
        .ok_or("current minted paint definition")?;
    let subject = oteryn_protocol_oteryn::world_object::UseTarget::ItemByDefinition(reference);
    let instance = *sqlx::types::Uuid::parse_str(&item)?.as_bytes();
    assert!(
        super::engnpc2_corpse_routing::world_quest_tools::matches_source(
            &subject,
            None,
            instance,
            Some(reference)
        )
    );
    assert!(
        !super::engnpc2_corpse_routing::world_quest_tools::matches_source(
            &subject, None, instance, None
        )
    );
    let atomic: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_character_quest_receipts q ON m.xmin=q.xmin WHERE q.transition_key=$1")
        .bind(&grant.key).fetch_one(&h.pool).await?;
    assert_eq!(
        atomic, 1,
        "paint MINT and both progress changes share one transaction"
    );
    assert!(matches!(
        apply().await.map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(h.count("game_reward_claim_mint_receipts").await?, 1);
    let repeat = QuestTransitionRequest {
        cause: QuestCause::Command(super::engitem_reward_text::command(12010)?),
        ..request.clone()
    };
    let outcome = h
        .root
        .commit_fixture_quest_reward(
            &authority,
            &h.node,
            f::fence(2)?,
            repeat,
            catalogue.clone(),
            admission,
            grant.clone(),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Refused(_)),
        "new command must not repeat the phase-seven grant: {outcome:?}"
    );
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_reward_claim_mint_receipts").await?, 1);

    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).bind(native.source_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    let permit = SpellLanePermit::of_fresh_scope(f::fence(2)?.runtime_scope)
        .await
        .map_err(f::debug)?;
    let context = QuestWorldContext {
        map_revision: "paint-grant-fixture-map".into(),
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        placement_context: pin.frame_binding().digest().to_vec(),
        reset_epoch: 0,
        actor_placement: f::id(7),
        movement_revision: 1,
    };
    for (index, callback) in callbacks.iter().enumerate() {
        let request = QuestTransitionRequest {
            transition_key: callback.key.clone(),
            cause: QuestCause::Use(super::engitem_reward_text::command(12001 + index as u64)?),
        };
        let outcome = h
            .root
            .commit_fixture_ground_callback(
                &authority,
                &h.node,
                f::fence(2 + index as u64)?,
                request,
                catalogue.clone(),
                Some((&permit, context.clone())),
                callback.clone(),
            )
            .await
            .map_err(f::debug)?;
        assert!(
            matches!(outcome, QuestTransitionOutcome::Committed(_)),
            "{outcome:?}"
        );
        let state: (i64,i16,i64) = sqlx::query_as("SELECT quantity,lifecycle,(SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=$1::uuid")
            .bind(&item).fetch_one(&h.pool).await?;
        assert_eq!(state, if index < 2 { (1, 1, 1) } else { (0, 2, 0) });
    }
    let state: Vec<(String, i64)> = sqlx::query_as(
        "SELECT track_key,value FROM game_character_quest_tracks ORDER BY track_key",
    )
    .fetch_all(&h.pool)
    .await?;
    assert!(state.contains(&(seals::QUESTLINE.into(), 9)));
    assert!(state.contains(&(seals::MISSION04.into(), 2)));
    assert_eq!(h.count("game_quest_callback_consumptions").await?, 1);
    assert_eq!(h.root_revision().await?, "5");
    Ok(())
}
