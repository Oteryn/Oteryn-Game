//! M02: ordered mutable Ground facts belong to the fenced quest transaction.
use super::engitem_reward_text::command;
use crate::bestiary_postgres_harness as fixture;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, world_callbacks};
use std::sync::Arc;

fn callback() -> fixture::TestResult<world_callbacks::Callback> {
    let text = serde_json::json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-callback/m02-ground-fixture", "quest":"oteryn:quest.ground_fixture",
        "trigger":"use","requires_world_admission":true,"selector":{"position":{"x":100,"y":100,"floor":-7}},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":[{"track":"oteryn:quest-progress/m02/stage","minimum":0,"maximum":0,"set":1}],
        "world_requirements":[{"position":{"x":100,"y":100,"floor":-7},
            "ground_stack":[{"item":"oteryn:item.tibia.i1735","quantity":1},
                     {"item":"oteryn:item.tibia.i3031","quantity":2}]}],
        "offerings":[],"return_on_failure":false,"failure_message":null,"sources":["M02 engine fixture"]
    }]}).to_string();
    Ok(serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(&text)?["callbacks"][0].clone(),
    )?)
}

#[test]
fn questpc2_ground_stack_predicates_use_current_fenced_custody() -> fixture::TestResult {
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
                    let h = fixture::Harness::create(admin, "questpc2_ground_stack", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M02_PG_EXECUTED ordered_ground duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "M02 PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M02_POSTGRES_VERSION={version}");
    let c = callback()?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: "oteryn:quest-progress/m02/stage".into(),
                quest: c.quest.clone(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![world_callbacks::primary_transition(&c)?],
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
    let position = [
        100_i32.to_be_bytes().as_slice(),
        100_i32.to_be_bytes().as_slice(),
        (-7_i16).to_be_bytes().as_slice(),
    ]
    .concat();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (id, key, quantity, ordinal) in [
        (90, "oteryn:item.tibia.i1735", 1_i64, 1_i64),
        (91, "oteryn:item.tibia.i3031", 2, 2),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,encode($5,'hex')::uuid)")
            .bind(fixture::id(id).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(key).bind(quantity).bind(fixture::id(92).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,$9)")
            .bind(fixture::id(id).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(fixture::CHANNEL).as_slice())
            .bind(&position).bind(b"ground-fixture".as_slice()).bind(&context.map_revision).bind(&context.content_revision)
            .bind(&context.placement_context).bind(ordinal).execute(&mut *tx).await?;
    }
    tx.commit().await?;
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
    let invoke = |revision, id| -> fixture::TestResult<_> {
        Ok(h.root.commit_fixture_world_callback(
            &authority,
            &h.node,
            fixture::fence(revision)?,
            QuestTransitionRequest {
                transition_key: c.key.clone(),
                cause: QuestCause::Use(command(id)?),
            },
            catalogue.clone(),
            &lane,
            context.clone(),
            c.clone(),
        ))
    };
    // Opposite order, despite identical per-definition totals, must refuse.
    assert!(matches!(
        invoke(1, 9700)?.await.map_err(fixture::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(h.root_revision().await?, "1");
    let mutate = |sql: &'static str| async move {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(sql).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    mutate("UPDATE game_item_ground_locations SET stack_ordinal=stack_ordinal+10").await?;
    mutate("UPDATE game_item_ground_locations SET stack_ordinal=13-stack_ordinal").await?;
    mutate("UPDATE game_item_instances SET quantity=1 WHERE definition_production_key='oteryn:item.tibia.i3031'").await?;
    assert!(matches!(
        invoke(1, 9701)?.await.map_err(fixture::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    mutate("UPDATE game_item_instances SET quantity=2 WHERE definition_production_key='oteryn:item.tibia.i3031'").await?;
    mutate("UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=2").await?;
    assert!(matches!(
        invoke(1, 9702)?.await.map_err(fixture::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    mutate("UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=1").await?;
    assert_eq!(h.root_revision().await?, "1");
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 0);
    assert!(matches!(
        invoke(1, 9703)?.await.map_err(fixture::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(h.root_revision().await?, "2");
    mutate("UPDATE game_item_ground_locations SET stack_ordinal=stack_ordinal+10").await?;
    mutate("UPDATE game_item_ground_locations SET stack_ordinal=13-stack_ordinal").await?;
    assert!(matches!(
        invoke(2, 9703)?.await.map_err(fixture::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(h.root_revision().await?, "2");
    let quantities: i64 =
        sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        quantities, 3,
        "read-only prerequisites never consume the stack"
    );
    Ok(())
}
