//! Two real Character sessions on independently assigned Channels share one World.
use super::chat_world_fixture::admit_second_node;
use super::*;
use crate::durability::content_activation::ContentActivationRequest;
use crate::durability::quest_world_callback::QuestWorldContext;
use oteryn_gameplay_world_change::mummys_curse::JarOutcome;

#[test]
fn cbchance_mummys_curse_shares_progress_and_keeps_character_cooldowns_separate() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async move |_| {
                reset();
                let h = Harness::create_with_admission_seed(
                    admin,
                    "cbchance_mummys_curse",
                    true,
                    false,
                )
                .await?;
                prepare_current_session(&h).await?;
                let (second_node, second) = admit_second_node(&h).await?;
                let first = Dependencies::new()?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let left = first.owner(&h, &authority);
                let right = second.owner_from(&h.root, &second_node, &authority);
                assert_ne!(left.channel_id, right.channel_id);
                let (left_fence, left_context) = activate(&h, &first, &left).await?;
                let (right_fence, right_context) = activate(&h, &second, &right).await?;
                let left_permit = left.spell_lane_permit().await.ok_or("left permit")?;
                let right_permit = right.spell_lane_permit().await.ok_or("right permit")?;
                let left_use = async |action, roll| {
                    h.root
                        .use_mummys_curse_jar(
                            &authority,
                            &h.node,
                            &left_permit,
                            left_fence,
                            left_context.clone(),
                            action,
                            roll,
                        )
                        .await
                };
                let right_use = async |action, roll| {
                    h.root
                        .use_mummys_curse_jar(
                            &authority,
                            &second_node,
                            &right_permit,
                            right_fence,
                            right_context.clone(),
                            action,
                            roll,
                        )
                        .await
                };

                let before = super::chat_world_fixture::clock(&h).await?;
                let (JarOutcome::Failed { retry_at }, 0) = left_use(50006, 10).await? else {
                    return Err("first player's failed roll must start the retry clock".into());
                };
                let after = super::chat_world_fixture::clock(&h).await?;
                assert!(((before + 1800) as u64..=(after + 1800) as u64).contains(&retry_at));
                assert_eq!(left_use(50006, 1).await?, (JarOutcome::CoolingDown, 0));
                assert_eq!(right_use(50007, 1).await?, (JarOutcome::Prerequisite, 0));
                assert_eq!(
                    right_use(50006, 1).await?,
                    (JarOutcome::Broken { progress: 1 }, 1)
                );
                assert_eq!(
                    left_use(50007, 1).await?,
                    (JarOutcome::Broken { progress: 2 }, 2)
                );
                assert_eq!(
                    right_use(50008, 1).await?,
                    (JarOutcome::Broken { progress: 3 }, 3)
                );
                assert_eq!(
                    left_use(50009, 1).await?,
                    (JarOutcome::Broken { progress: 4 }, 4)
                );
                assert_eq!(right_use(50010, 10).await?, (JarOutcome::FinalJar, 5));
                assert_eq!(left_use(50010, 1).await?, (JarOutcome::Refused, 5));
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_mummys_curse_retries")
                        .fetch_one(&h.pool)
                        .await?,
                    1
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Crystal jar test thread panicked")??;
    Ok(())
}

async fn activate(
    h: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<(
    crate::durability::character_progression::CurrentCharacterGameplayFence,
    QuestWorldContext,
)> {
    let world_id = deps.admitted.world_id;
    let channel_id = deps.admitted.channel_id;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .execute(&h.pool).await?;
    let context = {
        let runtime = deps.runtime.lock().await;
        let pin = runtime.content_pin();
        let server_artifact_digest = pin.server_artifact_digest();
        let client_artifact_digest = pin.client_artifact_digest();
        let frame_binding_digest = pin.frame_binding_digest();
        drop(runtime);
        assert!(
            h.root
                .record_content_activation(&ContentActivationRequest {
                    world_id,
                    channel_id,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest,
                    client_artifact_digest,
                    frame_binding_digest,
                })
                .await
                .map_err(debug)?
        );
        QuestWorldContext {
            map_revision: deps.room.movement_cells().scope().map_revision.clone(),
            content_revision: format!(
                "sha256:{}",
                server_artifact_digest
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            placement_context: frame_binding_digest.to_vec(),
            reset_epoch: 0,
            actor_placement: deps.actor.placement_identity(),
            movement_revision: 0,
            party: Vec::new(),
        }
    };
    let fence = owner
        .current_quest_fence(deps.admitted.game_session_id)
        .await
        .map_err(|()| "gameplay fence unavailable")?
        .ok_or("gameplay fence")?;
    Ok((fence, context))
}
