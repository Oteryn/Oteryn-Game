//! Crystal creaturescripts_kill.lua:10-39 through native death capture and settlement.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::combat::CapturedRewardPrincipal;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{
    ChannelRuntimeV1, CharacterId, CharacterLease, CommandId, CommandRef, MovementLocalPosition,
    NodeId, OwnerDamageCommand,
};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use std::sync::Arc;

const TRACK: &str =
    "oteryn:quest-progress/quest/u11_80/the_secret_library/falcon_bastion/killing_bosses";

#[test]
fn seclib1_falcon_damage_credit_raises_each_contributors_rank() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(
        "seclib1_falcon_damage_credit_raises_each_contributors_rank",
        false,
    )
}

pub(super) fn scenario(name: &'static str, oberon: bool) -> TestResult {
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "seclib1_encounter", false, false)
                .await?;
            let result = exercise(&h, oberon).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn exercise(h: &Harness, oberon: bool) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activation) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("generation")?;
    let native = generation.native_gameplay().ok_or("native policies")?;
    let at = if oberon {
        MovementLocalPosition {
            x: 33364,
            y: 31322,
            floor: 9,
        }
    } else {
        MovementLocalPosition {
            x: 33366,
            y: 31391,
            floor: 7,
        }
    };
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        activation.frame_binding().digest(),
        [4; 32],
        (at.x, at.y, at.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        64,
        pin,
    )
    .map_err(debug)?;
    runtime.use_classic_creature_deaths().map_err(debug)?;
    let session = deps.admitted.game_session_id;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    deps.actor = actor;
    runtime
        .initialize_pinned_test_position(actor, at)
        .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(Arc::new(
        load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(activation.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            generation.identity().server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    owner.admit_quest_session(&admitted).await;
    let (other, second) = super::grizzly_boss::vacancy::second_player(
        &owner,
        &deps,
        h,
        MovementLocalPosition { x: at.x - 1, ..at },
    )
    .await?;
    if oberon {
        return super::seclib1_oberon::exercise(&owner, admitted, second, at).await;
    }
    let other_session = second.game_session_id;
    let other_fence = second.item_fence.ok_or("second fence")?;
    let queue = owner.states.attack.lock().await.kills().clone();

    for (n, (boss, assist, expected)) in [
        ("grand_commander_soeren", false, [Some(1), None]),
        ("grand_chaplain_gaunder", true, [Some(3), Some(3)]),
        ("preceptor_lazare", true, [Some(3), Some(3)]),
        ("grand_canon_dominus", false, [Some(4), Some(3)]),
    ]
    .into_iter()
    .enumerate()
    {
        owner.refresh_quest_session(session, false).await;
        owner.refresh_quest_session(other_session, false).await;
        assert!(owner.refresh_source_party_presence(actor, session).await);
        let key = format!("oteryn:creature.{boss}");
        let now = owner.owner_now();
        {
            let mut runtime = owner.runtime.lock().await;
            let policy = runtime.companion_policy(&key).map_err(debug)?;
            let target = runtime
                .admit_encounter_actor(
                    &key,
                    policy.maximum_health,
                    MovementLocalPosition { x: at.x + 1, ..at },
                )
                .map_err(debug)?;
            runtime
                .install_creature_policy(target, &key)
                .map_err(debug)?;
            let health = runtime.companion_snapshot(target).map_err(debug)?.health;
            let lease = runtime
                .bound_attacker_lease(actor, session)
                .map_err(debug)?;
            let command = u64::try_from(n)? + 20;
            for (source, caster, damage) in [
                (other, other_session, i64::from(assist)),
                (actor, session, health - i64::from(assist)),
            ] {
                if damage > 0 {
                    runtime
                        .borrow_exact_actor_commit_at(now.get() / 1000)
                        .commit_damage_for_bound_attacker(
                            target,
                            source,
                            CommandRef::new(caster, CommandId::new(command).map_err(debug)?),
                            0,
                            OwnerDamageCommand {
                                target: key.as_bytes(),
                                occurrence: caster.as_bytes(),
                                binding: b"seclib1-falcon-native-death",
                                damage,
                            },
                        )
                        .map_err(debug)?;
                }
            }
            for _ in 0..2 {
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
                    CapturedRewardPrincipal {
                        character: lease.character_id(),
                        lease_generation: lease.generation(),
                        session,
                        actor,
                    },
                    now.get() / 1000,
                    Some(now),
                );
            }
        }
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        for (character, expected) in [
            (fence.character_id, expected[0]),
            (other_fence.character_id, expected[1]),
        ] {
            let value: Option<i64> = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
            ).bind(character.as_bytes().as_slice()).bind(TRACK).fetch_optional(&h.pool).await?;
            assert_eq!(
                value, expected,
                "credit after {boss} for character {character:?}"
            );
        }
    }
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-transition/falcon-bastion/%'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(
        receipts, 4,
        "corpse replay and lower-ranked bosses grant no extra receipts"
    );
    Ok(())
}
