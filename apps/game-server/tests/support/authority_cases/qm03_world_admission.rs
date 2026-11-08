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
    run_admission(false)
}

#[test]
fn m03_time_machine_production_binding_admission_postgres() -> f::TestResult {
    run_admission(true)
}

fn run_admission(production: bool) -> f::TestResult {
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
                    let result = exercise(&h, production).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M03_PG_EXECUTED world_admission production={production} duration={:?}",
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

async fn exercise(h: &f::Harness, production: bool) -> f::TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M03_POSTGRES {version}");
    let (callback, catalogue) = definition(production)?;
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
        cause: {
            let command = CommandRef::new(session, CommandId::new(9810).map_err(f::debug)?);
            if callback.trigger == world_callbacks::Trigger::Use {
                QuestCause::Use(command)
            } else {
                QuestCause::Step(command)
            }
        },
    };
    let attempt_number = std::sync::atomic::AtomicUsize::new(0);
    let attempt = |revision| -> f::TestResult<_> {
        let number = attempt_number.fetch_add(1, Ordering::SeqCst) + 1;
        let operation = h.root.commit_fixture_world_admission(
            &authority,
            &h.node,
            f::fence(revision)?,
            request.clone(),
            Arc::clone(&catalogue),
            &lane,
            context.clone(),
            &gate,
            callback.clone(),
        );
        Ok(async move {
            let started = std::time::Instant::now();
            let result = operation.await;
            eprintln!(
                "M03_ATTEMPT production={production} n={number} revision={revision} elapsed={:?} result={result:?}",
                started.elapsed()
            );
            result
        })
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
    let (_, regular) = blessings
        .henricus_quote(20, &[])
        .ok_or("five regular blessings")?;
    assert_eq!(regular.len(), 5);
    for blessing in regular {
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,'qm03-fixture')").bind(f::id(f::CHARACTER).as_slice()).bind(&blessing.storage_key).execute(&h.pool).await?;
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
    if production {
        let returning = world_callbacks::load_for(&catalogue)?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/m03/25-years/time-machine-return")
            .ok_or("production return route")?;
        assert!(returning.admission.is_none());
        assert!(returning.minimum_level.is_none());
        assert!(returning.storage.iter().all(|s| s.preserve));
        let returned = h
            .root
            .commit_fixture_world_admission(
                &authority,
                &h.node,
                f::fence(2)?,
                QuestTransitionRequest {
                    transition_key: returning.key.clone(),
                    cause: QuestCause::Use(CommandRef::new(
                        session,
                        CommandId::new(9811).map_err(f::debug)?,
                    )),
                },
                Arc::clone(&catalogue),
                &lane,
                context.clone(),
                &gate,
                returning,
            )
            .await
            .map_err(f::debug)?;
        assert!(
            matches!(returned, QuestTransitionOutcome::Committed(_)),
            "return remains usable with expired Premium and no blessings: {returned:?}"
        );
        let progress: i64 = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
        )
        .bind(&callback.quest)
        .bind(&callback.storage[0].track)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            progress, 1,
            "return cannot reset the admitted entrance stage"
        );
        let completed: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
        )
        .bind(&callback.quest)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            completed, 0,
            "entrance/return do not complete the anniversary quest"
        );
    }
    Ok(())
}

fn definition(
    production: bool,
) -> f::TestResult<(world_callbacks::Callback, Arc<QuestStateCatalogue>)> {
    if production {
        let catalogue =
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(f::debug)?
                .catalogue()
                .clone();
        let callback = world_callbacks::load_for(&catalogue)?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/m03/25-years/time-machine")
            .ok_or("production time-machine binding")?;
        assert_eq!(callback.quest, "oteryn:quest.25_years_of_tibia_quest");
        assert_eq!(callback.trigger, world_callbacks::Trigger::Use);
        assert_eq!(callback.minimum_level, Some(20));
        assert_eq!(
            callback.admission,
            Some(Admission {
                regular_blessings: true,
                promotion: true,
                premium: true
            })
        );
        let native = callback
            .selector
            .native
            .as_ref()
            .ok_or("native time machine")?;
        assert_eq!(
            (
                native.position.x,
                native.position.y,
                native.position.floor,
                native.ordinal,
                native.appearance
            ),
            (32430, 32166, -8, 1, 25098)
        );
        assert_eq!(
            callback.teleport,
            Some(world_callbacks::Position {
                x: 33453,
                y: 31030,
                floor: -8
            })
        );
        assert!(
            !catalogue
                .transition(&callback.key)
                .ok_or("production operation")?
                .completes
        );
        return Ok((callback, Arc::new(catalogue)));
    }
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
    Ok((callback, catalogue))
}
