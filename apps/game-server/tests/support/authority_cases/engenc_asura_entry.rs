//! Asura s1 uses the production bound world writer, with live Premium and replay.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{
    QuestCause, QuestTransitionOutcome, QuestTransitionRequest, grizzly::GrizzlyPremiumGate,
};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

struct Premium(AtomicBool);
impl GrizzlyPremiumGate for Premium {
    async fn task_permitted(
        &self,
        _: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

#[test]
fn engenc_asura_native_entry_premium_fence_and_replay() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "engenc_asura_entry", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M30_ASURA_PG_EXECUTED world_admission duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M30_ASURA PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let bindings = Bindings::parse(include_bytes!(
        "../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
    ))?;
    bindings.verify_embedded_recipe_evidence()?;
    let callback = bindings.world_callbacks().next().ok_or("Asura entry")?;
    let catalogue =
        Arc::new(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("base")?)?);
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    assert!(
        h.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: f::scope()?.world_id(),
                    channel_id: crate::foundation::ChannelId::decode(&f::id(f::CHANNEL))
                        .map_err(f::debug)?,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: native.source_digest(),
                    client_artifact_digest: pin.identity().client_artifact_digest(),
                    frame_binding_digest: pin.frame_binding().digest()
                }
            )
            .await
            .map_err(f::debug)?
    );
    let context = QuestWorldContext {
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
        actor_placement: f::id(17),
        movement_revision: 1,
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let lane = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(f::scope()?)
        .await
        .map_err(f::debug)?;
    let gate = Premium(AtomicBool::new(false));
    let session = GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?;
    let request = QuestTransitionRequest {
        transition_key: callback.key.clone(),
        cause: QuestCause::Step(CommandRef::new(
            session,
            CommandId::new(9810).map_err(f::debug)?,
        )),
    };
    let attempt = |revision| -> f::TestResult<_> {
        Ok(h.root.commit_character_world_callback_with_items_and_premium(
            &authority,
            &h.node,
            f::fence(revision)?,
            request.clone(),
            Arc::clone(&catalogue),
            &lane,
            context.clone(),
            None,
            None,
            &gate,
        ))
    };
    let before = h.root_revision().await?;
    assert!(matches!(
        attempt(1)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(h.root_revision().await?, before);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    gate.0.store(true, Ordering::SeqCst);
    let mut stale = f::fence(1)?;
    stale.character_lease_generation += 1;
    assert!(matches!(
        h.root
            .commit_character_world_callback_with_items_and_premium(
                &authority,
                &h.node,
                stale,
                request.clone(),
                Arc::clone(&catalogue),
                &lane,
                context.clone(),
                None,
                None,
                &gate,
            )
            .await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)
    ));
    assert_eq!(h.root_revision().await?, before);
    assert!(matches!(
        attempt(1)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(matches!(
        attempt(2)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(h.root_revision().await?, "2");
    let values: Vec<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 ORDER BY track_key",
    )
    .bind(&callback.quest)
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(values, [1, 1]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_states WHERE completed_character_revision IS NOT NULL").fetch_one(&h.pool).await?, 0);
    Ok(())
}
