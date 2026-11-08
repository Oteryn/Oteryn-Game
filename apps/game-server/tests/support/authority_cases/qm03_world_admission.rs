//! World predicates execute in the real fenced receipt writer, before progress changes.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{
    QuestCause, QuestTransitionOutcome, QuestTransitionRequest, grizzly::GrizzlyPremiumGate,
};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack,
    world_callbacks::{self},
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
    sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'world-policy-1') ON CONFLICT DO NOTHING")
        .bind(f::id(f::WORLD).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'world-policy-1',2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'qm03-owned-purchase-fixture')")
        .bind(f::id(f::WORLD).as_slice()).execute(&h.pool).await?;
    // Use the retained production quest binding unchanged, including level and storage gates.
    let callback = world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/qm03/anniversary")
        .ok_or("anniversary production callback")?;
    assert_eq!(callback.quest, "oteryn:quest.25_years_of_tibia_quest");
    assert!(callback.requires_world_admission);
    assert_eq!(callback.minimum_level, Some(20));
    let admission = callback.admission.as_ref().ok_or("production admission")?;
    assert!(admission.regular_blessings && admission.promotion && admission.premium);
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: callback.storage[0].track.clone(),
                quest: callback.quest.clone(),
                initial: -1,
                min: -1,
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
        cause: QuestCause::Use(CommandRef::new(
            session,
            CommandId::new(9810).map_err(f::debug)?,
        )),
    };
    let attempt = |revision| -> f::TestResult<_> {
        Ok(h.root.commit_character_world_callback_with_items(
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
    // Acquire permanent promotion through its real purchase/fee participant.
    // Do not disable production SQL guards or fabricate a promotion receipt.
    crate::character_promotion_fixture::purchase(h, 1).await?;
    assert!(
        matches!(
            attempt(2)?.await.map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "promotion alone does not supply blessings"
    );
    // Currency is fixture input only; every prerequisite grant uses its real owner.
    let mut funds = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *funds)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        crate::durability::item_fee_burn::test_support::item(
            120,
            "oteryn:item.tibia.i3043",
            "rev-1",
            100,
        ),
    ))
    .execute(&mut *funds)
    .await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,encode($5,'hex')::uuid)")
        .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(210).as_slice()).bind(f::id(120).as_slice()).bind(f::id(f::WORLD).as_slice()).bind(f::id(121).as_slice()).execute(&mut *funds).await?;
    funds.commit().await?;
    let mut revision = 2;
    buy_blessing(
        h,
        revision,
        0,
        "oteryn:npc.kawill",
        "oteryn:service.blessing.phoenix_prerequisite",
        0,
    )
    .await?;
    revision += 1;
    let blessings =
        oteryn_gameplay_content::content::blessing_catalogue::BlessingCatalogue::load()?;
    for (index, blessing) in blessings
        .blessings
        .iter()
        .filter(|b| b.storage_key != "oteryn:blessing.twist_of_fate")
        .enumerate()
    {
        buy_blessing(
            h,
            revision,
            u8::try_from(index + 1)?,
            blessing.npcs.iter().find(|npc| blessing.premium || oteryn_gameplay_content::content::npc_catalogue::premium_services::nonpremium_binding(&blessing.service, npc)).ok_or("blessing NPC binding")?,
            &blessing.service,
            blessing.price(50),
        )
        .await?;
        revision += 1;
    }
    let before = snapshot(h).await?;
    gate.0.store(false, Ordering::SeqCst);
    assert!(
        matches!(
            attempt(revision)?.await.map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "expired Premium is evaluated live"
    );
    assert_eq!(snapshot(h).await?, before);
    gate.0.store(true, Ordering::SeqCst);
    assert!(matches!(attempt(revision + 1)?.await,
        Err(crate::durability::character_progression::CharacterProgressionError::CharacterRevisionMismatch)),
        "live prerequisites cannot substitute for the current Character fence");
    assert_eq!(snapshot(h).await?, before);
    let committed = attempt(revision)?.await.map_err(f::debug)?;
    let QuestTransitionOutcome::Committed(receipt) = committed else {
        return Err(format!("admission {committed:?}").into());
    };
    gate.0.store(false, Ordering::SeqCst);
    consume_blessings_by_death(h, revision + 1).await?;
    let after = snapshot(h).await?;
    assert_eq!(
        attempt(revision)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(receipt)
    );
    assert_eq!(snapshot(h).await?, after);
    Ok(())
}

async fn buy_blessing(
    h: &f::Harness,
    revision: u64,
    index: u8,
    npc: &str,
    service: &str,
    price: u64,
) -> f::TestResult {
    use crate::durability::npc_purchase::{NpcPurchaseRequest, PurchaseOutcome, PurchaseService};
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    // The account port is controlled separately from durable promotion and blessings.
    let gate = PurchasePremium;
    let request = NpcPurchaseRequest {
        command: CommandRef::new(
            f::fence(revision)?.game_session_id,
            CommandId::new(100 + u64::from(index)).map_err(f::debug)?,
        ),
        occurrence: f::id(80 + index),
        transaction_id: f::id(90 + index),
        event_id: f::id(100 + index),
        account: f::id(40),
        npc_key: npc.into(),
        npc_identity: f::id(110 + index),
        npc_generation: 1,
        service: PurchaseService::Blessing(service.into()),
        coins: crate::durability::item_fee_burn::test_support::request(10 + index, price)?.change,
        expected_backpack: Some(f::id(210)),
        occurred_at_unix_ms: 1_790_000_000_000,
    };
    let sequencer =
        crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new();
    let result = sequencer
        .acquire(f::fence(revision)?.character_id)
        .await
        .commit_npc_purchase(
            &h.root,
            &authority,
            &h.node,
            f::fence(revision)?,
            request,
            &gate,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(result, PurchaseOutcome::Committed(_)),
        "{service}: {result:?}"
    );
    Ok(())
}

struct PurchasePremium;
impl crate::durability::npc_purchase::NpcPurchaseGate for PurchasePremium {
    async fn authorize(
        &self,
        _: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        _: [u8; 16],
        _: &crate::durability::npc_purchase::PurchaseService,
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(true)
    }
}

use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::durability::character_death::{
    CharacterDeathRequest, DeathCell, PlayerDeathOccurrence, PublicDeathCause,
};
use crate::foundation::{ChannelId, RuntimeScopeRefV1, WorldId};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
fn death_request() -> f::TestResult<CharacterDeathRequest<50>> {
    let context = ProgressionRevisionContext {
        profile: "profile-1".into(),
        ruleset: "ruleset-1".into(),
        content: "content-1".into(),
        simulation: "simulation-1".into(),
        evidence: "evidence-1".into(),
        declaration: "declaration-1".into(),
    };
    let mut thresholds = [LevelThreshold {
        level: 1,
        minimum_experience: ExactI64::new(0),
    }; 50];
    for (n, t) in thresholds.iter_mut().enumerate() {
        *t = LevelThreshold {
            level: u32::try_from(n + 1)?,
            minimum_experience: ExactI64::new(i64::try_from(n)? * 20),
        };
    }
    Ok(CharacterDeathRequest {
        // Invalid native key is retained as unpublishable provenance, never PvE.
        public_cause: Some(PublicDeathCause::native_creature("Dragon", "Dragon")),
        occurrence: PlayerDeathOccurrence::from_bytes(f::id(180)).map_err(f::debug)?,
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        held_blessings: Vec::new(),
        pvp_contributors: Vec::new(),
        death_cell: DeathCell {
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&f::id(f::WORLD)).map_err(f::debug)?,
                ChannelId::decode(&f::id(f::CHANNEL)).map_err(f::debug)?,
            ),
            spatial_position: vec![12, 13, 14, 7],
            map_revision: "death-map-1".into(),
        },
        respawn_position: b"temple:thais".to_vec(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds,
            terminal_exclusive_experience: ExactI64::new(1020),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        },
    })
}

async fn consume_blessings_by_death(h: &f::Harness, revision: u64) -> f::TestResult {
    let mut request = death_request()?;
    request.held_blessings = sqlx::query_scalar("SELECT blessing_key FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid ORDER BY blessing_key")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(request.held_blessings.len(), 7);
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    h.root
        .commit_character_death(&authority, &h.node, f::fence(revision)?, request)
        .await
        .map_err(f::debug)?;
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(f::id(f::CHARACTER).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(remaining, 0);
    Ok(())
}
