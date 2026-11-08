//! Bound World actor predicates use the live Character state and real receipt owner.
use super::engitem_reward_text::command;
use crate::bestiary_postgres_harness as fixture;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, world_callbacks};
use std::sync::Arc;

fn callback() -> fixture::TestResult<world_callbacks::Callback> {
    let text = serde_json::json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-callback/m03-ground-fixture", "quest":"oteryn:quest.ground_fixture",
        "trigger":"use","requires_world_admission":true,"selector":{"position":{"x":100,"y":100,"floor":-7}},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":[{"track":"oteryn:quest-progress/m03/stage","minimum":0,"maximum":0,"set":1}],
        "minimum_level":51,
        "offerings":[],"return_on_failure":false,"failure_message":null,"sources":["M03 engine fixture"]
    }]}).to_string();
    Ok(serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(&text)?["callbacks"][0].clone(),
    )?)
}

#[test]
fn questpc2_world_actor_bound_world_actor_uses_current_level() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, "questpc2_world_actor", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M03_PG_EXECUTED world_actor duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "M03 PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M03_POSTGRES_VERSION={version}");
    let denied = callback()?;
    let mut accepted = denied.clone();
    accepted.key.push_str("-accepted");
    accepted.minimum_level = Some(50);
    let c = denied.clone();
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: "oteryn:quest-progress/m03/stage".into(),
                quest: c.quest.clone(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![
                world_callbacks::primary_transition(&denied)?,
                world_callbacks::primary_transition(&accepted)?,
            ],
        )
        .map_err(fixture::debug)?,
    );
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            fixture::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
    h.root
        .record_content_activation(
            &crate::durability::content_activation::ContentActivationRequest {
                world_id: fixture::scope()?.world_id(),
                channel_id: crate::foundation::ChannelId::decode(&fixture::id(fixture::CHANNEL))
                    .map_err(fixture::debug)?,
                activation_sequence: 1,
                previous_sequence: None,
                server_artifact_digest: native.source_digest(),
                client_artifact_digest: pin.identity().client_artifact_digest(),
                frame_binding_digest: pin.frame_binding().digest(),
            },
        )
        .await
        .map_err(fixture::debug)?;
    let context = crate::durability::quest_world_callback::QuestWorldContext {
        map_revision: "map-1".into(),
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
        actor_placement: fixture::id(17),
        movement_revision: 1,
    };
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let lane =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fixture::scope()?)
            .await
            .map_err(fixture::debug)?;
    let invoke = |revision, id, callback: world_callbacks::Callback| -> fixture::TestResult<_> {
        Ok(h.root.commit_fixture_world_callback(
            &authority,
            &h.node,
            fixture::fence(revision)?,
            QuestTransitionRequest {
                transition_key: callback.key.clone(),
                cause: QuestCause::Use(command(id)?),
            },
            catalogue.clone(),
            &lane,
            context.clone(),
            callback,
        ))
    };
    assert!(matches!(
        invoke(1, 9800, denied)?.await.map_err(fixture::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(h.root_revision().await?, "1");
    assert!(matches!(
        invoke(1, 9801, accepted.clone())?
            .await
            .map_err(fixture::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(h.root_revision().await?, "2");
    assert!(matches!(
        invoke(2, 9801, accepted)?.await.map_err(fixture::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(h.root_revision().await?, "2");
    Ok(())
}
