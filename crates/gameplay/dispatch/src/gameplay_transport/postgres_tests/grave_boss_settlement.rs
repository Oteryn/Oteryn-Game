use super::fixture::{Harness, debug, id};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::foundation::{
    CharacterId, CharacterLease, CommandId, CommandRef, ExactActorRef, GameSessionId,
    MovementLocalPosition, NodeId, OwnerDamageCommand,
};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
    death_objectives::{self, DeathObjective},
    grave_danger, loader,
};
use std::sync::Arc;

const ZELOS: &str = "oteryn:creature.king_zelos";
const SEED: &str = "oteryn:quest-transition/grave-danger/fixture-prerequisites";
const KNIGHT: &str = "oteryn:creature.count_vlarkorth";

#[test]
fn zelos_credits_each_eligible_damage_contributor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(&admin, "gz_contributors", ZELOS, false)
}

#[test]
fn grave_boss_credit_waits_for_private_reward_insert_and_retries_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(&admin, "gz_zelos_fault", ZELOS, true)?;
    scenario(&admin, "gz_knight_fault", KNIGHT, true)
}

fn scenario(admin: &str, tag: &'static str, creature: &'static str, fault: bool) -> TestResult {
    let admin = admin.to_owned();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            let case = async {
                let h = Harness::create_with_admission_seed(admin, tag, false, false).await?;
                let result = exercise(&h, creature, fault).await;
                h.cleanup().await?;
                result
            };
            runtime
                .block_on(Box::pin(case))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Grave Danger settlement scenario panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &Harness, creature: &str, fault: bool) -> TestResult {
    prepare_current_session(h).await?;
    let evidence = super::house_transfer_fixtures::RecoverySource::start(h).await?;
    let peer = super::boss_rewards_fixtures::privacy::admit_gameplay_peer(h, &evidence.evidence, 1)
        .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let objective = death_objectives::load()
        .map_err(debug)?
        .iter()
        .find(|objective| objective.creature == creature)
        .ok_or("Grave Danger objective")?;
    let spawn = objective.guardian.as_ref().ok_or("boss spawn")?.position;
    let at = MovementLocalPosition {
        x: i32::from(spawn.x),
        y: i32::from(spawn.y),
        floor: -i16::from(spawn.floor),
    };

    let pin = crate::foundation::ChannelContentPin::from_activation(
        deps.admitted.world_id,
        activated.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        activated.frame_binding().digest(),
        [4; 32],
        (at.x, at.y, at.floor),
    );
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        pin,
    )
    .map_err(debug)?;
    runtime.use_classic_creature_deaths().map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    for (session, x) in [(deps.admitted.game_session_id, at.x - 1), (peer, at.x + 1)] {
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, MovementLocalPosition { x, ..at })
            .map_err(debug)?;
        if session == deps.admitted.game_session_id {
            deps.actor = actor;
            deps.admitted.runtime_actor = Some(actor);
        }
    }
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    let catalogue = loader::load_embedded_quest_state("content-1").map_err(debug)?;
    owner.quest_catalogue = Some(Arc::new(
        catalogue
            .catalogue()
            .with_fixture_transition(QuestTransition {
                key: SEED.into(),
                quest: objective.quest.clone(),
                completes: false,
                experience: None,
                world_change: None,
                effects: [
                    (objective.stage.clone(), objective.stage_value),
                    (objective.counter.clone(), 0),
                ]
                .into_iter()
                .map(|(track, value)| QuestEffect {
                    track,
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(value),
                })
                .collect(),
            })
            .map_err(debug)?,
    ));
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "progression fence")?
            .ok_or("progression fence")?
            .item_fence(),
    );
    let request = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    owner
        .root
        .initialize_admission_equipment(
            owner.character,
            owner.holder,
            &admitted.item_fence.ok_or("equipment fence")?,
            active.identity().server_artifact_digest(),
            request.operation(),
        )
        .await
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    assert!(
        owner
            .progression_binding(admitted.game_session_id)
            .is_some()
    );
    let session = admitted.game_session_id;
    let primary = prepare_player(&deps, &owner, session).await?;
    let other = prepare_player(&deps, &owner, peer).await?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    {
        let mut runtime = owner.runtime.lock().await;
        let policy = runtime.companion_policy(creature).map_err(debug)?;
        let health = policy.maximum_health;
        let target = runtime
            .admit_encounter_actor(creature, health, at)
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, creature)
            .map_err(debug)?;
        if !fault {
            damage(&mut runtime, target, other, peer, creature, health / 3, now)?;
        }
        damage(
            &mut runtime,
            target,
            primary,
            session,
            creature,
            if fault { health } else { health - health / 3 },
            now,
        )?;
        let lease = runtime
            .bound_attacker_lease(primary, session)
            .map_err(debug)?;
        record_creature_kill(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: &queue,
                rewards: native.reward_table(),
                charms: None,
            },
            target,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                actor: primary,
                session,
            },
            now,
            Some(observed),
        );
    }
    assert!(queue.has_session(session));
    if fault {
        for statement in [
            "CREATE SEQUENCE grave_chest_fault",
            "CREATE FUNCTION grave_chest_fault() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('grave_chest_fault'); RAISE EXCEPTION 'Grave Danger chest fixture' USING ERRCODE='23514'; END $$",
            "CREATE TRIGGER grave_chest_fault BEFORE INSERT ON game_character_boss_reward_chests FOR EACH ROW EXECUTE FUNCTION grave_chest_fault()",
        ] {
            sqlx::query(statement).execute(&h.pool).await?;
        }
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Deferred
        );
        let attempted: bool = sqlx::query_scalar("SELECT is_called FROM grave_chest_fault")
            .fetch_one(&h.pool)
            .await?;
        assert!(
            attempted,
            "the real private reward insert must reach the fault"
        );
        assert_eq!(progress(h, id(41), objective).await?, (0, -1));
        assert_eq!(rewards(h).await?, (0, 0, 0, 0));
        assert_eq!(credits(h, objective).await?, 0);
        sqlx::query("DROP TRIGGER grave_chest_fault ON game_character_boss_reward_chests")
            .execute(&h.pool)
            .await?;
    }
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    let expected = (1, if creature == ZELOS { 4 } else { -1 });
    assert_eq!(
        progress(h, id(101), objective).await?,
        if fault { (0, -1) } else { expected }
    );
    assert_eq!(progress(h, id(41), objective).await?, expected);
    let counts = rewards(h).await?;
    let contributors = if fault { 1 } else { 2 };
    assert_eq!(counts, (1, contributors, 1, 1));
    assert_eq!(credits(h, objective).await?, contributors);
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    assert_eq!(rewards(h).await?, counts);
    assert_eq!(credits(h, objective).await?, contributors);
    Ok(())
}

async fn prepare_player(
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult<ExactActorRef> {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let actor = runtime
            .positioned_actor_census()
            .map_err(debug)?
            .into_iter()
            .find(|(_, _, observed)| *observed == Some(session))
            .ok_or("positioned player")?
            .0;
        let lease = CharacterLease::new(
            CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            fence.character_lease_generation,
        )
        .map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, lease)
            .map_err(debug)?;
        actor
    };
    deps.initialize_player(owner, actor, session).await;
    let admitted = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        controller: None,
        ..deps.admitted
    };
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: SEED.into(),
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(1).map_err(debug)?,
                )),
            },
            Arc::clone(owner.quest_catalogue.as_ref().ok_or("quest catalogue")?),
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "{outcome:?}"
    );
    owner.admit_quest_session(&admitted).await;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    Ok(actor)
}

fn damage(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    target: ExactActorRef,
    actor: ExactActorRef,
    session: GameSessionId,
    creature: &str,
    amount: i64,
    now: u64,
) -> TestResult {
    runtime
        .borrow_exact_actor_commit_at(now)
        .commit_damage_for_bound_attacker(
            target,
            actor,
            CommandRef::new(session, CommandId::new(10).map_err(debug)?),
            0,
            OwnerDamageCommand {
                target: creature.as_bytes(),
                occurrence: session.as_bytes(),
                binding: b"grave-boss-settlement",
                damage: amount,
            },
        )
        .map_err(debug)?;
    Ok(())
}

async fn progress(
    h: &Harness,
    character: [u8; 16],
    objective: &DeathObjective,
) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT COALESCE(max(value) FILTER (WHERE track_key=$2),-1), COALESCE(max(value) FILTER (WHERE track_key=$3),-1) FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key IN ($2,$3)")
        .bind(character.as_slice()).bind(&objective.counter).bind(grave_danger::QUESTLINE).fetch_one(&h.pool).await?)
}

async fn credits(h: &Harness, objective: &DeathObjective) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&objective.key)
    .fetch_one(&h.pool)
    .await?)
}

async fn rewards(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_boss_reward_deaths), (SELECT count(*) FROM game_character_boss_reward_chests), (SELECT count(*) FROM game_party_kill_xp_receipts), (SELECT count(*) FROM game_character_xp_receipts)").fetch_one(&h.pool).await?)
}
