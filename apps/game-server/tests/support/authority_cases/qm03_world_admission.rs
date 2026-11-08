//! World predicates execute in the real fenced receipt writer, before progress changes.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{
    QuestCause, QuestTransitionOutcome, QuestTransitionRequest, grizzly::GrizzlyPremiumGate,
};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack,
    world_callbacks::{self, Admission, Storage},
};
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
fn qm03_world_admission_live_blessings_promotion_premium_fences_and_replay() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "qm03_world_admission", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M03_PG_EXECUTED world_admission duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M03 PG thread failed")?
        .map_err(Into::into)
}

async fn snapshot(h: &f::Harness) -> f::TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('revision',(SELECT character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid),'tracks',(SELECT count(*) FROM game_character_quest_tracks),'receipts',(SELECT count(*) FROM game_character_quest_receipts))")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let mut callback = world_callbacks::load()?
        .into_iter()
        .find(|c| !c.storage.is_empty() && !c.reversible && c.effects.is_empty())
        .ok_or("fixture callback")?;
    callback.key = "oteryn:quest-callback/qm03-admission".into();
    callback.quest = "oteryn:quest.qm03_admission".into();
    callback.trigger = world_callbacks::Trigger::StepIn;
    callback.admission = Some(Admission {
        regular_blessings: true,
        promotion: true,
        premium: true,
    });
    callback.minimum_level = None;
    callback.world_change = None;
    callback.requires_world_admission = false;
    callback.native_rule = None;
    callback.state_ops.clear();
    callback.offerings.clear();
    callback.item_on_target = None;
    callback.storage = vec![Storage {
        track: "oteryn:quest-progress/qm03-admission/stage".into(),
        minimum: 0,
        maximum: 0,
        set: 1,
        preserve: false,
        increment: false,
        at_least: false,
    }];
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: callback.storage[0].track.clone(),
                quest: callback.quest.clone(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![world_callbacks::primary_transition(&callback).map_err(f::debug)?],
        )
        .map_err(f::debug)?,
    );
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
        Ok(h.root.commit_fixture_world_admission(
            &authority,
            &h.node,
            f::fence(revision)?,
            request.clone(),
            Arc::clone(&catalogue),
            &lane,
            context.clone(),
            &gate,
            callback.clone(),
        ))
    };
    let before = snapshot(h).await?;
    assert!(
        matches!(
            attempt(1)?.await.map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "no live predicates may be ignored"
    );
    assert_eq!(snapshot(h).await?, before);
    gate.0.store(true, Ordering::SeqCst);
    assert!(
        matches!(
            attempt(1)?.await.map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "Premium alone does not supply promotion/blessings"
    );
    // Fixture setup uses the admitted database role; production mutations remain owned by purchase/death.
    sqlx::raw_sql("ALTER TABLE game_character_promotions DISABLE TRIGGER ALL; ALTER TABLE game_character_blessings DISABLE TRIGGER ALL").execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_promotions(character_id,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)").bind(f::id(f::CHARACTER).as_slice()).bind(f::id(91).as_slice()).execute(&h.pool).await?;
    assert!(
        matches!(
            attempt(1)?.await.map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "promotion alone does not supply blessings"
    );
    let blessings =
        oteryn_gameplay_content::content::blessing_catalogue::BlessingCatalogue::load()?;
    for blessing in &blessings.blessings {
        if blessing.storage_key != "oteryn:blessing.twist_of_fate" {
            sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,'qm03-fixture')").bind(f::id(f::CHARACTER).as_slice()).bind(&blessing.storage_key).execute(&h.pool).await?;
        }
    }
    gate.0.store(false, Ordering::SeqCst);
    assert!(
        matches!(
            attempt(1)?.await.map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "expired Premium is evaluated live"
    );
    assert_eq!(snapshot(h).await?, before);
    gate.0.store(true, Ordering::SeqCst);
    assert!(matches!(attempt(2)?.await,
        Err(crate::durability::character_progression::CharacterProgressionError::CharacterRevisionMismatch)),
        "live prerequisites cannot substitute for the current Character fence");
    assert_eq!(snapshot(h).await?, before);
    let committed = attempt(1)?.await.map_err(f::debug)?;
    let QuestTransitionOutcome::Committed(receipt) = committed else {
        return Err(format!("admission {committed:?}").into());
    };
    let after = snapshot(h).await?;
    gate.0.store(false, Ordering::SeqCst);
    sqlx::query("DELETE FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice())
        .execute(&h.pool)
        .await?;
    assert_eq!(
        attempt(1)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(receipt)
    );
    assert_eq!(snapshot(h).await?, after);
    Ok(())
}
